use serde::Deserialize;
use tracing::info;

#[cfg(feature = "dev")]
mod tg;

mod db;
mod rpc;
mod http;

const APP_NAME: &str = "read4me";

#[derive(Deserialize, Debug)]
struct Config {
    #[cfg(feature = "dev")]
    tg_token: String,
    tg_valid_user_ids: String,
    tg_root_user_ids: String,
    ya_auth_token: String,
    server_address: String,
    cert_pem_path: String,
    key_pem_path: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cfg = envy::from_env::<Config>().expect("unable to parse env variables");

    let db_client = db::sqlite::Client::new();
    let tts_client = rpc::tts::Client::new(&cfg.ya_auth_token).await;

    let tg_valid_user_ids = cfg.tg_valid_user_ids.split(",").map(str::to_string).collect();
    let tg_root_user_ids = cfg.tg_root_user_ids.split(",").map(str::to_string).collect();

    info!("starting web server on address={}...", cfg.server_address);
    http::server::init(
        db_client,
        tts_client,
        tg_valid_user_ids,
        tg_root_user_ids,
        &cfg.server_address,
        &cfg.cert_pem_path,
        &cfg.key_pem_path,
    ).await;
    info!("web server has been closed...");
}
