use crate::proxy::Proxy;
use std::pin::Pin;
use std::task::{Context, Poll};

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use worker::*;

use std::sync::{Arc, Mutex};
use std::task::Waker;

use super::doh;

struct MockUDPStreamInner {
    response: Option<Vec<u8>>,
    waker: Option<Waker>,
}

impl MockUDPStreamInner {
    fn set_response(&mut self, response: Vec<u8>) {
        self.response = Some(response);
        if let Some(waker) = self.waker.take() {
            waker.wake()
        }
    }
}

pub struct MockUDPStream {
    inner: Arc<Mutex<MockUDPStreamInner>>,
}

impl MockUDPStream {
    pub fn new() -> Self {
        MockUDPStream {
            inner: Arc::new(Mutex::new(MockUDPStreamInner {
                response: None,
                waker: None,
            })),
        }
    }
}

#[async_trait]
impl Proxy for MockUDPStream {
    async fn process(&mut self) -> Result<()> {
        Ok(())
    }
}

impl AsyncRead for MockUDPStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<tokio::io::Result<()>> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(response) = inner.response.take() {
            let len = std::cmp::min(buf.remaining(), response.len());
            buf.put_slice(&response[..len]);
            Poll::Ready(Ok(()))
        } else {
            inner.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl AsyncWrite for MockUDPStream {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<tokio::io::Result<usize>> {
        let inner_ref = self.inner.clone();

        let b = buf.to_vec();
        wasm_bindgen_futures::spawn_local(async move {
            match doh::doh(b.as_slice()).await {
                Ok(x) => inner_ref.lock().unwrap().set_response(x),
                Err(e) => console_error!("doh failed: {e:?}"),
            };
        });

        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<tokio::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<tokio::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
