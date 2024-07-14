use crate::config::Config;
use crate::proxy::{vmess::encoding, ws::WebSocketStream, Proxy, RequestContext};

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};
use worker::*;

pub struct VmessStream<'a> {
    pub config: Arc<Config>,
    pub context: RequestContext,
    pub ws: WebSocketStream<'a>,
}

unsafe impl<'a> Send for VmessStream<'a> {}

impl<'a> VmessStream<'a> {
    pub fn new(config: Arc<Config>, context: RequestContext, ws: WebSocketStream<'a>) -> Self {
        Self {
            config,
            context,
            ws,
        }
    }
}

#[async_trait]
impl<'a> Proxy for VmessStream<'a> {
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

        let header =
            encoding::encode_response_header(&header.key, &header.iv, header.response_header)?;
        self.write(&header.length).await?;
        self.write(&header.payload).await?;

        tokio::io::copy_bidirectional(self, &mut upstream).await?;

        Ok(())
    }
}

impl<'a> AsyncRead for VmessStream<'a> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<tokio::io::Result<()>> {
        let mut pinned = std::pin::pin!(&mut self.ws);
        pinned.as_mut().poll_read(cx, buf)
    }
}

impl<'a> AsyncWrite for VmessStream<'a> {
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
