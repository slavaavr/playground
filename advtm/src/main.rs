use serde::Deserialize;
use tracing::info;
use crate::client::telegram;

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
    let tg_valid_user_ids: Vec<String> = cfg.tg_valid_user_ids
        .split(",")
        .map(str::to_string)
        .collect();

    let telegram = telegram::Client::new(cfg.tg_token);
    let db = db::sqlite::Client::new();

    telegram.create_web_hook(cfg.server_address.clone()).await;
    info!("starting web server on address={}...", cfg.server_address);

    api::server::run(api::server::Config {
        address: cfg.server_address,
        cert_pem_path: cfg.cert_pem_path,
        key_pem_path: cfg.key_pem_path,
        db,
    }).await;

    info!("web server has been closed...");
}
