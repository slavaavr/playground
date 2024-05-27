use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct TextEventRequest {
    pub update_id: i64,
    pub message: TextMessage,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TextMessage {
    pub date: i64,
    pub chat: Chat,
    pub message_id: i64,
    pub from: From,
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Chat {
    pub last_name: String,
    pub id: i64,
    #[serde(rename = "type")]
    pub r#type: String,
    pub first_name: String,
    pub username: String,
}


#[derive(Serialize, Deserialize, Debug)]
pub struct From {
    pub last_name: String,
    pub id: i64,
    pub first_name: String,
    pub username: String,
}