use anyhow::{Context, Result};
use axum::{
    response::{Html, IntoResponse},
    routing::post,
    Form, Router,
};
use dotenv::dotenv;
use openapi::apis::{
    api20100401_message_api::{create_message, CreateMessageParams},
    configuration::Configuration,
};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv()?;
    let twilio_config = Configuration {
        basic_auth: Some((
            env::var("TWILIO_API_KEY_SID")?,
            Some(env::var("TWILIO_API_KEY_SECRET")?),
        )),
        ..Default::default()
    };

    let app = Router::new().route("/", post(handle_incoming_sms));

    // run it
    let listener = tokio::net::TcpListener::bind(format!(
        "{}:{}",
        env::var("CALLBACK_IP")?,
        env::var("CALLBACK_PORT")?
    ))
    .await
    .unwrap();
    println!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

// field names must be exact (including case) to match API
#[allow(non_snake_case)]
#[derive(serde::Deserialize)]
struct SmsMessage {
    Body: String,
    From: String,
}

// Handler for incoming SMS messages
async fn handle_incoming_sms(
    Form(SmsMessage { Body, From }): Form<SmsMessage>,
) -> impl IntoResponse {
    Html(format!(
        r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <Response>
        <Message>Hello {From}, Thank you for your submission: {Body}</Message>
        </Response>
        "#
    ))
}

async fn send(twilio_config: &Configuration, to: String, message: String) -> Result<()> {
    let message_params = CreateMessageParams {
        account_sid: env::var("TWILIO_ACCOUNT_SID")?,
        to,
        from: Some(env::var("SERVER_NUMBER")?),
        body: Some(message),
        ..Default::default()
    };
    let message = create_message(twilio_config, message_params)
        .await
        .context("While sending message")?;
    println!("Message sent with SID {}", message.sid.unwrap().unwrap());
    Ok(())
}
