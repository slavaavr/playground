use std::path::Path;
use std::sync::{Arc, Mutex};
use axum_server::tls_rustls::RustlsConfig;
use tokio::sync::mpsc::Sender;
use crate::api::{handlers, worker};
use crate::db;

pub struct Config {
    pub address: String,
    pub cert_pem_path: String,
    pub key_pem_path: String,
    pub db: db::sqlite::Client,
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<db::sqlite::Client>>,
    pub tx: Sender<worker::Data>,
}

pub async fn run(cfg: Config) {
    let (tx, rx) = tokio::sync::mpsc::channel(1);

    let app = axum::Router::new()
        .route("/", axum::routing::post(handlers::root))
        .with_state(AppState {
            db: Arc::new(Mutex::new(cfg.db)),
            tx,
        });

    let tls_cfg = RustlsConfig::from_pem_file(
        Path::new(&cfg.cert_pem_path),
        Path::new(&cfg.key_pem_path),
    ).await.expect("unable to create tls config");

    worker::run(rx);

    axum_server::bind_rustls(
        cfg.address.parse().expect("unable to parse addr"),
        tls_cfg,
    ).serve(app.into_make_service()).await.expect("unable to serve requests");
}