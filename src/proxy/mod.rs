pub mod vless;
pub mod vmess;

use crate::config::*;

use tokio::io::{AsyncRead, AsyncWrite};
use worker::*;

#[derive(Default)]
pub struct RequestContext {
    remote_addr: String,
    remote_port: u16,
}

pub trait AsyncRW: AsyncRead + AsyncWrite + Unpin {}
impl<S: AsyncRead + AsyncWrite + Unpin> AsyncRW for S {}

async fn connect_outbound(ctx: RequestContext, outbound: Outbound) -> Result<Box<dyn AsyncRW>> {
    console_log!(
        "connecting to upstream {}:{}",
        ctx.remote_addr,
        ctx.remote_port
    );

    match outbound.protocol {
        Protocol::Vless => {
            let addr = &outbound.addresses[fastrand::usize(..outbound.addresses.len())];
            let socket = Socket::builder().connect(addr, outbound.port)?;
            let mut stream = vless::outbound::VlessStream::new(ctx, outbound, socket);
            stream.process().await?;

            Ok(Box::new(stream))
        }
        _ => {
            // FIXME
            let addr = if ctx.remote_addr.contains(':') {
                format!("[{}]", ctx.remote_addr)
            } else {
                ctx.remote_addr
            };
            let socket = Socket::builder().connect(addr, ctx.remote_port)?;
            Ok(Box::new(socket))
        }
    }
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
