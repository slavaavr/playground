use std::path::Path;
use axum_server::tls_rustls::RustlsConfig;
use crate::api::server::handlers;

pub struct Config {
    pub addr: String,
    pub cert_pem_path: String,
    pub key_pem_path: String,
}

pub async fn init(cfg: Config) {
    let tls_cfg = RustlsConfig::from_pem_file(
        Path::new(&cfg.cert_pem_path),
        Path::new(&cfg.key_pem_path),
    ).await.expect("unable to create tls config");

    let hdl = handlers::Handlers::new(todo!());

    let app = axum::Router::new()
        .route("/", axum::routing::get({
            || hdl.root()
        }));

    axum_server::bind_rustls(
        cfg.addr.parse().expect("unable to parse addr"),
        tls_cfg,
    ).serve(app.into_make_service()).await.unwrap();
}