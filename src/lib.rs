mod common;
mod config;
mod link;
mod proxy;

use std::sync::Arc;

use crate::config::Config;
use crate::link::generate_link;
use crate::proxy::RequestContext;

use worker::*;

lazy_static::lazy_static! {
    static ref CONFIG: Arc<Config> = {
        let c = include_str!(env!("CONFIG_PATH"));
        Arc::new(Config::new(c))
    };
}

#[event(fetch)]
async fn main(req: Request, _: Env, _: Context) -> Result<Response> {
    match req.path().as_str() {
        "/link" => link(req, CONFIG.clone()),
        path => match CONFIG.dispatch_inbound(path) {
            Some(inbound) => {
                let context = RequestContext {
                    inbound,
                    request: Some(req),
                    ..Default::default()
                };
                tunnel(CONFIG.clone(), context).await
            }
            None => Response::empty(),
        },
    }
}

async fn tunnel(config: Arc<Config>, context: RequestContext) -> Result<Response> {
    let WebSocketPair { server, client } = WebSocketPair::new()?;

    server.accept()?;
    wasm_bindgen_futures::spawn_local(async move {
        let events = server.events().unwrap();

        if let Err(e) = proxy::process(config, context, &server, events).await {
            console_log!("[tunnel]: {}", e);
        }
    });

    Response::from_websocket(client)
}

fn link(req: Request, config: Arc<Config>) -> Result<Response> {
    let host = req.url()?.host().map(|x| x.to_string()).unwrap_or_default();
    Response::from_json(&generate_link(&config, &host))
}
