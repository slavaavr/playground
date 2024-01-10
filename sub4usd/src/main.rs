use std::{env, thread};
use std::sync::{Arc, mpsc};
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use frankenstein::{BotCommand, ChatId, GetUpdatesParams, SendMessageParams, SetMyCommandsParams, TelegramApi, UpdateContent};

enum ChanEvent {
    Price((f64, String)),
    AddChat(i64),
    RemoveChat(i64),
}

fn main() {
    let tg_token = env::var("TG_TOKEN").expect("unable to get TG_TOKEN env");

    let price_update_interval = 3 * 60 * Duration::from_secs(60);
    let tg_api = Arc::new(frankenstein::Api::new(&tg_token));
    let (tx, rx) = mpsc::channel::<ChanEvent>();

    let tg_api_clone = tg_api.clone();
    let tx_clone = tx.clone();

    thread::spawn(move || run_usd_price_updater(tx, price_update_interval));
    thread::spawn(move || run_tg_notifier(tg_api, rx));

    run_tg_loop(tg_api_clone, tx_clone);
}

fn run_tg_loop(tg_api: Arc<frankenstein::Api>, tx: Sender<ChanEvent>) {
    const SUBSCRIBE: &str = "subscribe";
    const UNSUBSCRIBE: &str = "unsubscribe";

    tg_api.set_my_commands(&SetMyCommandsParams::builder()
        .commands(vec![
            BotCommand::builder().command(SUBSCRIBE).description("subscribe to currency usd").build(),
            BotCommand::builder().command(UNSUBSCRIBE).description("unsubscribe to currency usd").build(),
        ])
        .build(),
    ).expect("unable to set commands");

    let mut update_params = GetUpdatesParams::builder().build();

    loop {
        let res = tg_api
            .get_updates(&update_params)
            .expect("unable to get tg updates");

        for update in res.result {
            if let UpdateContent::Message(msg) = update.content {
                if let Some(text) = msg.text {
                    if text == format!("/{SUBSCRIBE}") {
                        println!("added chat_id {:?}", msg.chat.id);
                        tx.send(ChanEvent::AddChat(msg.chat.id)).expect("unable to send add chat_id");
                    } else if text == format!("/{UNSUBSCRIBE}") {
                        tx.send(ChanEvent::RemoveChat(msg.chat.id)).expect("unable to send remove chat_id");
                    }
                }
            }

            update_params = GetUpdatesParams::builder().offset(update.update_id + 1).build()
        }

        thread::sleep(Duration::from_secs(3))
    }
}

fn run_tg_notifier(tg_api: Arc<frankenstein::Api>, rx: Receiver<ChanEvent>) {
    let mut chats: Vec<i64> = vec![];
    let mut last_price = 0.0;
    let mut last_info = String::new();

    let send_event = |chat_id, price, info| {
        tg_api.send_message(&SendMessageParams::builder()
            .chat_id(ChatId::from(chat_id))
            .text(format!("{price}: || {info} ||"))
            .build(),
        ).expect("unable to send message to tg");
    };

    loop {
        let event = rx.recv().expect("unable to receive event");

        match event {
            ChanEvent::Price((price, info)) => {
                last_price = price;
                last_info = info;
                println!("got new price {}", last_price);

                for chat_id in &chats {
                    send_event(chat_id.clone(), last_price, last_info.clone());
                }
            }
            ChanEvent::AddChat(chat_id) => {
                chats.push(chat_id);
                send_event(chat_id, last_price, last_info.clone());
            }
            ChanEvent::RemoveChat(chat_id) => {
                chats.retain(|&x| x != chat_id)
            }
        }
    }
}

fn run_usd_price_updater(tx: Sender<ChanEvent>, price_update_interval: Duration) {
    let usd_curr_url = "https://www.banki.ru/products/currencyNodejsApi/getBanksOrExchanges/?sortAttribute=sale&order=asc&regionUrl=sankt-peterburg&currencyId=840&amount=&page=1&latitude=59.939084&longitude=30.315879&isExchangeOffices=1";
    let mut prev_price: f64 = 0.0;

    loop {
        let res: banki::Response = ureq::get(usd_curr_url)
            .set("cache-control", "no-cache")
            .set("pragma", "no-cache")
            .set("x-requested-with", "XMLHttpRequest")
            .call()
            .expect("unable to get forecast")
            .into_json()
            .expect("unable to parse response");

        let res = &res.list[0];
        let price = res.exchange.sale;
        let info = format!("{}. {}", res.name, res.contact_information.address);

        if price.ne(&prev_price) {
            tx.send(ChanEvent::Price((price, info))).expect("unable to send price to channel");
            prev_price = price;
        }

        thread::sleep(price_update_interval);
    }
}

mod banki {
    use serde::Deserialize;

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Response {
        pub list: Vec<ResponseItem>,
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct ResponseItem {
        pub id: i64,
        pub name: String,
        pub bank_name: String,
        pub exchange: Exchange,
        pub contact_information: ContactInformation,
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Exchange {
        pub buy: f64,
        pub sale: f64,
        pub symbol: String,
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct ContactInformation {
        pub address: String,
        pub phone: String,
        pub metro_station: Option<String>,
    }
}