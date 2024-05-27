use std::collections::HashMap;
use tokio::sync::mpsc::{Receiver, Sender};
use crate::db::sqlite::schema::Event;
use crate::service::rate;

pub enum DataType {
    Add,
    Delete,
}

pub struct Data {
    pub event: Event,
    pub typ: DataType,
}

impl Data {
    pub fn new(e: Event, t: DataType) -> Self {
        Self { event: e, typ: t }
    }
}

pub struct Pool {
    rx: Receiver<Data>,
    rate_service: Box<dyn rate::RateProvider>,
}

impl Pool {
    pub fn new(
        rx: Receiver<Data>,
        rate_service: Box<dyn rate::RateProvider>,
    ) -> Self {
        Self {
            rx,
            rate_service,
        }
    }

    pub fn run(&mut self) {
        tokio::spawn(async move {
            let mem = HashMap::new();

            while let Some(d) = self.rx.recv().await {
                d.event.chat_id
            }
        });
    }
}