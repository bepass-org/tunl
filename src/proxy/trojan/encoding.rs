use crate::proxy::Network;

use sha2::{Digest, Sha224};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use worker::*;

pub struct Header {
    pub network: Network,
    pub address: String,
    pub port: u16,
}

pub async fn decode_request_header<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    password: &str,
) -> Result<Header> {
    // TODO: using BufReader instead of reading directly from the stream

    // +-----------------------+---------+----------------+---------+----------+
    // | hex(SHA224(password)) |  CRLF   | Trojan Request |  CRLF   | Payload  |
    // +-----------------------+---------+----------------+---------+----------+
    // |          56           | X'0D0A' |    Variable    | X'0D0A' | Variable |
    // +-----------------------+---------+----------------+---------+----------+
    let mut crlf = [0u8; 2];

    let mut header_pass = [0u8; 56];
    stream.read_exact(&mut header_pass).await?;
    {
        let header_pass = String::from_utf8_lossy(&header_pass);
        let password = {
            let p = &crate::sha224!(&password)[..];
            crate::hex!(p)
        };

        if password != header_pass {
            return Err(Error::RustError("invalid password".to_string()));
        }
    }

    stream.read_exact(&mut crlf).await?;

    // +-----+------+----------+----------+
    // | CMD | ATYP | DST.ADDR | DST.PORT |
    // +-----+------+----------+----------+
    // |  1  |  1   | Variable |    2     |
    // +-----+------+----------+----------+
    let network = match stream.read_u8().await? {
        0x01 => Network::Tcp,
        0x03 => Network::Udp,
        _ => return Err(Error::RustError("invalid network type".to_string())),
    };

    let address = match stream.read_u8().await? {
        0x01 => crate::common::parse_ipv4(stream).await?,
        0x03 => crate::common::parse_domain(stream).await?,
        0x04 => crate::common::parse_ipv6(stream).await?,
        _ => return Err(Error::RustError("invalid address".to_string())),
    };
    let port = {
        let mut p = [0u8; 2];
        stream.read_exact(&mut p).await?;
        u16::from_be_bytes(p)
    };

    // UDP
    // +--------+
    // | Length |
    // +--------+
    // |   2    |
    // +--------+
    match network {
        Network::Udp => {
            stream.read_exact(&mut [0u8; 2]).await?;
        }
        _ => {}
    }
    stream.read_exact(&mut crlf).await?;

    Ok(Header {
        network,
        address,
        port,
    })
}
