mod common;
mod config;
mod link;
mod proxy;

use crate::config::{Config, Outbound};
use crate::link::generate_link;
use crate::proxy::*;

use worker::*;

lazy_static::lazy_static! {
    static ref CONFIG: &'static str = {
        include_str!(env!("CONFIG_PATH"))
    };
}

#[event(fetch)]
async fn main(req: Request, env: Env, _: Context) -> Result<Response> {
    let config = Config::new(&CONFIG);

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

fn link(req: Request, cx: RouteContext<Config>) -> Result<Response> {
    let config = cx.data;
    let host = req.url()?.host().map(|x| x.to_string()).unwrap_or_default();
    Response::from_json(&generate_link(&config, &host))
}
