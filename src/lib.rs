mod common;
mod config;
mod link;
mod proxy;

use crate::config::{Config, Outbound};
use crate::link::generate_link;
use crate::proxy::*;

use wirefilter::Type;
use worker::*;

lazy_static::lazy_static! {
    static ref CONFIG: &'static str = {
        include_str!(env!("CONFIG_PATH"))
    };
}

#[durable_object]
struct Tunl {
    config: Config,
}

impl Tunl {
    async fn tunnel(&self) -> Result<Response> {
        let config = self.config.clone();
        let WebSocketPair { server, client } = WebSocketPair::new()?;

        server.accept()?;
        wasm_bindgen_futures::spawn_local(async move {
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

    fn link(&self, req: Request) -> Result<Response> {
        let host = req.url()?.host().map(|x| x.to_string()).unwrap_or_default();
        Response::from_json(&generate_link(&self.config, &host))
    }
}

// TODO: make sure about durable objects persistence.
// it's heavy to build the config per every request, so we use durable objects
// to make the config once and use it in every request.
#[durable_object]
impl DurableObject for Tunl {
    fn new(state: State, _: Env) -> Self {
        let config = Config::new(&CONFIG);
        Self { config }
    }

    async fn fetch(&mut self, req: Request) -> Result<Response> {
        match req.path().as_str() {
            "/link" => self.link(req),
            _ => self.tunnel().await,
        }
    }
}

#[event(fetch)]
async fn main(req: Request, env: Env, _: Context) -> Result<Response> {
    let namespace = env.durable_object("TUNL")?;
    let stub = namespace.id_from_name("TUNL")?.get_stub()?;
    stub.fetch_with_request(req).await
}
