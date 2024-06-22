mod common;
mod config;
mod link;
mod proxy;

use crate::config::{Config, Inbound, Protocol};
use crate::link::generate_link;
use crate::proxy::*;

use worker::*;

lazy_static::lazy_static! {
    static ref CONFIG: &'static str = {
        include_str!(env!("CONFIG_PATH"))
    };
}

#[event(fetch)]
async fn main(req: Request, _: Env, _: Context) -> Result<Response> {
    let config = Config::new(&CONFIG);

    console_log!("config {:?}", config.inbound.len());

    match req.path().as_str() {
        "/link" => link(req, config),
        path => {
            for inbound in config.inbound.clone() {
                if inbound.path == path {
                    return tunnel(config, inbound).await;
                }
            }
            return Response::error("not found", 404);
        }
    }
}

async fn tunnel(config: Config, inbound: Inbound) -> Result<Response> {
    let WebSocketPair { server, client } = WebSocketPair::new()?;

    server.accept()?;
    wasm_bindgen_futures::spawn_local(async move {
        let events = server.events().unwrap();

        if let Err(e) = match inbound.protocol {
            Protocol::Vless => {
                VlessStream::new(config, inbound, &server, events)
                    .process()
                    .await
            }
            Protocol::Vmess => {
                VmessStream::new(config, inbound, &server, events)
                    .process()
                    .await
            }
        } {
            console_log!("[tunnel]: {}", e);
        }
    });

    Response::from_websocket(client)
}

fn link(req: Request, config: Config) -> Result<Response> {
    let host = req.url()?.host().map(|x| x.to_string()).unwrap_or_default();
    Response::from_json(&generate_link(&config, &host))
}
