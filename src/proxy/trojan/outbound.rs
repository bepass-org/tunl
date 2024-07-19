use crate::common::encode_addr;
use crate::proxy::{Proxy, RequestContext};

use std::pin::Pin;
use std::task::{Context, Poll};

use crate::config::Outbound;

use async_trait::async_trait;
use bytes::{BufMut, BytesMut};
use sha2::{Digest, Sha224};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};
use worker::*;

pub struct TrojanStream {
    pub stream: Socket,
    pub buffer: BytesMut,
    pub outbound: Outbound,
    context: RequestContext,
    handshaked: bool,
}

impl TrojanStream {
    pub fn new(context: RequestContext, outbound: Outbound, stream: Socket) -> Self {
        let buffer = BytesMut::new();

        Self {
            context,
            outbound,
            stream,
            buffer,
            handshaked: false,
        }
    }
}

#[async_trait]
impl Proxy for TrojanStream {
    async fn process(&mut self) -> Result<()> {
        let crlf = [0xd, 0xa];

        let mut cmd: Vec<u8> = vec![];

        let password = {
            let p = &crate::sha224!(&self.outbound.password)[..];
            crate::hex!(p)
        };
        cmd.extend_from_slice(password.as_bytes());

        cmd.extend_from_slice(&crlf);
        cmd.extend_from_slice(&[
            0x1, // TODO: udp
            0x1, // TODO: ipv6 & domain
        ]);

        cmd.extend_from_slice(&encode_addr(&self.context.address)?);
        cmd.extend_from_slice(&self.context.port.to_be_bytes());

        cmd.extend_from_slice(&crlf);

        self.stream.write_all(&cmd).await?;

        Ok(())
    }
}

impl AsyncRead for TrojanStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<tokio::io::Result<()>> {
        if self.buffer.len() > 0 {
            let size = std::cmp::min(buf.remaining(), self.buffer.len());
            let data = self.buffer.split_to(size);
            buf.put_slice(&data);
            return Poll::Ready(Ok(()));
        }

        match Pin::new(&mut self.stream).poll_read(cx, buf) {
            Poll::Ready(Ok(())) => {
                self.buffer.put_slice(buf.filled());
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl AsyncWrite for TrojanStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<tokio::io::Result<usize>> {
        Pin::new(&mut self.stream).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context) -> Poll<tokio::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context) -> Poll<tokio::io::Result<()>> {
        unimplemented!()
    }
}
