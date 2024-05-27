use crate::service::rate::{RateData, RateProvider};

pub struct Provider;

impl RateProvider for Provider {
    async fn get_usd_rate(&self) -> Result<RateData, String> {
        let url = "https://www.banki.ru/products/currencyNodejsApi/getBanksOrExchanges/?sortAttribute=sale&order=asc&regionUrl=sankt-peterburg&currencyId=840&amount=&page=1&latitude=59.939084&longitude=30.315879&isExchangeOffices=1";

        let client = reqwest::Client::new();
        
        let res: api::Response = client.get(url)
            .header("cache-control", "no-cache")
            .header("pragma", "no-cache")
            .header("x-requested-with", "XMLHttpRequest")
            .send().await
            .map_err(|err| format!("unable to build request: {}", err))?
            .json().await
            .map_err(|err| format!("unable to parse as json: {}", err))?;

        let res = &res.list[0];
        let price = res.exchange.sale;
        let info = format!("{}. {}", res.name, res.contact_information.address);

        return Ok(RateData::new(price, info));
    }
}

mod api {
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