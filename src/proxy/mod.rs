pub mod relay;
pub mod vless;
pub mod vmess;

use crate::config::*;

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};
use worker::*;

#[async_trait]
pub trait Proxy: AsyncRead + AsyncWrite + Unpin + Send {
    async fn process(&mut self) -> Result<()>;
}

#[async_trait]
impl Proxy for Socket {
    async fn process(&mut self) -> Result<()> {
        Ok(())
    }
}

#[derive(Default, Debug)]
pub enum Network {
    #[default]
    Tcp,
    Udp,
}

impl Network {
    fn from_byte(b: u8) -> Result<Self> {
        match b {
            0x01 => Ok(Self::Tcp),
            0x02 => Ok(Self::Udp),
            _ => Err(Error::RustError("invalid network type".to_string())),
        }
    }

    fn to_byte(&self) -> u8 {
        match self {
            Self::Tcp => 0x01,
            Self::Udp => 0x02,
        }
    }
}

#[derive(Default)]
pub struct RequestContext {
    address: String,
    port: u16,
    network: Network,
}

async fn connect_outbound(ctx: RequestContext, outbound: Outbound) -> Result<Box<dyn Proxy>> {
    let (addr, port) = match outbound.protocol {
        Protocol::Freedom => (&ctx.address, ctx.port),
        _ => (
            &outbound.addresses[fastrand::usize(..outbound.addresses.len())],
            outbound.port,
        ),
    };

    console_log!(
        "[{:?}] connecting to upstream {addr}:{port}",
        outbound.protocol
    );

    let socket = Socket::builder().connect(addr, port)?;

    let mut stream: Box<dyn Proxy> = match outbound.protocol {
        Protocol::Vless => Box::new(vless::outbound::VlessStream::new(ctx, outbound, socket)),
        Protocol::Relay => Box::new(relay::outbound::RelayStream::new(ctx, socket)),
        _ => Box::new(socket),
    };

    stream.process().await?;
    Ok(stream)
}

pub async fn process(
    config: Config,
    inbound: Inbound,
    ws: &WebSocket,
    events: EventStream<'_>,
) -> Result<()> {
    match inbound.protocol {
        Protocol::Vmess => {
            vmess::inbound::VmessStream::new(config, inbound, events, ws)
                .process()
                .await
        }
        Protocol::Vless => {
            vless::inbound::VlessStream::new(config, inbound, events, ws)
                .process()
                .await
        }
        _ => return Err(Error::RustError("invalid inbound protocol".to_string())),
    }
}
