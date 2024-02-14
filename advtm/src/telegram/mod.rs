pub async fn init(token: &str) {
    #[cfg(feature = "dev")]
    create_web_hook(token)
}

fn create_web_hook(token: &str) {
    use frankenstein::AsyncApi;
    let api = AsyncApi::new(token);
}