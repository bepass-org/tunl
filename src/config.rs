use uuid::Uuid;

pub enum Outbound {
    Vless,
    Vmess,
}

impl Default for Outbound {
    fn default() -> Self {
        Self::Vmess
    }
}

impl From<String> for Outbound {
    fn from(s: String) -> Self {
        match s.as_str() {
            "VLESS" => Self::Vless,
            "VMESS" => Self::Vmess,
            _ => Self::default(),
        }
    }
}

pub struct Config {
    pub uuid: Uuid,
    pub host: String,
    pub outbound: Outbound,
}
