use crate::proxy::{self, Proxy, RequestContext};

use std::net::IpAddr;
use std::pin::Pin;
use std::task::{Context, Poll};

use async_trait::async_trait;
use bincode::{Decode, Encode};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};
use worker::*;

#[derive(Decode, Encode)]
pub enum RelayVersion {
    V1,
    V2,
}

#[derive(Decode, Encode)]
enum Network {
    Tcp,
    Udp,
}

impl Network {
    fn from(net: &proxy::Network) -> Self {
        match net {
            proxy::Network::Tcp => Self::Tcp,
            proxy::Network::Udp => Self::Udp,
        }
    }
}

#[derive(Decode, Encode)]
struct Header {
    pub ver: RelayVersion,
    pub net: Network,
    pub addr: IpAddr,
    pub port: u16,
}

pub struct RelayStream {
    pub stream: Socket,
    context: RequestContext,
    version: RelayVersion,
}

impl RelayStream {
    pub fn new(context: RequestContext, stream: Socket, version: RelayVersion) -> Self {
        Self {
            context,
            stream,
            version,
        }
    }

    async fn process_v1(&mut self) -> Result<()> {
        let header = {
            let addr = &self.context.address;
            let port = self.context.port;
            let network = format!("{:?}", self.context.network).to_lowercase();

            format!("{network}@{addr}${port}\r\n").as_bytes().to_vec()
        };

        self.stream.write_all(&header).await?;
        Ok(())
    }

    async fn process_v2(&mut self) -> Result<()> {
        // +---------+---------+---------+---------+---------+
        // | 2 Bytes | 1 Byte  | 1 Byte  | n Bytes | 2 Bytes |
        // +---------+---------+---------+---------+---------+
        // | length  | version | network | address | port    |
        // +---------+---------+---------+---------+---------+

        let address = self
            .context
            .address
            .parse::<IpAddr>()
            .map_err(|_| Error::RustError("invalid ip address".to_string()))?;

        let header = Header {
            ver: RelayVersion::V2,
            net: Network::from(&self.context.network),
            addr: address,
            port: self.context.port,
        };

        let mut slice = [0u8; 128];
        let len = bincode::encode_into_slice(header, &mut slice, bincode::config::standard())
            .map_err(|e| Error::RustError(format!("bincode {e}")))?;

        let len_bytes = (len as u16).to_be_bytes();
        self.stream
            .write_all(&[&len_bytes, &slice[..len]].concat())
            .await?;

        Ok(())
    }
}

#[async_trait]
impl Proxy for RelayStream {
    async fn process(&mut self) -> Result<()> {
        match &self.version {
            RelayVersion::V1 => self.process_v1().await,
            RelayVersion::V2 => self.process_v2().await,
        }
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
