use crate::common::encode_addr;
use crate::proxy::{Proxy, RequestContext};

use std::pin::Pin;
use std::task::{Context, Poll};

use crate::config::Outbound;

use async_trait::async_trait;
use bytes::{BufMut, BytesMut};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};
use worker::*;

pub struct VlessStream {
    pub stream: Socket,
    pub buffer: BytesMut,
    pub outbound: Outbound,
    context: RequestContext,
    handshaked: bool,
}

impl VlessStream {
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
impl Proxy for VlessStream {
    async fn process(&mut self) -> Result<()> {
        let mut cmd = vec![0x00u8];
        cmd.extend_from_slice(self.outbound.uuid.clone().as_bytes());
        cmd.extend_from_slice(&[0x00]);
        cmd.extend_from_slice(&[self.context.network.to_byte()]);

        cmd.extend_from_slice(&self.context.port.to_be_bytes());
        let addr = encode_addr(&self.context.address)?;
        if addr.len() > 4 {
            cmd.extend_from_slice(&[0x02]);
        } else {
            cmd.extend_from_slice(&[0x01]);
        }
        cmd.extend_from_slice(&addr);

        self.stream.write_all(&cmd).await?;

        Ok(())
    }
}

impl AsyncRead for VlessStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<tokio::io::Result<()>> {
        if self.buffer.len() > 0 {
            let size = std::cmp::min(buf.remaining(), self.buffer.len());
            let data = self.buffer.split_to(size);

            if !self.handshaked {
                // ignore the two first bytes for now
                buf.put_slice(&data[2..]);
                self.handshaked = true;
            } else {
                buf.put_slice(&data);
            }

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

impl AsyncWrite for VlessStream {
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
