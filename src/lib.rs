mod common;
mod config;
mod link;
mod proxy;

use crate::config::{Config, Inbound};
use crate::link::generate_link;

use worker::*;

lazy_static::lazy_static! {
    static ref CONFIG: &'static str = {
        include_str!(env!("CONFIG_PATH"))
    };
}

#[event(fetch)]
async fn main(req: Request, _: Env, _: Context) -> Result<Response> {
    let config = Config::new(&CONFIG);

    match req.path().as_str() {
        "/link" => link(req, config),
        path => match config.dispatch_inbound(path) {
            Some(inbound) => tunnel(config, inbound).await,
            None => Response::empty(),
        },
    }
}

async fn tunnel(config: Config, inbound: Inbound) -> Result<Response> {
    let WebSocketPair { server, client } = WebSocketPair::new()?;

    server.accept()?;
    wasm_bindgen_futures::spawn_local(async move {
        let events = server.events().unwrap();

        if let Err(e) = proxy::process(config, inbound, &server, events).await {
            console_log!("[tunnel]: {}", e);
        }
    });

    Response::from_websocket(client)
}

fn link(req: Request, config: Config) -> Result<Response> {
    let host = req.url()?.host().map(|x| x.to_string()).unwrap_or_default();
    Response::from_json(&generate_link(&config, &host))
}
