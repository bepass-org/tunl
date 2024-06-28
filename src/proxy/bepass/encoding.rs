use crate::proxy::Network;

use worker::*;

pub struct Header {
    pub network: Network,
    pub address: String,
    pub port: u16,
}

pub fn decode_request_header(request: &Request) -> Result<Header> {
    let queries = request.query::<std::collections::HashMap<String, String>>()?;

    let address = queries
        .get("host")
        .ok_or(Error::RustError("invalid request address".to_string()))?;

    let port = {
        let p = queries
            .get("port")
            .ok_or(Error::RustError("invalid request port".to_string()))?;

        p.parse::<u16>()
            .map_err(|_| Error::RustError("invalid port number".to_string()))?
    };

    let network = {
        let net = queries
            .get("net")
            .ok_or(Error::RustError("invalid request network".to_string()))?;
        Network::from_str(&net)?
    };

    Ok(Header {
        address: address.to_string(),
        network,
        port,
    })
}
