pub struct RateData {
    pub price: f64,
    pub description: String,
}

impl RateData {
    pub fn new(price: f64, description: String) -> Self {
        Self { price, description }
    }
}

pub trait RateProvider {
    async fn get_usd_rate(&self) -> Result<RateData, String>;
}
