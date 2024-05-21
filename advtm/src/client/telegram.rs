use frankenstein::{AsyncTelegramApi, AsyncApi, SetWebhookParams};

pub async fn init(token: &str, url: &str) {
    create_web_hook(token, url).await;
}

async fn create_web_hook(token: &str, url: &str) {
    let api = AsyncApi::new(token);
    api.set_webhook(&SetWebhookParams {
        url: url.into(),
        certificate: None,
        ip_address: None,
        max_connections: None,
        allowed_updates: None,
        drop_pending_updates: None,
        secret_token: Some(token.into()),
    }).await.expect("unable to set webhook");
}