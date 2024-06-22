use crate::config::Config;

use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::{BufMut, BytesMut};
use futures_util::Stream;
use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use worker::*;

pin_project! {
    pub struct VlessStream<'a> {
        pub config: Config,
        pub ws: &'a WebSocket,
        pub buffer: BytesMut,
        #[pin]
        pub events: EventStream<'a>,
    }
}

impl<'a> VlessStream<'a> {
    pub fn new(config: Config, ws: &'a WebSocket, events: EventStream<'a>) -> Self {
        let buffer = BytesMut::new();

        Self {
            config,
            ws,
            buffer,
            events,
        }
    }

    pub async fn process(&mut self) -> Result<()> {
        // https://xtls.github.io/Xray-docs-next/en/development/protocols/vless.html
        // +------------------+-----------------+---------------------------------+---------------------------------+-------------+---------+--------------+---------+--------------+
        // |      1 byte      |    16 bytes     |             1 byte              |             M bytes             |   1 byte    | 2 bytes |    1 byte    | S bytes |   X bytes    |
        // +------------------+-----------------+---------------------------------+---------------------------------+-------------+---------+--------------+---------+--------------+
        // | Protocol Version | Equivalent UUID | Additional Information Length M | Additional Information ProtoBuf | Instruction | Port    | Address Type | Address | Request Data |
        // +------------------+-----------------+---------------------------------+---------------------------------+-------------+---------+--------------+---------+--------------+

        // ignore protocl version
        self.read_u8().await?;

        // UUID
        let mut uuid = [0u8; 16];
        self.read_exact(&mut uuid).await?;
        let uuid = uuid::Uuid::from_bytes(uuid);
        if self.config.uuid != uuid {
            return Err(Error::RustError("incorrect uuid".to_string()));
        }

        // additional information
        let len = self.read_u8().await?;
        let mut addon = vec![0u8; len as _];
        self.read_exact(&mut addon).await?;

        // instruction
        self.read_u8().await?;

        // addr:port
        let mut port = [0u8; 2];
        self.read_exact(&mut port).await?;
        let mut port = u16::from_be_bytes(port);
        let mut addr = crate::common::parse_addr(self).await?;

        let use_relay = self.config.is_relay_request(addr.clone());
        let mut relay_header = vec![];
        if use_relay {
            relay_header = format!("tcp@{addr}${port}\r\n").as_bytes().to_vec();
            (addr, port) = self.config.random_relay();
        }

        console_log!("connecting to upstream {}:{}", addr, port);
        let mut upstream = Socket::builder().connect(addr.clone(), port)?;

        if use_relay {
            upstream.write(&relay_header).await?;
        }

        // +-----------------------------------------------+------------------------------------+------------------------------------+---------------+
        // |                    1 Byte                     |               1 Byte               |              N Bytes               |    Y Bytes    |
        // +-----------------------------------------------+------------------------------------+------------------------------------+---------------+
        // | Protocol Version, consistent with the request | Length of additional information N | Additional information in ProtoBuf | Response data |
        // +-----------------------------------------------+------------------------------------+------------------------------------+---------------+
        self.write(&[0u8; 2]).await?; // no additional information

        tokio::io::copy_bidirectional(self, &mut upstream).await?;

        Ok(())
    }
}

impl<'a> AsyncRead for VlessStream<'a> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<tokio::io::Result<()>> {
        let mut this = self.project();

        loop {
            let size = std::cmp::min(this.buffer.len(), buf.remaining());
            if size > 0 {
                buf.put_slice(&this.buffer.split_to(size));
                return Poll::Ready(Ok(()));
            }

            match this.events.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(WebsocketEvent::Message(msg)))) => {
                    msg.bytes().iter().for_each(|x| this.buffer.put_slice(&x));
                }
                Poll::Pending => return Poll::Pending,
                _ => return Poll::Ready(Ok(())),
            }
        }
    }
}

impl<'a> AsyncWrite for VlessStream<'a> {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<tokio::io::Result<usize>> {
        return Poll::Ready(
            self.ws
                .send_with_bytes(buf)
                .map(|_| buf.len())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
        );
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<tokio::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<tokio::io::Result<()>> {
        unimplemented!()
    }
}
