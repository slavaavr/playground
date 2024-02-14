use serde::Deserialize;
use tracing::info;

mod db;
mod telegram;
mod service;

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
    let tg_valid_user_ids = cfg.tg_valid_user_ids.split(",").map(str::to_string).collect();

    telegram::init(&cfg.tg_token);
    info!("starting web server on address={}...", cfg.server_address);


    info!("web server has been closed...");
}
