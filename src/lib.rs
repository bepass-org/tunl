mod common;
mod config;
mod link;
mod proxy;

use crate::config::{Config, Outbound};
use crate::link::generate_link;
use crate::proxy::*;

use uuid::Uuid;
use worker::*;

#[event(fetch)]
async fn main(req: Request, env: Env, _: Context) -> Result<Response> {
    let uuid = env
        .var("UUID")
        .map(|x| Uuid::parse_str(&x.to_string()).unwrap_or_default())?;
    let outbound = env
        .var("OUTBOUND")
        .map(|x| Outbound::from(x.to_string()))
        .unwrap_or_default();
    let host = req.url()?.host().map(|x| x.to_string()).unwrap_or_default();
    let config = Config {
        uuid,
        host,
        outbound,
    };

    Router::with_data(config)
        .on_async("/", tunnel)
        .on("/link", link)
        .run(req, env)
        .await
}

async fn tunnel(_: Request, cx: RouteContext<Config>) -> Result<Response> {
    let WebSocketPair { server, client } = WebSocketPair::new()?;

    server.accept()?;
    wasm_bindgen_futures::spawn_local(async move {
        let config = cx.data;
        let events = server.events().unwrap();

        if let Err(e) = match config.outbound {
            Outbound::Vless => VlessStream::new(config, &server, events).process().await,
            Outbound::Vmess => VmessStream::new(config, &server, events).process().await,
        } {
            console_log!("[tunnel]: {}", e);
        }
    });

    Response::from_websocket(client)
}

fn link(_: Request, cx: RouteContext<Config>) -> Result<Response> {
    Response::from_json(&generate_link(&cx.data))
}
