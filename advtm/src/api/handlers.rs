use axum::{extract, Json};
use crate::api::{requests, worker};
use crate::api::server::AppState;
use crate::db::sqlite::schema::{Event, EventType};

pub async fn root(
    state: extract::State<AppState>,
    req: Json<requests::TextEventRequest>,
) -> () {
    println!("{:#?}", req);

    let text = get_text(&req);

    let e = Event {
        id: 0,
        chat_id: req.message.chat.id,
        typ: EventType::from(text),
        user: Some(req.message.from.first_name.clone()),
        meta: None,
    };

    let is_event_exist = |chat_id, typ| {
        state.db.clone()
            .lock()
            .unwrap()
            .get_event(chat_id, typ).is_some()
    };

    if is_event_exist(e.chat_id, e.typ.clone()) {
        state.db.clone()
            .lock()
            .unwrap()
            .delete_event(e.chat_id, e.typ.clone())
            .expect("unable to delete event");

        state.tx.send(worker::Data::new(e, worker::DataType::Delete)).await.unwrap();

        println!("event deleted");
    } else {
        state.db.clone()
            .lock()
            .unwrap()
            .add_event(e.clone())
            .expect("unable to add event");

        state.tx.send(worker::Data::new(e, worker::DataType::Add)).await.unwrap();

        println!("event created");
    }
}

fn get_text(req: &requests::TextEventRequest) -> String {
    let text = req.message.text.trim().to_string();

    if let Some(t) = text.strip_prefix("/") {
        return t.to_string();
    }

    return text;
}