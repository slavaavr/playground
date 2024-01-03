use serde::Deserialize;
use tracing::info;

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

mod http {
    pub mod server {
        use std::fmt::Debug;
        use std::path::Path;
        use std::sync::Arc;

        use askama::Template;
        use axum::{extract, response::Redirect, Router, routing::{delete, get, post}};
        use axum::http::{Request, StatusCode};
        use axum::middleware::{self, Next};
        use axum::response::Response;
        use axum_extra::extract::cookie::{Cookie, CookieJar};
        use axum_server::tls_rustls::RustlsConfig;
        use serde::Deserialize;
        use tokio::sync::Mutex;
        use tracing::error;

        use crate::{db, rpc};

        const IMG: &str = "https://static.wixstatic.com/media/82daf4_25d109065ad2499485b2f605379022a4.jpg/v1/fill/w_516,h_560,al_c,lg_1,q_80,enc_auto/82daf4_25d109065ad2499485b2f605379022a4.jpg";

        // templates

        #[derive(Template)]
        #[template(path = "index.html")]
        struct IndexTemplate {
            image_url: String,
            auth_url: String,
        }

        #[derive(Template)]
        #[template(path = "sentences.html")]
        struct SentencesTemplate {
            is_admin: bool,
            sentences_url: String,
            sentences: Vec<Sentence>,
        }

        struct Sentence {
            id: i32,
            text: String,
        }

        impl Sentence {
            pub fn new(id: i32, text: String) -> Self {
                Sentence { id, text }
            }
        }

        // request-response

        #[derive(Deserialize, Debug)]
        struct AuthRequest {
            tg_id: String,
        }

        #[derive(Deserialize, Debug)]
        struct AddSentenceRequest {
            text: String,
        }

        // state

        #[derive(Clone)]
        struct AppState {
            db_client: Arc<Mutex<db::sqlite::Client>>,
            tts_client: Arc<Mutex<rpc::tts::Client>>,
            tg_valid_user_ids: Vec<String>,
            tg_root_user_ids: Vec<String>,
        }

        // handlers

        mod urls {
            pub const ROOT: &str = "/";
            pub const AUTH: &str = "/auth";
            pub const SENTENCES: &str = "/sentences";
            pub const ADD_SENTENCE: &str = "/sentences";
            pub const DROP_SENTENCE: &str = "/sentences/:id";
            pub const PLAY_SENTENCE: &str = "/sentences/:id/play";
            pub const ASSETS: &str = "/assets";
        }

        async fn root() -> IndexTemplate {
            IndexTemplate {
                image_url: IMG.into(),
                auth_url: urls::AUTH.into(),
            }
        }

        async fn auth(
            extract::State(state): extract::State<AppState>,
            extract::Json(req): extract::Json<AuthRequest>,
        ) -> axum::response::Result<(CookieJar, Redirect)> {
            if state.tg_valid_user_ids.contains(&req.tg_id) {
                let jar = CookieJar::new()
                    .add(Cookie::new("id", req.tg_id));

                return Ok((jar, Redirect::to(urls::SENTENCES)))
            }

            error!("got invalid tg_id='{}'", req.tg_id);
            Err(StatusCode::FORBIDDEN.into())
        }

        async fn sentences(
            extract::State(state): extract::State<AppState>,
            jar: CookieJar,
        ) -> axum::response::Result<SentencesTemplate> {
            let id = jar.get("id")
                .expect("unable to get cookie").value().to_string();
            let mut is_admin = false;
            if state.tg_root_user_ids.contains(&id) {
                is_admin = true;
            }

            let list = state.db_client
                .lock().await
                .list_sentences()?
                .iter()
                .map(|e| Sentence::new(e.id, e.text.clone()))
                .collect();

            Ok(SentencesTemplate {
                is_admin,
                sentences_url: urls::SENTENCES.into(),
                sentences: list,
            })
        }

        async fn add_sentence(
            extract::State(state): extract::State<AppState>,
            extract::Json(req): extract::Json<AddSentenceRequest>,
        ) -> axum::response::Result<String> {
            let id = state.db_client
                .lock().await
                .add_sentence(req.text)?;
            Ok(id.to_string())
        }

        async fn drop_sentence(
            extract::State(state): extract::State<AppState>,
            extract::Path(id): extract::Path<i32>,
        ) -> axum::response::Result<()> {
            let s = state.db_client
                .lock().await
                .get_sentence(id)?;

            state.db_client
                .lock().await
                .drop_sentence(id)?;

            if s.uri.is_some() {
                fs::drop_audio(id).await?;
            }

            Ok(())
        }

        async fn play_sentence(
            extract::State(state): extract::State<AppState>,
            extract::Path(id): extract::Path<i32>,
        ) -> axum::response::Result<String> {
            let s = state.db_client
                .lock().await
                .get_sentence(id)?;

            let get_url = |uri| {
                format!("{}/{}", urls::ASSETS, uri)
            };

            if let Some(uri) = s.uri {
                return Ok(get_url(uri));
            }

            let audio = state.tts_client
                .lock().await
                .synthesise_text(s.text).await
                .map_err(|err| format!("unable to synthesise text: {err}"))?;

            let uri = fs::add_audio(s.id, audio).await?;

            state.db_client
                .lock().await
                .update_sentence_uri(id, uri.clone())?;

            Ok(get_url(uri))
        }

        async fn auth_layer<B>(
            extract::State(state): extract::State<AppState>,
            jar: CookieJar,
            request: Request<B>,
            next: Next<B>
        ) -> Result<Response, StatusCode> {
            if let Some(id) = jar.get("id") {
                let id = id.value().to_string();
                if state.tg_valid_user_ids.contains(&id) {
                    let response = next.run(request).await;
                    return Ok(response);
                }

                error!("got user with invalid id='{}'", id);
            }

            Err(StatusCode::UNAUTHORIZED)
        }

        mod fs {
            use tower_http::services::ServeDir;

            const ASSETS_DIR: &str = "./assets";

            pub async fn serve_dir() -> ServeDir {
                tokio::fs::create_dir_all(ASSETS_DIR)
                    .await
                    .expect("unable to create dir for storing assets");

                ServeDir::new(ASSETS_DIR)
            }

            fn audio_name(id: i32) -> String {
                format!("{id}.mp3")
            }

            fn audio_name_path(id: i32) -> String {
                format!("{ASSETS_DIR}/{}", audio_name(id))
            }

            pub async fn add_audio(id: i32, audio: Vec<u8>) -> Result<String, String> {
                tokio::fs::write(audio_name_path(id), audio).await
                    .map_err(|err| format!("unable to save audio: {err}"))?;

                Ok(audio_name(id))
            }

            pub async fn drop_audio(id: i32) -> Result<(), String> {
                tokio::fs::remove_file(audio_name_path(id))
                    .await
                    .map_err(|err| format!("unable to drop audio with id={id}: {err}"))
            }
        }

        pub async fn init(
            db_client: db::sqlite::Client,
            tts_client: rpc::tts::Client,
            tg_valid_user_ids: Vec<String>,
            tg_root_user_ids: Vec<String>,
            addr: &str,
            cert_pem_path: &str,
            key_pem_path: &str,
        ) {
            let cfg = RustlsConfig::from_pem_file(Path::new(cert_pem_path), Path::new(key_pem_path))
                .await
                .expect("unable to create tls config");

            let state = AppState {
                db_client: Arc::new(Mutex::new(db_client)),
                tts_client: Arc::new(Mutex::new(tts_client)),
                tg_valid_user_ids,
                tg_root_user_ids,
            };

            let auth_middleware = middleware::from_fn_with_state(state.clone(),auth_layer);

            let app = Router::new()
                .route(urls::ROOT, get(root))
                .route(urls::AUTH, post(auth))
                .route(urls::SENTENCES, get(sentences)
                    .route_layer(auth_middleware.clone()),
                )
                .route(urls::ADD_SENTENCE, post(add_sentence)
                    .route_layer(auth_middleware.clone()),
                )
                .route(urls::DROP_SENTENCE, delete(drop_sentence)
                    .route_layer(auth_middleware.clone()),
                )
                .route(urls::PLAY_SENTENCE, post(play_sentence)
                    .route_layer(auth_middleware)
                )
                .nest_service(urls::ASSETS, fs::serve_dir().await)
                .with_state(state);


            axum_server::bind_rustls(addr.to_string().parse().expect("invalid address"), cfg)
                .serve(app.into_make_service()).await.unwrap();
        }
    }
}

mod rpc {
    pub mod tts {
        use std::collections::HashMap;
        use std::sync::{Arc, Mutex};
        use std::time::Duration;

        use tonic::Request;
        use tonic::transport::{Channel, ClientTlsConfig};
        use tracing::error;

        use internal::*;

        mod internal {
            tonic::include_proto!("speechkit.tts.v3");
        }

        const FOLDER_ID: &str = "b1ghol6q54ma6v8o2mvk";

        const TTS_URL: &str = "https://tts.api.cloud.yandex.net:443";
        const IAM_URL: &str = "https://iam.api.cloud.yandex.net/iam/v1/tokens";

        pub struct Client {
            client: synthesizer_client::SynthesizerClient<Channel>,
            token: Arc<Mutex<String>>,
        }

        impl Client {
            pub async fn new(auth_token: &str) -> Self {
                let auth_token = auth_token.to_string();
                let token = Arc::new(Mutex::new(String::new()));
                let token2 = token.clone();

                tokio::spawn(async move {
                    let http_client = reqwest::Client::new();
                    let req_body = HashMap::from([("yandexPassportOauthToken", auth_token.as_str())]);
                    let mut interval = tokio::time::interval(Duration::from_secs(6 * 60 * 60));
                    loop {
                        interval.tick().await;

                        let res = http_client.post(IAM_URL)
                            .json(&req_body)
                            .send()
                            .await;

                        match res {
                            Ok(body) => {
                                let map = body
                                    .json::<HashMap<String, String>>()
                                    .await
                                    .expect("unable to parse json body");
                                let val = map.get("iamToken")
                                    .expect("token field not found").clone();
                                *token.lock().unwrap() = val;
                            }
                            Err(err) => error!("error requesting iam token: {}", err)
                        };
                    }
                });

                let channel = Channel::from_static(TTS_URL)
                    .tls_config(ClientTlsConfig::new()).unwrap()
                    .timeout(Duration::from_secs(5))
                    .rate_limit(5, Duration::from_secs(1))
                    .concurrency_limit(256)
                    .connect()
                    .await
                    .expect("unable to connect a channel");

                Self {
                    client: synthesizer_client::SynthesizerClient::new(channel),
                    token: token2,
                }
            }

            pub async fn synthesise_text(&mut self, text: String) -> Result<Vec<u8>, String> {
                let mut req = Request::new(UtteranceSynthesisRequest {
                    model: "".into(),
                    hints: vec![
                        Hints { hint: Some(hints::Hint::Speed(0.8)) },
                        Hints { hint: Some(hints::Hint::Voice("ermil".into())) },
                        Hints { hint: Some(hints::Hint::Role("neutral".into())) },
                    ],
                    output_audio_spec: None,
                    loudness_normalization_type: 0,
                    unsafe_mode: false,
                    utterance: Some(utterance_synthesis_request::Utterance::Text(text)),
                });

                let token = format!("Bearer {}", self.token.lock().unwrap());

                req.metadata_mut().insert("authorization", token.parse().unwrap());
                req.metadata_mut().insert("x-folder-id", FOLDER_ID.parse().unwrap());

                let resp = self.client
                    .utterance_synthesis(req)
                    .await
                    .map_err(|err| format!("unable to synthesise the text: {err}"))?;

                let mut resp = resp.into_inner();
                let mut audio = Vec::new();

                while let Some(it) = resp.message().await
                    .map_err(|err| format!("unable to read the response: {err}"))? {
                    if let Some(mut chunk) = it.audio_chunk {
                        audio.append(&mut chunk.data);
                    }
                }

                Ok(audio)
            }
        }
    }
}

mod db {
    pub mod sqlite {
        use rusqlite::{Connection, Row};
        use sea_query::{
            ColumnDef,
            ConditionalStatement,
            Expr,
            Iden,
            Order,
            OrderedStatement,
            Query,
            SchemaStatementBuilder,
            SqliteQueryBuilder,
            Table,
        };
        use sea_query_rusqlite::RusqliteBinder;

        #[derive(Iden)]
        enum SentenceIden {
            #[iden = "sentence"]
            Table,
            Id,
            Text,
            Uri,
        }

        pub struct Sentence {
            pub id: i32,
            pub text: String,
            pub uri: Option<String>,
        }

        impl From<&Row<'_>> for Sentence {
            fn from(row: &Row) -> Self {
                Self {
                    id: row.get_unwrap(SentenceIden::Id.to_string().as_str()),
                    text: row.get_unwrap(SentenceIden::Text.to_string().as_str()),
                    uri: row.get_unwrap(SentenceIden::Uri.to_string().as_str()),
                }
            }
        }

        pub struct Client {
            conn: Connection,
        }

        impl Client {
            pub fn new() -> Self {
                let conn = Connection::open(format!("{}.db", crate::APP_NAME)).expect("unable to connect db");

                let init_schema = Table::create()
                    .table(SentenceIden::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SentenceIden::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key()
                    )
                    .col(ColumnDef::new(SentenceIden::Text).text().not_null())
                    .col(ColumnDef::new(SentenceIden::Uri).text().null())
                    .build(SqliteQueryBuilder);

                conn.execute(&init_schema, []).expect("unable to init schema");

                Self { conn }
            }

            pub fn add_sentence(&self, text: String) -> Result<i32, String> {
                let sql = Query::insert()
                    .into_table(SentenceIden::Table)
                    .columns([SentenceIden::Text])
                    .values_panic([text.into()])
                    .build_rusqlite(SqliteQueryBuilder);


                let mut stmt = self.conn.prepare(&sql.0).expect("unable to prepare stmt");
                let id = stmt.insert(&*sql.1.as_params())
                    .map_err(|err| format!("unable to insert sentence: {err}"))?;

                Ok(id as i32)
            }

            pub fn drop_sentence(&self, id: i32) -> Result<(), String> {
                let sql = Query::delete()
                    .from_table(SentenceIden::Table)
                    .and_where(Expr::col(SentenceIden::Id).eq(id))
                    .build_rusqlite(SqliteQueryBuilder);

                self.conn
                    .execute(&sql.0, &*sql.1.as_params())
                    .map_err(|err| format!("unable to drop sentence id='{id}: {err}'"))?;

                Ok(())
            }

            pub fn get_sentence(&self, id: i32) -> Result<Sentence, String> {
                let sql = Query::select()
                    .columns([SentenceIden::Id, SentenceIden::Text, SentenceIden::Uri])
                    .from(SentenceIden::Table)
                    .and_where(Expr::col(SentenceIden::Id).eq(id))
                    .build_rusqlite(SqliteQueryBuilder);

                let mut stmt = self.conn.prepare(sql.0.as_str()).expect("unable to prepare stmt");
                let res = stmt.query_row(&*sql.1.as_params(), |row| Ok(Sentence::from(row)))
                    .map_err(|err| format!("unable to get sentence id='{id}': {err}"))?;

                Ok(res)
            }

            pub fn list_sentences(&self) -> Result<Vec<Sentence>, String> {
                let sql = Query::select()
                    .columns([SentenceIden::Id, SentenceIden::Text, SentenceIden::Uri])
                    .from(SentenceIden::Table)
                    .order_by(SentenceIden::Id, Order::Desc)
                    .build_rusqlite(SqliteQueryBuilder);

                let mut stmt = self.conn.prepare(sql.0.as_str()).expect("unable to prepare stmt");
                let mut rows = stmt.query(&*sql.1.as_params())
                    .map_err(|err| format!("unable to list sentences: {err}"))?;

                let mut res = Vec::new();

                while let Some(row) = rows.next().map_err(|err| format!("unable to do next(): {err}"))? {
                    res.push(Sentence::from(row));
                }

                Ok(res)
            }

            pub fn update_sentence_uri(&self, id: i32, uri: String) -> Result<(), String> {
                let sql = Query::update()
                    .table(SentenceIden::Table)
                    .value(SentenceIden::Uri, uri)
                    .and_where(Expr::col(SentenceIden::Id).eq(id))
                    .build_rusqlite(SqliteQueryBuilder);

                let mut stmt = self.conn.prepare(sql.0.as_str()).expect("unable to prepare stmt");
                stmt.execute(&*sql.1.as_params())
                    .map_err(|err| format!("unable to update sentence with id={id}: {err}"))?;

                Ok(())
            }
        }
    }
}

#[cfg(feature = "dev")]
mod tg {
    use frankenstein::{AsyncApi, AsyncTelegramApi, MenuButton, MenuButtonWebApp, SetChatMenuButtonParams, WebAppInfo};

    pub async fn init(token: &str, api_url: &str) {
        let api = AsyncApi::new(token);
        set_chat_menu_btn(&api, api_url).await;
    }

    async fn set_chat_menu_btn(api: &AsyncApi, api_url: &str) {
        api.set_chat_menu_button(
            SetChatMenuButtonParams::builder()
                .menu_button(
                    MenuButton::WebApp(MenuButtonWebApp::builder()
                        .text(crate::APP_NAME)
                        .web_app(WebAppInfo::builder().url(api_url.to_string()).build())
                        .build())
                )
                .build(),
        ).await.expect("unable to update chat menu button");
    }
}
