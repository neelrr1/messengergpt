use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct WebhookPayload {
    pub entry: Vec<Entry>
}

#[derive(Debug, Deserialize)]
pub struct Entry {
    pub messaging: Vec<IncomingMessage>
}

#[derive(Debug, Deserialize)]
pub struct IncomingMessage {
    pub sender: Sender,
    pub recipient: Recipient,
    pub message: Message,
}

#[derive(Debug, Serialize)]
pub struct OutgoingMessage {
    pub recipient: Recipient,
    pub messaging_type: MessagingType,
    pub message: Message,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum MessagingType {
    Response,
    Update,
}

#[derive(Debug, Deserialize)]
pub struct Sender {
    pub id: String,
    // user_ref: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Recipient {
    pub id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Message {
    pub mid: Option<String>,
    pub text: String,
}
