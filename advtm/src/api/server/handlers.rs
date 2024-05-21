use std::sync::{Arc, Mutex};
use crate::db;

pub struct Handlers {
    db: Arc<Mutex<db::sqlite::Client>>,
}

impl Handlers {
    pub fn new(db: db::sqlite::Client) -> Self {
        Handlers {
            db: Arc::new(Mutex::new(db))
        }
    }

    pub async fn root(&self) -> impl axum::response::IntoResponse {
        todo!()
    }
}