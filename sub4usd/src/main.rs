use std::{env, thread};
use std::cmp::max;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use chrono::Timelike;
use frankenstein::{BotCommand, ChatId, Error, GetUpdatesParams, SendMessageParams, SetMyCommandsParams, TelegramApi, UpdateContent};
use regex::Regex;
use tracing::{error, info};

use crate::exchange::RateData;

mod exchange;

enum ChanEvent {
    Price((f64, String)),
    AddChat(i64),
    RemoveChat(i64),
}

struct State {
    price_update_interval: Duration,
}

fn main() {
    tracing_subscriber::fmt::init();

    let tg_token: String = env::var("TG_TOKEN")
        .expect("unable to get TG_TOKEN env");

    let tg_chats: Vec<i64> = env::var_os("TG_CHATS")
        .unwrap_or_default()
        .into_string()
        .expect("unable to cast TG_CHATS env to string")
        .split(",")
        .filter(|&s| !s.is_empty())
        .map(|s| s.parse::<i64>().unwrap())
        .collect();

    let state = Arc::new(Mutex::new(State {
        price_update_interval: Duration::from_secs(3 * 60 * 60),
    }));
    let tg_api = Arc::new(frankenstein::Api::new(&tg_token));
    let provider = Box::new(exchange::TinkoffProvider);
    let (tx, rx) = mpsc::channel::<ChanEvent>();

    let tg_api_clone = tg_api.clone();
    let tx_clone = tx.clone();
    let state_clone = state.clone();

    thread::spawn(move || run_usd_price_updater(tx, provider, state));
    thread::spawn(move || run_tg_notifier(rx, tg_api, tg_chats));

    run_tg_loop(tx_clone, tg_api_clone, state_clone);
}

fn run_tg_loop(
    tx: Sender<ChanEvent>,
    tg_api: Arc<frankenstein::Api>,
    state: Arc<Mutex<State>>,
) {
    const SUBSCRIBE: &str = "subscribe";
    const UNSUBSCRIBE: &str = "unsubscribe";

    let update_interval_re = Regex::new(r"set:update_interval\s+(?<hour>\d+)h\s*").unwrap();

    tg_api.set_my_commands(&SetMyCommandsParams::builder()
        .commands(vec![
            BotCommand::builder().command(SUBSCRIBE).description("subscribe to currency usd").build(),
            BotCommand::builder().command(UNSUBSCRIBE).description("unsubscribe to currency usd").build(),
        ])
        .build(),
    ).expect("unable to set commands");

    let handle_command = |chat_id: i64, text: String| {
        if text == format!("/{SUBSCRIBE}") {
            info!("added chat_id {:?}", chat_id);
            tx.send(ChanEvent::AddChat(chat_id))
                .expect("unable to send add chat_id");
            return;
        }

        if text == format!("/{UNSUBSCRIBE}") {
            tx.send(ChanEvent::RemoveChat(chat_id))
                .expect("unable to send remove chat_id");
            return;
        }

        if update_interval_re.is_match(&text) {
            let caps = update_interval_re.captures(&text).unwrap();
            let hour = caps.name("hour").unwrap()
                .as_str().parse::<u64>().unwrap();
            state.lock().unwrap().price_update_interval = Duration::from_secs(hour * 60 * 60);
            return;
        }
    };

    let mut update_params = GetUpdatesParams::builder().build();

    loop {
        let res = tg_api
            .get_updates(&update_params);

        match res {
            Ok(res) => {
                for update in res.result {
                    update_params = GetUpdatesParams::builder()
                        .offset(update.update_id + 1)
                        .build();

                    if let UpdateContent::Message(msg) = update.content {
                        if let Some(text) = msg.text {
                            handle_command(msg.chat.id, text);
                        }
                    }
                }
            }
            Err(err) => {
                error!("error while getting updates from tg: {:?}", err);
                thread::sleep(Duration::from_secs(5 * 60))
            }
        }

        let hour = chrono::Local::now().hour();
        let sleep_dur_sec = if let 0..=6 = hour { max(1, 6 - hour) * 60 * 60 } else { 3 };

        thread::sleep(Duration::from_secs(sleep_dur_sec as u64));
    }
}

fn run_tg_notifier(
    rx: Receiver<ChanEvent>,
    tg_api: Arc<frankenstein::Api>,
    default_chats: Vec<i64>,
) {
    let mut chats: Vec<i64> = default_chats;
    let mut last_price = 0.0;
    let mut last_info = String::new();

    let send_event = |chat_id, price, info| -> Result<(), &str> {
        let res = tg_api.send_message(&SendMessageParams::builder()
            .chat_id(ChatId::from(chat_id))
            .text(format!("{price} :: {info}"))
            .build(),
        );

        if let Err(Error::Api(err)) = res {
            error!("sending event to tg: {:?}", err);

            if err.error_code == 403 {
                return Err("user has blocked the bot");
            }
        }

        return Ok(());
    };

    loop {
        let event = rx.recv().expect("unable to receive event");

        match event {
            ChanEvent::Price((price, info)) => {
                last_price = price;
                last_info = info;
                info!("got new price {}", last_price);

                for chat_id in chats.clone() {
                    if let Err(_) = send_event(chat_id, last_price, last_info.clone()) {
                        chats.retain(|&x| x != chat_id);
                    }
                }
            }
            ChanEvent::AddChat(chat_id) => {
                chats.push(chat_id);
                send_event(chat_id, last_price, last_info.clone()).unwrap();
            }
            ChanEvent::RemoveChat(chat_id) => {
                chats.retain(|&x| x != chat_id);
            }
        }
    }
}

fn run_usd_price_updater(
    tx: Sender<ChanEvent>,
    provider: Box<dyn exchange::RateProvider>,
    state: Arc<Mutex<State>>,
) {
    let mut prev_price: f64 = 0.0;

    loop {
        let RateData(price, info) = provider.get_usd_rate()
            .expect("unable to get rate");

        if price.ne(&prev_price) {
            tx.send(ChanEvent::Price((price, info))).expect("unable to send price to channel");
            prev_price = price;
        }

        let dur = {
            let state = state.lock().unwrap();
            state.price_update_interval
        };

        thread::sleep(dur);
    }
}

#[cfg(test)]
mod test {
    use regex::Regex;

    #[test]
    fn update_interval_re() {
        let update_interval_re = Regex::new(r"set:update_interval\s+(?<hour>\d+)h\s*")
            .expect("unable to parse regex");
        let caps = update_interval_re
            .captures("set:update_interval  42h ")
            .expect("unable to get captures");
        let hour = caps
            .name("hour").expect("unable to get hour")
            .as_str().parse::<u64>().expect("unable to parse hour");

        assert_eq!(hour, 42);
    }
}