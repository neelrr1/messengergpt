use serde::{Deserialize, Serialize};

// Messenger Webhook Types
#[derive(Debug, Deserialize)]
pub struct WebhookPayload {
    pub entry: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
pub struct Entry {
    pub messaging: Vec<IncomingMessage>,
}

#[derive(Debug, Deserialize)]
pub struct IncomingMessage {
    pub sender: Sender,
    pub recipient: Recipient,
    pub timestamp: u128,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mid: Option<String>,
    pub text: String,
}

// OpenAI Types
#[derive(Serialize)]
pub struct OpenAIRequest {
    pub model: String,
    pub messages: Vec<OpenAIMessage>
}

#[derive(Deserialize)]
pub struct OpenAIResponse {
    pub choices: Vec<OpenAIChoice>
}

#[derive(Deserialize)]
pub struct OpenAIChoice {
    pub message: OpenAIMessage
}

#[derive(Serialize, Deserialize)]
pub struct OpenAIMessage {
    pub role: String,
    pub content: String
}