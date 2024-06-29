use crate::proxy::Network;

use worker::*;

#[derive(Default)]
pub struct Header {
    pub network: Network,
    pub address: String,
    pub port: u16,
}

pub fn decode_request_header(request: &Request) -> Result<Header> {
    let mut header = Header::default();

    let url = request.url()?;
    let mut pairs = url.query_pairs();

    while let Some((k, v)) = pairs.next() {
        match k.as_ref() {
            "host" => header.address = v.to_string(),
            "port" => {
                header.port = v
                    .parse::<u16>()
                    .map_err(|_| Error::RustError("invalid port number".to_string()))?;
            }
            "net" => {
                header.network = Network::from_str(&v)?;
            }
            _ => {
                // ignore other query params for now
            }
        }
    }

    Ok(header)
}
