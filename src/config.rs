use std::net::IpAddr;

use cidr::IpCidr;
use serde::Deserialize;
use uuid::{self, Uuid};

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Vless,
    Vmess,
}

impl Default for Protocol {
    fn default() -> Self {
        Self::Vmess
    }
}

#[derive(Default, Clone, Deserialize)]
pub struct Inbound {
    pub protocol: Protocol,
    pub uuid: Uuid,
    pub path: String,
}

#[derive(Default, Clone, Deserialize)]
pub struct RelayConfig {
    pub r#match: Vec<IpCidr>,
    pub addresses: Vec<String>,
    pub port: u16,
}

#[derive(Default, Clone, Deserialize)]
pub struct Config {
    pub inbound: Vec<Inbound>,
    pub relay: RelayConfig,
}

impl Config {
    pub fn new(buf: &str) -> Self {
        // TODO: notify the user in case of having an invalid config format
        toml::from_str(buf).unwrap_or_default()
    }

    pub fn is_relay_request(&self, ip: String) -> bool {
        if let Ok(ip) = ip.parse::<IpAddr>() {
            return self.relay.r#match.iter().any(|cidr| cidr.contains(&ip));
        }
        false
    }

    pub fn random_relay(&self) -> (String, u16) {
        let i = fastrand::usize(..self.relay.addresses.len());
        (self.relay.addresses[i].clone(), self.relay.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config() {
        let buf = r#"
            [[inbound]]
            protocol = "vless"
            uuid = "0fbf4f81-2598-4b6a-a623-0ead4cb9efa8"
            path = "/vless"

            [[inbound]]
            protocol = "vmess"
            uuid = "0fbf4f81-2598-4b6a-a623-0ead4cb9efa8"
            path = "/vmess"

            # forward matched connections to a bepass-relay server
            [relay]
            match = ["173.245.48.0/20",
                     "103.21.244.0/22",
                     "103.22.200.0/22",
                     "103.31.4.0/22",
                     "141.101.64.0/18",
                     "108.162.192.0/18",
                     "190.93.240.0/20",
                     "188.114.96.0/20",
                     "197.234.240.0/22",
                     "198.41.128.0/17",
                     "162.158.0.0/15",
                     "104.16.0.0/13",
                     "104.24.0.0/14",
                     "172.64.0.0/13",
                     "131.0.72.0/22"]
            addresses = ["1.1.1.1"]
            port = 6666
        "#;
        let config = Config::new(buf);

        assert_eq!(config.inbound[0].protocol, Protocol::Vless);
        assert_eq!(
            config.inbound[1].uuid,
            uuid::uuid!("0fbf4f81-2598-4b6a-a623-0ead4cb9efa8")
        );
        assert_eq!(config.relay.addresses, vec!["1.1.1.1"]);
    }
}
