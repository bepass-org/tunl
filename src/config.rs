use std::net::IpAddr;
use std::sync::Arc;

use serde::Deserialize;
use uuid::{self, Uuid};
use wirefilter::{ExecutionContext, Field, Filter, Scheme};
#[cfg(not(test))]
use worker::console_log;

lazy_static::lazy_static! {
    static ref SCHEME: Scheme = Scheme! {
        ip: Ip,
        port: Int,
    };
}

pub fn fill_wirefilter_ctx<'a>(
    ip: String,
    port: u16,
    fields: &[Field<'a>],
) -> ExecutionContext<'a> {
    let mut ctx = ExecutionContext::new(&SCHEME);

    for field in fields.iter() {
        match field.name() {
            "ip" => {
                if let Ok(ip) = ip.parse::<IpAddr>() {
                    let _ = ctx.set_field_value(*field, ip);
                }
            }
            "port" => {
                let _ = ctx.set_field_value(*field, port as i32);
            }
            _ => {}
        }
    }

    ctx
}

#[derive(Clone)]
pub struct CompiledExpr {
    pub filter: Filter<'static>,
    pub fields: Arc<Vec<Field<'static>>>,
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outbound {
    Vless,
    Vmess,
}

impl Default for Outbound {
    fn default() -> Self {
        Self::Vmess
    }
}

#[derive(Default, Clone, Deserialize)]
pub struct RelayConfig {
    pub r#match: String,
    pub addresses: Vec<String>,
    // TODO: remove this and only use addresses field
    pub port: u16,
}

#[derive(Default, Clone, Deserialize)]
pub struct Config {
    pub uuid: Uuid,
    pub outbound: Outbound,
    pub relay: RelayConfig,
    // store the rules' AST because it's heavy to create them per every request
    #[serde(skip)]
    pub compiled_expr: Option<CompiledExpr>,
}

impl Config {
    pub fn new(buf: &str) -> Self {
        // TODO: notify the user in case of having an invalid config format
        let mut config: Self = toml::from_str(buf).unwrap_or_default();

        config.compiled_expr = match SCHEME.parse(&config.relay.r#match) {
            Ok(ast) => {
                let mut fields = vec![];
                SCHEME
                    .iter()
                    .filter(|(n, _)| ast.uses(n).unwrap_or(false))
                    .filter_map(|(n, _)| SCHEME.get_field(n).ok())
                    .for_each(|f| {
                        if !fields.contains(&f) {
                            fields.push(f);
                        }
                    });
                let filter = ast.compile();

                Some(CompiledExpr {
                    filter,
                    fields: Arc::new(fields),
                })
            }
            Err(_e) => {
                #[cfg(not(test))]
                console_log!("{_e}");

                None
            }
        };

        config
    }

    pub fn is_relay_request(&self, addr: String, port: u16) -> bool {
        if let Some(expr) = &self.compiled_expr {
            let ctx = fill_wirefilter_ctx(addr, port, &expr.fields);
            return expr.filter.execute(&ctx).unwrap_or_default();
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
            uuid = "0fbf4f81-2598-4b6a-a623-0ead4cb9efa8"
            outbound = "vless"

            [relay]
            match = "ip == 4.2.2.4 && port == 53"
            addresses = ["1.1.1.1"]
            port = 6666
        "#;
        let config = Config::new(buf);

        assert_eq!(config.outbound, Outbound::Vless);
        assert_eq!(
            config.uuid,
            uuid::uuid!("0fbf4f81-2598-4b6a-a623-0ead4cb9efa8")
        );
        assert_eq!(config.relay.r#match, "ip == 4.2.2.4 && port == 53");
        assert_eq!(config.relay.addresses, vec!["1.1.1.1"]);
    }
}
