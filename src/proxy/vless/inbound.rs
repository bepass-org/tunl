use crate::config::Config;
use crate::proxy::{vless::encoding, Proxy, RequestContext};

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use async_trait::async_trait;
use bytes::{BufMut, BytesMut};
use futures_util::Stream;
use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};
use worker::*;

pin_project! {
    pub struct VlessStream<'a> {
        pub config: Arc<Config>,
        pub context: RequestContext,
        pub ws: &'a WebSocket,
        pub buffer: BytesMut,
        #[pin]
        pub events: EventStream<'a>,
    }
}

unsafe impl<'a> Send for VlessStream<'a> {}

impl<'a> VlessStream<'a> {
    pub fn new(
        config: Arc<Config>,
        context: RequestContext,
        events: EventStream<'a>,
        ws: &'a WebSocket,
    ) -> Self {
        let buffer = BytesMut::new();

        Self {
            config,
            context,
            ws,
            buffer,
            events,
        }
    }
}

#[async_trait]
impl<'a> Proxy for VlessStream<'a> {
    async fn process(&mut self) -> Result<()> {
        let uuid = self.context.inbound.uuid;
        let header = encoding::decode_request_header(&mut self, &uuid.into_bytes()).await?;

        let mut context = self.context.clone();
        {
            context.address = header.address;
            context.port = header.port;
            context.network = header.network;
        }

        let outbound = self.config.dispatch_outbound(&context);
        let mut upstream = crate::proxy::connect_outbound(context, outbound).await?;

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
