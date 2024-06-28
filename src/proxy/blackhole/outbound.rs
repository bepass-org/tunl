use crate::proxy::Proxy;

use std::pin::Pin;
use std::task::{Context, Poll};

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use worker::*;

pub struct BlackholeStream;

#[async_trait]
impl Proxy for BlackholeStream {
    async fn process(&mut self) -> Result<()> {
        Ok(())
    }
}

impl AsyncRead for BlackholeStream {
    fn poll_read(
        self: Pin<&mut Self>,
        _: &mut Context,
        _: &mut ReadBuf<'_>,
    ) -> Poll<tokio::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for BlackholeStream {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context,
        buf: &[u8],
    ) -> Poll<tokio::io::Result<usize>> {
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context) -> Poll<tokio::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context) -> Poll<tokio::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
