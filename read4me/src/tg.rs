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