use crate::config::{Config, Outbound};

use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use serde::Serialize;
use serde_json::json;

#[derive(Serialize)]
pub struct Link {
    link: String,
}

pub fn generate_link(config: &Config) -> Link {
    let link = match config.outbound {
        Outbound::Vless => generate_vless_link(config),
        Outbound::Vmess => generate_vmess_link(config),
    };

    Link { link }
}

fn generate_vless_link(config: &Config) -> String {
    format!(
        "vless://{}@{}:443?type=ws&security=tls#tunl",
        config.uuid, config.host,
    )
}

fn generate_vmess_link(config: &Config) -> String {
    let host = config.host.to_string();
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
