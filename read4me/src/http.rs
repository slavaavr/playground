pub mod server {
    use std::path::Path;
    use std::sync::Arc;

    use axum::Router;
    use axum::middleware;
    use axum::routing::{delete, get, post};
    use axum_server::tls_rustls::RustlsConfig;
    use tokio::sync::Mutex;

    use crate::{db, rpc};
    use crate::http::fs;

    #[derive(Clone)]
    pub struct AppState {
        db_client: Arc<Mutex<db::sqlite::Client>>,
        tts_client: Arc<Mutex<rpc::tts::Client>>,
        tg_valid_user_ids: Vec<String>,
        tg_root_user_ids: Vec<String>,
    }

    mod urls {
        pub const ROOT: &str = "/";
        pub const AUTH: &str = "/auth";
        pub const SENTENCES: &str = "/sentences";
        pub const ADD_SENTENCE: &str = "/sentences";
        pub const DROP_SENTENCE: &str = "/sentences/:id";
        pub const PLAY_SENTENCE: &str = "/sentences/:id/play";
        pub const ASSETS: &str = "/assets";
    }

    mod tmpl {
        use askama::Template;

        #[derive(Template)]
        #[template(path = "index.html")]
        pub struct IndexTemplate {
            pub image_url: String,
            pub auth_url: String,
        }

        #[derive(Template)]
        #[template(path = "sentences.html")]
        pub struct SentencesTemplate {
            pub is_admin: bool,
            pub sentences_url: String,
            pub sentences: Vec<Sentence>,
        }

        pub struct Sentence {
            pub id: i32,
            pub text: String,
        }

        impl Sentence {
            pub fn new(id: i32, text: String) -> Self {
                Sentence { id, text }
            }
        }
    }

    mod request {
        use serde::Deserialize;

        #[derive(Deserialize, Debug)]
        pub struct Auth {
            pub tg_id: String,
        }

        #[derive(Deserialize, Debug)]
        pub struct AddSentence {
            pub text: String,
        }
    }

    mod handlers {
        use axum::{extract, response::Redirect};
        use axum::http::StatusCode;
        use axum_extra::extract::cookie::{Cookie, CookieJar};
        use tracing::error;

        use crate::http::fs;
        use crate::http::server::{AppState, request, tmpl, urls};

        const IMG: &str = "https://static.wixstatic.com/media/82daf4_25d109065ad2499485b2f605379022a4.jpg/v1/fill/w_516,h_560,al_c,lg_1,q_80,enc_auto/82daf4_25d109065ad2499485b2f605379022a4.jpg";

        pub async fn root() -> tmpl::IndexTemplate {
            tmpl::IndexTemplate {
                image_url: IMG.into(),
                auth_url: urls::AUTH.into(),
            }
        }

        pub async fn auth(
            extract::State(state): extract::State<AppState>,
            extract::Json(req): extract::Json<request::Auth>,
        ) -> axum::response::Result<(CookieJar, Redirect)> {
            if state.tg_valid_user_ids.contains(&req.tg_id) {
                let jar = CookieJar::new()
                    .add(Cookie::new("id", req.tg_id));

                return Ok((jar, Redirect::to(urls::SENTENCES)));
            }

            error!("got invalid tg_id='{}'", req.tg_id);
            Err(StatusCode::FORBIDDEN.into())
        }

        pub async fn sentences(
            extract::State(state): extract::State<AppState>,
            jar: CookieJar,
        ) -> axum::response::Result<tmpl::SentencesTemplate> {
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
                .map(|e| tmpl::Sentence::new(e.id, e.text.clone()))
                .collect();

            Ok(tmpl::SentencesTemplate {
                is_admin,
                sentences_url: urls::SENTENCES.into(),
                sentences: list,
            })
        }

        pub async fn add_sentence(
            extract::State(state): extract::State<AppState>,
            extract::Json(req): extract::Json<request::AddSentence>,
        ) -> axum::response::Result<String> {
            let id = state.db_client
                .lock().await
                .add_sentence(req.text)?;
            Ok(id.to_string())
        }

        pub async fn drop_sentence(
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

        pub async fn play_sentence(
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
    }

    mod mdlwr {
        use axum::extract;
        use axum::http::{Request, StatusCode};
        use axum::middleware::Next;
        use axum::response::Response;
        use axum_extra::extract::CookieJar;
        use tracing::error;

        use crate::http::server::AppState;

        pub async fn auth_layer<B>(
            extract::State(state): extract::State<AppState>,
            jar: CookieJar,
            request: Request<B>,
            next: Next<B>,
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
        let cfg = RustlsConfig::from_pem_file(
            Path::new(cert_pem_path),
            Path::new(key_pem_path),
        ).await.expect("unable to create tls config");

        let state = AppState {
            db_client: Arc::new(Mutex::new(db_client)),
            tts_client: Arc::new(Mutex::new(tts_client)),
            tg_valid_user_ids,
            tg_root_user_ids,
        };

        let auth_middleware = middleware::from_fn_with_state(
            state.clone(),
            mdlwr::auth_layer,
        );

        let app = Router::new()
            .route(urls::ROOT, get(handlers::root))
            .route(urls::AUTH, post(handlers::auth))
            .route(urls::SENTENCES, get(handlers::sentences)
                .route_layer(auth_middleware.clone()),
            )
            .route(urls::ADD_SENTENCE, post(handlers::add_sentence)
                .route_layer(auth_middleware.clone()),
            )
            .route(urls::DROP_SENTENCE, delete(handlers::drop_sentence)
                .route_layer(auth_middleware.clone()),
            )
            .route(urls::PLAY_SENTENCE, post(handlers::play_sentence)
                .route_layer(auth_middleware),
            )
            .nest_service(urls::ASSETS, fs::serve_dir().await)
            .with_state(state);


        axum_server::bind_rustls(addr.to_string().parse().expect("invalid address"), cfg)
            .serve(app.into_make_service()).await.unwrap();
    }
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


    fn audio_name(id: i32) -> String {
        format!("{id}.mp3")
    }

    fn audio_name_path(id: i32) -> String {
        format!("{ASSETS_DIR}/{}", audio_name(id))
    }
}