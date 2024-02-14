pub struct RateData(pub f64, pub String);

pub trait RateProvider {
    async fn get_usd_rate(&self) -> Result<RateData, String>;
}

pub struct TinkoffProvider;

impl RateProvider for TinkoffProvider {
    async fn get_usd_rate(&self) -> Result<RateData, String> {
        mod tinkoff {
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

        let url = "https://api.tinkoff.ru/v1/currency_rates?from=USD&to=RUB";

        let res: tinkoff::Response = reqwest::get(url)
            .call()
            .map_err(|err| format!("unable to get forecast: {}", err))?
            .into_json()
            .map_err(|err| format!("unable to parse response: {}", err))?;


        let rate = res.payload.rates.iter()
            .find(|&r| r.category == "DebitCardsTransfers")
            .ok_or("unable to find valid rate")?;

        return Ok(RateData(rate.sell, "from tinkoff".into()));
    }
}

pub struct BankiProvider;

impl RateProvider for BankiProvider {
    async fn get_usd_rate(&self) -> Result<RateData, String> {
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

        let url = "https://www.banki.ru/products/currencyNodejsApi/getBanksOrExchanges/?sortAttribute=sale&order=asc&regionUrl=sankt-peterburg&currencyId=840&amount=&page=1&latitude=59.939084&longitude=30.315879&isExchangeOffices=1";

        let res: banki::Response = reqwest::get(url)
            .set("cache-control", "no-cache")
            .set("pragma", "no-cache")
            .set("x-requested-with", "XMLHttpRequest")
            .call()
            .map_err(|err| format!("unable to get forecast: {}", err))?
            .into_json()
            .map_err(|err| format!("unable to parse response: {}", err))?;

        let res = &res.list[0];
        let price = res.exchange.sale;
        let info = format!("{}. {}", res.name, res.contact_information.address);

        return Ok(RateData(price, info));
    }
}