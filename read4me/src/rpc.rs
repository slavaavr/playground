pub mod tts {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use internal::*;
    use tonic::Request;
    use tonic::transport::{Channel, ClientTlsConfig};
    use tracing::error;

    mod internal {
        tonic::include_proto!("speechkit.tts.v3");
    }

    const FOLDER_ID: &str = "b1ghol6q54ma6v8o2mvk";

    const TTS_URL: &str = "https://tts.api.cloud.yandex.net:443";
    const IAM_URL: &str = "https://iam.api.cloud.yandex.net/iam/v1/tokens";

    pub struct Client {
        client: synthesizer_client::SynthesizerClient<Channel>,
        token: Arc<Mutex<String>>,
    }

    impl Client {
        pub async fn new(auth_token: &str) -> Self {
            let auth_token = auth_token.to_string();
            let token = Arc::new(Mutex::new(String::new()));
            let token2 = token.clone();

            tokio::spawn(async move {
                let http_client = reqwest::Client::new();
                let req_body = HashMap::from([("yandexPassportOauthToken", auth_token.as_str())]);
                let mut interval = tokio::time::interval(Duration::from_secs(6 * 60 * 60));
                loop {
                    interval.tick().await;

                    let res = http_client.post(IAM_URL)
                        .json(&req_body)
                        .send()
                        .await;

                    match res {
                        Ok(body) => {
                            let map = body
                                .json::<HashMap<String, String>>()
                                .await
                                .expect("unable to parse json body");
                            let val = map.get("iamToken")
                                .expect("token field not found").clone();
                            *token.lock().unwrap() = val;
                        }
                        Err(err) => error!("error requesting iam token: {}", err)
                    };
                }
            });

            let channel = Channel::from_static(TTS_URL)
                .tls_config(ClientTlsConfig::new()).unwrap()
                .timeout(Duration::from_secs(5))
                .rate_limit(5, Duration::from_secs(1))
                .concurrency_limit(256)
                .connect()
                .await
                .expect("unable to connect a channel");

            Self {
                client: synthesizer_client::SynthesizerClient::new(channel),
                token: token2,
            }
        }

        pub async fn synthesise_text(&mut self, text: String) -> Result<Vec<u8>, String> {
            let mut req = Request::new(UtteranceSynthesisRequest {
                model: "".into(),
                hints: vec![
                    Hints { hint: Some(hints::Hint::Speed(0.8)) },
                    Hints { hint: Some(hints::Hint::Voice("ermil".into())) },
                    Hints { hint: Some(hints::Hint::Role("neutral".into())) },
                ],
                output_audio_spec: None,
                loudness_normalization_type: 0,
                unsafe_mode: false,
                utterance: Some(utterance_synthesis_request::Utterance::Text(text)),
            });

            let token = format!("Bearer {}", self.token.lock().unwrap());

            req.metadata_mut().insert("authorization", token.parse().unwrap());
            req.metadata_mut().insert("x-folder-id", FOLDER_ID.parse().unwrap());

            let resp = self.client
                .utterance_synthesis(req)
                .await
                .map_err(|err| format!("unable to synthesise the text: {err}"))?;

            let mut resp = resp.into_inner();
            let mut audio = Vec::new();

            while let Some(it) = resp.message().await
                .map_err(|err| format!("unable to read the response: {err}"))? {
                if let Some(mut chunk) = it.audio_chunk {
                    audio.append(&mut chunk.data);
                }
            }

            Ok(audio)
        }
    }
}
