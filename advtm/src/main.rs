use serde::Deserialize;
use tracing::info;

mod client;
mod db;
mod service;
mod api;

const APP_NAME: &str = "advtm";

#[derive(Deserialize, Debug)]
struct Config {
    tg_token: String,
    tg_valid_user_ids: String,
    server_address: String,
    cert_pem_path: String,
    key_pem_path: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cfg = envy::from_env::<Config>().expect("unable to parse env variables");
    let tg_valid_user_ids: Vec<String> = cfg.tg_valid_user_ids.split(",").map(str::to_string).collect();

    client::telegram::init(&cfg.tg_token, &cfg.server_address).await;
    info!("starting web server on address={}...", cfg.server_address);

    api::server::init(api::server::Config {
        addr: cfg.server_address,
        cert_pem_path: cfg.cert_pem_path,
        key_pem_path: cfg.key_pem_path,
    }).await;


    info!("web server has been closed...");
}
