use crate::common::{
    hash, KDFSALT_CONST_AEAD_RESP_HEADER_IV, KDFSALT_CONST_AEAD_RESP_HEADER_KEY,
    KDFSALT_CONST_AEAD_RESP_HEADER_LEN_IV, KDFSALT_CONST_AEAD_RESP_HEADER_LEN_KEY,
    KDFSALT_CONST_VMESS_HEADER_PAYLOAD_AEAD_IV, KDFSALT_CONST_VMESS_HEADER_PAYLOAD_AEAD_KEY,
    KDFSALT_CONST_VMESS_HEADER_PAYLOAD_LENGTH_AEAD_IV,
    KDFSALT_CONST_VMESS_HEADER_PAYLOAD_LENGTH_AEAD_KEY,
};
use crate::proxy::Network;

use std::io::Cursor;

use aes::cipher::KeyInit;
use aes_gcm::{
    aead::{Aead, Payload},
    Aes128Gcm,
};
use md5::{Digest, Md5};
use sha2::Sha256;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use worker::*;

pub struct RequestHeader {
    pub network: Network,
    pub address: String,
    pub port: u16,
    pub key: [u8; 16],
    pub iv: [u8; 16],
    pub response_header: u8,
}

pub struct ResponseHeader {
    pub length: Vec<u8>,
    pub payload: Vec<u8>,
}

pub async fn decode_request_header<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    uuid: &[u8; 16],
) -> Result<RequestHeader> {
    let mut stream = Cursor::new(aead_decrypt(stream, uuid).await?);

    // https://xtls.github.io/en/development/protocols/vmess.html#command-section
    //
    // +---------+--------------------+---------------------+-------------------------------+---------+----------+-------------------+----------+---------+---------+--------------+---------+--------------+----------+
    // | 1 Byte  |      16 Bytes      |      16 Bytes       |            1 Byte             | 1 Byte  |  4 bits  |      4 bits       |  1 Byte  | 1 Byte  | 2 Bytes |    1 Byte    | N Bytes |   P Bytes    | 4 Bytes  |
    // +---------+--------------------+---------------------+-------------------------------+---------+----------+-------------------+----------+---------+---------+--------------+---------+--------------+----------+
    // | Version | Data Encryption IV | Data Encryption Key | Response Authentication Value | Options | Reserved | Encryption Method | Reserved | Command | Port    | Address Type | Address | Random Value | Checksum |
    // +---------+--------------------+---------------------+-------------------------------+---------+----------+-------------------+----------+---------+---------+--------------+---------+--------------+----------+

    // version
    if stream.read_u8().await? != 1 {
        return Err(Error::RustError("invalid request version".to_string()));
    }

    let mut iv = [0u8; 16];
    let mut key = [0u8; 16];
    stream.read_exact(&mut iv).await?;
    stream.read_exact(&mut key).await?;

    // ignore options for now
    let mut options = [0u8; 5];
    stream.read_exact(&mut options).await?;

    let network = Network::from_byte(options[4])?;

    let port = {
        let mut p = [0u8; 2];
        stream.read_exact(&mut p).await?;
        u16::from_be_bytes(p)
    };
    let address = match stream.read_u8().await? {
        0x01 => crate::common::parse_ipv4(&mut stream).await?,
        0x02 => crate::common::parse_domain(&mut stream).await?,
        0x03 => crate::common::parse_ipv6(&mut stream).await?,
        _ => return Err(Error::RustError("invalid address".to_string())),
    };

    Ok(RequestHeader {
        network,
        address,
        port,
        key,
        iv,
        response_header: options[0],
    })
}

pub fn encode_response_header(
    key: &[u8; 16],
    iv: &[u8; 16],
    response_header: u8,
) -> Result<ResponseHeader> {
    let key = &crate::sha256!(&key)[..16];
    let iv = &crate::sha256!(&iv)[..16];

    // https://github.com/v2ray/v2ray-core/blob/master/proxy/vmess/encoding/client.go#L196
    let length_key = &hash::kdf(&key, &[KDFSALT_CONST_AEAD_RESP_HEADER_LEN_KEY])[..16];
    let length_iv = &hash::kdf(&iv, &[KDFSALT_CONST_AEAD_RESP_HEADER_LEN_IV])[..12];
    let length = Aes128Gcm::new(length_key.into())
        // 4 bytes header: https://github.com/v2ray/v2ray-core/blob/master/proxy/vmess/encoding/client.go#L238
        .encrypt(length_iv.into(), &4u16.to_be_bytes()[..])
        .map_err(|e| Error::RustError(e.to_string()))?;

    let payload_key = &hash::kdf(&key, &[KDFSALT_CONST_AEAD_RESP_HEADER_KEY])[..16];
    let payload_iv = &hash::kdf(&iv, &[KDFSALT_CONST_AEAD_RESP_HEADER_IV])[..12];
    let payload = {
        let header = [
            response_header, // https://github.com/v2ray/v2ray-core/blob/master/proxy/vmess/encoding/client.go#L242
            0x00,
            0x00,
            0x00,
        ];
        Aes128Gcm::new(payload_key.into())
            .encrypt(payload_iv.into(), &header[..])
            .map_err(|e| Error::RustError(e.to_string()))?
    };

    Ok(ResponseHeader { length, payload })
}

async fn aead_decrypt<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    uuid: &[u8; 16],
) -> Result<Vec<u8>> {
    let key = crate::md5!(uuid, b"c48619fe-8f02-49e0-b9e9-edf763e17e21");

    // +-------------------+-------------------+-------------------+
    // |     Auth ID       |   Header Length   |       Nonce       |
    // +-------------------+-------------------+-------------------+
    // |     16 Bytes      |     18 Bytes      |      8 Bytes      |
    // +-------------------+-------------------+-------------------+
    let mut auth_id = [0u8; 16];
    let mut len = [0u8; 18];
    let mut nonce = [0u8; 8];
    stream.read_exact(&mut auth_id).await?;
    stream.read_exact(&mut len).await?;
    stream.read_exact(&mut nonce).await?;

    // https://github.com/v2fly/v2ray-core/blob/master/proxy/vmess/aead/kdf.go
    let header_length = {
        let header_length_key = &hash::kdf(
            &key,
            &[
                KDFSALT_CONST_VMESS_HEADER_PAYLOAD_LENGTH_AEAD_KEY,
                &auth_id,
                &nonce,
            ],
        )[..16];
        let header_length_nonce = &hash::kdf(
            &key,
            &[
                KDFSALT_CONST_VMESS_HEADER_PAYLOAD_LENGTH_AEAD_IV,
                &auth_id,
                &nonce,
            ],
        )[..12];

        let payload = Payload {
            msg: &len,
            aad: &auth_id,
        };

        let len = Aes128Gcm::new(header_length_key.into())
            .decrypt(header_length_nonce.into(), payload)
            .map_err(|e| Error::RustError(e.to_string()))?;

        ((len[0] as u16) << 8) | (len[1] as u16)
    };

    // 16 bytes padding
    let mut cmd = vec![0u8; (header_length + 16) as _];
    stream.read_exact(&mut cmd).await?;

    let header_payload = {
        let payload_key = &hash::kdf(
            &key,
            &[
                KDFSALT_CONST_VMESS_HEADER_PAYLOAD_AEAD_KEY,
                &auth_id,
                &nonce,
            ],
        )[..16];
        let payload_nonce = &hash::kdf(
            &key,
            &[KDFSALT_CONST_VMESS_HEADER_PAYLOAD_AEAD_IV, &auth_id, &nonce],
        )[..12];

        let payload = Payload {
            msg: &cmd,
            aad: &auth_id,
        };

        Aes128Gcm::new(payload_key.into())
            .decrypt(payload_nonce.into(), payload)
            .map_err(|e| Error::RustError(e.to_string()))?
    };

    Ok(header_payload)
}
