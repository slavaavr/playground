use frankenstein::{AsyncTelegramApi, AsyncApi, SetWebhookParams};

pub struct Client {
    api: AsyncApi,
}

impl Client {
    pub fn new(token: String) -> Self {
        Self {
            api: AsyncApi::new(&token)
        }
    }

    pub async fn create_web_hook(&self, url: String) {
        self.api.set_webhook(&SetWebhookParams {
            url: url.into(),
            certificate: None,
            ip_address: None,
            max_connections: None,
            allowed_updates: None,
            drop_pending_updates: None,
            secret_token: None,
        }).await.expect("unable to set webhook");
    }
}