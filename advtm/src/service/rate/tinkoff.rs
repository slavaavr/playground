use crate::service::rate::*;

pub struct Provider;

impl RateProvider for Provider {
    async fn get_usd_rate(&self) -> Result<RateData, String> {
        let url = "https://api.tinkoff.ru/v1/currency_rates?from=USD&to=RUB";

        let res: api::Response = reqwest::get(url).await
            .map_err(|err| format!("unable to get rate: {}", err))?
            .json().await
            .map_err(|err| format!("unable to parse as json: {}", err))?;

        let rate = res.payload.rates.iter()
            .find(|&r| r.category == "DebitCardsTransfers")
            .ok_or("unable to find valid rate")?;

        return Ok(RateData::new(rate.sell, "from tinkoff".into()));
    }
}

mod api {
    use serde::Deserialize;

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Response {
        pub payload: Payload,
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Payload {
        pub rates: Vec<Rate>,
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Rate {
        pub category: String,
        pub buy: f64,
        pub sell: f64,
    }
}