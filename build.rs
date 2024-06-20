use std::env;

fn main() {
    let root = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is required");
    let path = env::var("CONFIG_PATH").unwrap_or(format!("{}/config.toml", root));
    println!("cargo:rustc-env=CONFIG_PATH={}", path);
}
