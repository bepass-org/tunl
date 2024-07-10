pub mod hash;

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use tokio::io::{AsyncRead, AsyncReadExt};
use worker::*;

pub const KDFSALT_CONST_VMESS_HEADER_PAYLOAD_LENGTH_AEAD_KEY: &[u8] =
    b"VMess Header AEAD Key_Length";
pub const KDFSALT_CONST_VMESS_HEADER_PAYLOAD_LENGTH_AEAD_IV: &[u8] =
    b"VMess Header AEAD Nonce_Length";
pub const KDFSALT_CONST_VMESS_HEADER_PAYLOAD_AEAD_KEY: &[u8] = b"VMess Header AEAD Key";
pub const KDFSALT_CONST_VMESS_HEADER_PAYLOAD_AEAD_IV: &[u8] = b"VMess Header AEAD Nonce";
pub const KDFSALT_CONST_AEAD_RESP_HEADER_LEN_KEY: &[u8] = b"AEAD Resp Header Len Key";
pub const KDFSALT_CONST_AEAD_RESP_HEADER_LEN_IV: &[u8] = b"AEAD Resp Header Len IV";
pub const KDFSALT_CONST_AEAD_RESP_HEADER_KEY: &[u8] = b"AEAD Resp Header Key";
pub const KDFSALT_CONST_AEAD_RESP_HEADER_IV: &[u8] = b"AEAD Resp Header IV";

#[macro_export]
macro_rules! md5 {
    ( $($v:expr),+ ) => {
        {
            let mut hash = Md5::new();
            $(
                hash.update($v);
            )*
            hash.finalize()
        }
    }
}

#[macro_export]
macro_rules! sha256 {
    ( $($v:expr),+ ) => {
        {
            let mut hash = Sha256::new();
            $(
                hash.update($v);
            )*
            hash.finalize()
        }
    }
}

#[macro_export]
macro_rules! sha224 {
    ( $($v:expr),+ ) => {
        {
            let mut hash = Sha224::new();
            $(
                hash.update($v);
            )*
            hash.finalize()
        }
    }
}

#[macro_export]
macro_rules! hex {
    ($v:expr) => {
        $v.iter().map(|b| format!("{:02x}", b)).collect::<String>()
    };
}

pub fn encode_addr(addr: &str) -> Result<Vec<u8>> {
    let ip = addr
        .parse::<IpAddr>()
        .map_err(|_| Error::RustError("couldn't encode ip address".to_string()))?;
    Ok(match ip {
        IpAddr::V4(ip) => ip.octets().to_vec(),
        IpAddr::V6(ip) => ip.octets().to_vec(),
    })
}

pub async fn parse_ipv4<R: AsyncRead + std::marker::Unpin>(buf: &mut R) -> Result<String> {
    let mut addr = [0u8; 4];
    buf.read_exact(&mut addr).await?;
    Ok(Ipv4Addr::new(addr[0], addr[1], addr[2], addr[3]).to_string())
}

pub async fn parse_ipv6<R: AsyncRead + std::marker::Unpin>(buf: &mut R) -> Result<String> {
    let mut addr = [0u8; 16];
    buf.read_exact(&mut addr).await?;
    Ok(Ipv6Addr::new(
        ((addr[0] as u16) << 16) | (addr[1] as u16),
        ((addr[2] as u16) << 16) | (addr[3] as u16),
        ((addr[4] as u16) << 16) | (addr[5] as u16),
        ((addr[6] as u16) << 16) | (addr[7] as u16),
        ((addr[8] as u16) << 16) | (addr[9] as u16),
        ((addr[10] as u16) << 16) | (addr[11] as u16),
        ((addr[12] as u16) << 16) | (addr[13] as u16),
        ((addr[14] as u16) << 16) | (addr[15] as u16),
    )
    .to_string())
}

pub async fn parse_domain<R: AsyncRead + std::marker::Unpin>(buf: &mut R) -> Result<String> {
    let len = buf.read_u8().await?;
    let mut domain = vec![0u8; len as _];
    buf.read_exact(&mut domain).await?;
    Ok(String::from_utf8_lossy(&domain).to_string())
}
