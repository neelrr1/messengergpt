use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    extract::Query,
    handler::Handler,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::get,
    Json, Router,
};
use ngrok::{prelude::TunnelBuilder, tunnel::UrlTunnel};
use types::{OpenAIMessage, OpenAIRequest, OpenAIResponse, OutgoingMessage, WebhookPayload};

use crate::types::{Message, MessagingType, Recipient};

mod types;

#[macro_use]
extern crate lazy_static;

const FIVE_MINS_MILLIS: u128 = 1000 * 60 * 5;
const PORT: i32 = 80;
lazy_static! {
    static ref HTTP_CLIENT: reqwest::Client = reqwest::Client::new();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env variables
    dotenv::dotenv().ok();
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route(
            "/webhook",
            get(verify_webhook).post(post_webhook.layer(middleware::from_fn(always_successful))),
        );

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
        println!("Serving HTTP traffic on port {}", PORT);
        axum::Server::bind(&format!("0.0.0.0:{}", PORT).parse().unwrap())
            .serve(app.into_make_service())
            .await
            .expect("Server failed to start!");
    }

    Ok(())
}

// Since our server doesn't support the entire Messenger Webhook event spec (only text messages are supported),
// we need to ack all events that come to our webhook so that they don't get retried infinitely.
async fn always_successful<B>(request: Request<B>, next: Next<B>) -> Response {
    let mut response = next.run(request).await;
    *response.status_mut() = StatusCode::OK;

    response
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

    let is_old = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis()
        - event.timestamp
        > FIVE_MINS_MILLIS;
    let res = send_response(&event.sender.id, &event.message).await;
    if !is_old
        && res
            .is_err()
    {
        println!("Error processing webhook event!\n{}", res.err().unwrap());
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
    println!("Response: {}", body.message.text);

    HTTP_CLIENT
        .post("https://graph.facebook.com/v2.6/me/messages")
        .query(&[("access_token", dotenv::var("PAGE_ACCESS_TOKEN").unwrap())])
        .json(&body)
        .send()
        .await?;
    Ok(())
}

async fn generate_response(query: &str) -> Result<String, reqwest::Error> {
    // for free testing
    if query == "ping" {
        return Ok("pong".to_string());
    }

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
