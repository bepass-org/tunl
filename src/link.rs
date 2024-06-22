use crate::config::{Config, Inbound, Protocol};

use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use serde::Serialize;
use serde_json::json;

#[derive(Serialize)]
pub struct Link {
    links: Vec<String>,
}

pub fn generate_link(config: &Config, host: &str) -> Link {
    let links = config
        .inbound
        .clone()
        .into_iter()
        .map(|inbound| match inbound.protocol {
            Protocol::Vless => generate_vless_link(&inbound, host),
            Protocol::Vmess => generate_vmess_link(&inbound, host),
        })
        .collect();

    Link { links }
}

fn generate_vless_link(config: &Inbound, host: &str) -> String {
    format!(
        "vless://{}@{}:443?type=ws&security=tls#tunl",
        config.uuid, host,
    )
}

fn generate_vmess_link(config: &Inbound, host: &str) -> String {
    let uuid = config.uuid.to_string();
    let config = json!({
        "ps": "tunl",
        "v": "2",
        "add": host,
        "port": "443",
        "id": uuid,
        "aid": "0",
        "scy": "zero",
        "net": "ws",
        "type": "none",
        "tls": "tls",
        "sni": "",
        "alpn": ""}
    );
    format!("vmess://{}", URL_SAFE.encode(config.to_string()))
}
