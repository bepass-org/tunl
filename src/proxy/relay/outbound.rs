use crate::proxy::{Proxy, RequestContext};

use std::pin::Pin;
use std::task::{Context, Poll};

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};
use worker::*;

pub struct RelayStream {
    pub stream: Socket,
    context: RequestContext,
}

impl RelayStream {
    pub fn new(context: RequestContext, stream: Socket) -> Self {
        Self { context, stream }
    }
}

#[async_trait]
impl Proxy for RelayStream {
    async fn process(&mut self) -> Result<()> {
        let header = {
            let addr = &self.context.remote_addr;
            let port = self.context.remote_port;
            let network = format!("{:?}", self.context.network).to_lowercase();

            format!("{network}@{addr}${port}\r\n").as_bytes().to_vec()
        };

        self.stream.write_all(&header).await?;
        Ok(())
    }
}

impl AsyncRead for RelayStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<tokio::io::Result<()>> {
        Pin::new(&mut self.stream).poll_read(cx, buf)
    }
}

impl AsyncWrite for RelayStream {
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
