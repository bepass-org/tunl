//! Mock some udp based protocols.
//! - DNS: receive the dns request from in-bound but handle with DoH wireformat
//! - QUIC:
//!     TODO: maybe we can receive the request and act like a sni-proxy?
//! - WebRTC:
//!     TODO: maybe we can forge a response to force it to use TCP?

pub mod doh;
pub mod outbound;
