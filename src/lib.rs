mod common;
mod config;
mod proxy;

use crate::config::Config;
use crate::proxy::*;

use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;
use worker::*;

#[event(fetch)]
async fn main(req: Request, env: Env, _: Context) -> Result<Response> {
    let uuid = env
        .var("UUID")
        .map(|x| Uuid::parse_str(&x.to_string()).unwrap_or_default())?;
    let host = req.url()?.host().map(|x| x.to_string()).unwrap_or_default();
    let config = Config { uuid, host };

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
        let events = server.events().unwrap();
        if let Err(e) = VmessStream::new(cx.data, &server, events).process().await {
            console_log!("[tunnel]: {}", e);
        }
    });

    Response::from_websocket(client)
}

fn link(_: Request, cx: RouteContext<Config>) -> Result<Response> {
    #[derive(Serialize)]
    struct Link {
        description: String,
        link: String,
    }

    let link = {
        let host = cx.data.host.to_string();
        let uuid = cx.data.uuid.to_string();
        let config = json!({
            "ps": "tunl",
            "v": "2",
            "add": "162.159.16.149",
            "port": "80",
            "id": uuid,
            "aid": "0",
            "scy": "zero",
            "net": "ws",
            "type": "none",
            "host": host,
            "path": "",
            "tls": "",
            "sni": "",
            "alpn": ""}
        );
        format!("vmess://{}", URL_SAFE.encode(config.to_string()))
    };

    Response::from_json(&Link {
        link,
        description:
            "visit https://scanner.github1.cloud/ and replace the IP address in the configuration with a clean one".to_string()
    })
}
