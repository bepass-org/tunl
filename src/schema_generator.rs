#![allow(unused)]

mod common;
mod config;
mod link;
mod proxy;

use crate::config::Config;

use schemars::gen::SchemaSettings;
use schemars::JsonSchema;

fn save_schema<T: JsonSchema>() {
    let settings = SchemaSettings::draft07();
    let gen = settings.into_generator();
    let schema = gen.into_root_schema_for::<T>();
    let schema_str = serde_json::to_string_pretty(&schema).unwrap();
    println!("{schema_str}")
}

fn main() {
    save_schema::<Config>();
}
