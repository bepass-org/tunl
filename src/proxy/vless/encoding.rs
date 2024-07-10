use crate::proxy::Network;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use worker::*;

pub struct Header {
    pub network: Network,
    pub address: String,
    pub port: u16,
}

pub async fn decode_request_header<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    uuid: &[u8; 16],
) -> Result<Header> {
    // https://xtls.github.io/Xray-docs-next/en/development/protocols/vless.html
    // +------------------+-----------------+---------------------------------+---------------------------------+-------------+---------+--------------+---------+
    // |      1 byte      |    16 bytes     |             1 byte              |             M bytes             |   1 byte    | 2 bytes |    1 byte    | S bytes |
    // +------------------+-----------------+---------------------------------+---------------------------------+-------------+---------+--------------+---------+
    // | Protocol Version | Equivalent UUID | Additional Information Length M | Additional Information ProtoBuf | Instruction | Port    | Address Type | Address |
    // +------------------+-----------------+---------------------------------+---------------------------------+-------------+---------+--------------+---------+

    // version
    if stream.read_u8().await? != 0 {
        return Err(Error::RustError("invalid request version".to_string()));
    }

    let mut id = [0u8; 16];
    stream.read_exact(&mut id).await?;
    if &id != uuid {
        return Err(Error::RustError("incorrect request user id".to_string()));
    }

    // Addons (ignore for now)
    let len = stream.read_u8().await?;
    let mut addon = vec![0u8; len as _];
    stream.read_exact(&mut addon).await?;

    let network = Network::from_byte(stream.read_u8().await?)?;

    let port = {
        let mut p = [0u8; 2];
        stream.read_exact(&mut p).await?;
        u16::from_be_bytes(p)
    };
    let address = match stream.read_u8().await? {
        0x01 => crate::common::parse_ipv4(stream).await?,
        0x02 => crate::common::parse_domain(stream).await?,
        0x03 => crate::common::parse_ipv6(stream).await?,
        _ => return Err(Error::RustError("invalid address".to_string())),
    };

    Ok(Header {
        network,
        address,
        port,
    })
}
