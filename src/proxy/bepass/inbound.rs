use crate::config::Config;
use crate::proxy::{bepass::encoding, ws::WebSocketStream, Proxy, RequestContext};

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use async_trait::async_trait;
use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use worker::*;

pin_project! {
    pub struct BepassStream<'a> {
        pub config: Arc<Config>,
        pub context: RequestContext,
        pub ws: WebSocketStream<'a>,
    }
}

unsafe impl<'a> Send for BepassStream<'a> {}

impl<'a> BepassStream<'a> {
    pub fn new(config: Arc<Config>, context: RequestContext, ws: WebSocketStream<'a>) -> Self {
        Self {
            config,
            context,
            ws,
        }
    }
}

#[async_trait]
impl<'a> Proxy for BepassStream<'a> {
    async fn process(&mut self) -> Result<()> {
        let request = self.context.request.as_ref().ok_or(Error::RustError(
            "failed to retrive request context".to_string(),
        ))?;
        let header = encoding::decode_request_header(request)?;

        let mut context = self.context.clone();
        {
            context.address = header.address.clone();
            context.port = header.port;
            context.network = header.network;
        }

        let outbound = self.config.dispatch_outbound(&context);
        let mut upstream = crate::proxy::connect_outbound(context, outbound).await?;

        tokio::io::copy_bidirectional(self, &mut upstream).await?;

        Ok(())
    }
}

impl<'a> AsyncRead for BepassStream<'a> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<tokio::io::Result<()>> {
        let mut pinned = std::pin::pin!(&mut self.ws);
        pinned.as_mut().poll_read(cx, buf)
    }
}

impl<'a> AsyncWrite for BepassStream<'a> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<tokio::io::Result<usize>> {
        let mut pinned = std::pin::pin!(&mut self.ws);
        pinned.as_mut().poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<tokio::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<tokio::io::Result<()>> {
        unimplemented!()
    }
}
