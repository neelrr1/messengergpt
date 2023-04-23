use std::collections::HashMap;

use axum::{extract::Query, http::StatusCode, routing::get, Json, Router};
use ngrok::{prelude::TunnelBuilder, tunnel::UrlTunnel};
use types::{OpenAIMessage, OpenAIRequest, OpenAIResponse, OutgoingMessage, WebhookPayload};

use crate::types::{Message, MessagingType, Recipient};

use dotenv;

mod types;

#[macro_use]
extern crate lazy_static;

const PORT: i32 = 8080;
lazy_static! {
    static ref HTTP_CLIENT: reqwest::Client = reqwest::Client::new();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env variables
    dotenv::dotenv().ok();

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/webhook", get(verify_webhook).post(post_webhook));

    // Use ngrok in dev mode
    if dotenv::var("ENVIRONMENT").unwrap() == "dev" {
        let tun = ngrok::Session::builder()
            // Read the token from the NGROK_AUTHTOKEN environment variable
            .authtoken(dotenv::var("NGROK_AUTHTOKEN").unwrap())
            // Connect the ngrok session
            .connect()
            .await?
            // Start a tunnel with an HTTP edge
            .http_endpoint()
            .listen()
            .await?;
        println!("Ngrok Tunnel started on URL: {:?}", tun.url());

        axum::Server::builder(tun)
            .serve(app.into_make_service())
            .await
            .expect("Server failed to start!");
    } else {
        // run it with hyper on localhost:PORT
        axum::Server::bind(&format!("0.0.0.0:{}", PORT).parse().unwrap())
            .serve(app.into_make_service())
            .await
            .expect("Server failed to start!");
        println!("Serving HTTP traffic on port {}", PORT);
    }

    Ok(())
}

async fn post_webhook(Json(body): Json<WebhookPayload>) -> Result<String, StatusCode> {
    let event = body
        .entry
        .first()
        .expect("malformed input")
        .messaging
        .first()
        .expect("malformed input");
    println!("Webhook received with message: {}", event.message.text);
    if send_response(&event.sender.id, &event.message)
        .await
        .is_err()
    {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    Ok("Message received!".to_string())
}

async fn send_response(
    recipient_id: &String,
    incoming_message: &Message,
) -> Result<(), reqwest::Error> {
    let body = OutgoingMessage {
        messaging_type: MessagingType::Response,
        recipient: Recipient {
            id: recipient_id.to_owned(),
        },
        message: Message {
            mid: None,
            text: generate_response(&incoming_message.text).await?,
        },
    };

    HTTP_CLIENT
        .post("https://graph.facebook.com/v2.6/me/messages")
        .query(&[("access_token", dotenv::var("PAGE_ACCESS_TOKEN").unwrap())])
        .json(&body)
        .send()
        .await?;
    Ok(())
}

async fn generate_response(query: &str) -> Result<String, reqwest::Error> {
    let res = HTTP_CLIENT
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(dotenv::var("OPENAI_KEY").unwrap())
        .json(&OpenAIRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![OpenAIMessage {
                role: "user".to_string(),
                content: query.to_string(),
            }],
        })
        .send()
        .await?;

    Ok(res
        .json::<OpenAIResponse>()
        .await?
        .choices
        .first()
        .unwrap()
        .message
        .content
        .to_owned())
}

async fn verify_webhook(params: Query<HashMap<String, String>>) -> Result<String, StatusCode> {
    if params.contains_key("hub.mode")
        && params.contains_key("hub.verify_token")
        && params.contains_key("hub.challenge")
    {
        let mode = params.get("hub.mode").unwrap();
        let verify_token = params.get("hub.verify_token").unwrap();
        let challenge = params.get("hub.challenge").unwrap();

        if mode == "subscribe" && verify_token == &dotenv::var("VERIFY_TOKEN").unwrap() {
            println!("Webhook verified!");
            Ok(challenge.to_string())
        } else {
            Err(StatusCode::FORBIDDEN)
        }
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}
