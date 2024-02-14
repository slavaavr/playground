use scraper::{Html, Selector};

pub async fn get_houses() -> Result<Vec<String>, String> {
    let resp = reqwest::get("https://levada-b-h.by").await
        .map_err(|err| format!("unable to get levada: {err}"))?;

    let resp = resp.text().await.expect("unable to get text data from levada resp");

    let doc = Html::parse_document(&resp);
    let prices_selector = Selector::parse(".prices").expect("unable to create selector");
    let prices = doc.select(&prices_selector).next().expect("unable to get prices tag");

    let mut res = vec![];

    for txt in prices.text() {
        if txt.contains("Дом") {
            let txt = String::from(txt);
            if !res.contains(&txt) {
                res.push(txt)
            }
        }
    }

    return Ok(res);
}

#[cfg(test)]
mod test {
    use crate::service::levada::get_houses;

    #[tokio::test]
    async fn test() {
        let res = get_houses().await.unwrap();
        println!("{:?}", res)
    }
}