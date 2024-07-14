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
        .filter_map(|inbound| match inbound.protocol {
            Protocol::Vless => Some(generate_vless_link(&inbound, host)),
            Protocol::Vmess => Some(generate_vmess_link(&inbound, host)),
            Protocol::Trojan => Some(generate_trojan_link(&inbound, host)),
            _ => None,
        })
        .collect();

    Link { links }
}

fn generate_vless_link(config: &Inbound, host: &str) -> String {
    format!(
        "vless://{}@{}:443?type=ws&security=tls&path={}#tunl",
        config.uuid, host, config.path
    )
}

fn generate_vmess_link(config: &Inbound, host: &str) -> String {
    let uuid = config.uuid.to_string();
    let path = &config.path;
    let config = json!({
        "ps": "tunl",
        "v": "2",
        "add": host,
        "port": "443",
        "id": uuid,
        "path": path,
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

fn generate_trojan_link(config: &Inbound, host: &str) -> String {
    format!(
        "trojan://{}@{}:443?security=tls&type=ws&path={}#tunl",
        config.password, host, config.path
    )
}
