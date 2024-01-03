use std::{env, thread};
use std::sync::{Arc, mpsc};
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use frankenstein::{BotCommand, ChatId, GetUpdatesParams, SendMessageParams, SetMyCommandsParams, TelegramApi, UpdateContent};

enum ChanEvent {
    Price(f32),
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
    let mut last_price: f32 = 0.0;

    let send_event = |chat_id, price| {
        tg_api.send_message(&SendMessageParams::builder()
            .chat_id(ChatId::from(chat_id))
            .text(format!("usd: {price}"))
            .build(),
        ).expect("unable to send message to tg");
    };

    loop {
        let event = rx.recv().expect("unable to receive event");

        match event {
            ChanEvent::Price(p) => {
                last_price = p;
                println!("got new price {}", last_price);

                for chat_id in &chats {
                    send_event(chat_id.clone(), last_price);
                }
            }
            ChanEvent::AddChat(chat_id) => {
                chats.push(chat_id);
                send_event(chat_id, last_price);
            }
            ChanEvent::RemoveChat(chat_id) => {
                chats.retain(|&x| x != chat_id)
            }
        }
    }
}

fn run_usd_price_updater(tx: Sender<ChanEvent>, price_update_interval: Duration) {
    let usd_curr_url = "https://ligovka.ru/detailed/usd";
    let mut prev_price: f32 = 0.0;

    loop {
        let res = ureq::get(usd_curr_url)
            .call()
            .expect("unable to get usd currency forecast")
            .into_string()
            .expect("unable to convert response to string");

        let idx = res.find("<td class=\"money_quantity\">от 1000</td>")
            .expect("unable to find prices for quantity >= 1000 usd");
        let res = res[idx..].to_string();
        let res: String = res.splitn(2, "<td class=\"money_price\">").collect::<Vec<_>>()[1].into();
        let price: f32 = res.splitn(2, "<").collect::<Vec<_>>()[0].parse()
            .expect("unable to parse string as price");

        if price.ne(&prev_price) {
            tx.send(ChanEvent::Price(price)).expect("unable to send price to channel");
            prev_price = price;
        }

        thread::sleep(price_update_interval);
    }
}
