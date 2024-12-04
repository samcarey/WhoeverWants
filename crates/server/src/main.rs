use crate::command::Command;
use anyhow::{bail, Context, Result};
use axum::{
    response::{Html, IntoResponse},
    routing::post,
    Extension, Form, Router,
};
use dotenv::dotenv;
use enum_iterator::all;
use ical::parser::vcard::component::VcardContact;
use log::*;
use once_cell::sync::Lazy;
use openapi::apis::{
    api20100401_message_api::{create_message, CreateMessageParams},
    configuration::Configuration,
};
use sqlx::{query, query_as, Pool, Sqlite};
use std::collections::HashMap;
use std::env;
use std::sync::Mutex;
use std::time::{Duration, Instant};

mod command;
#[cfg(test)]
mod test;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv()?;
    env_logger::init();
    info!("Starting up");
    let twilio_config = Configuration {
        basic_auth: Some((
            env::var("TWILIO_API_KEY_SID")?,
            Some(env::var("TWILIO_API_KEY_SECRET")?),
        )),
        ..Default::default()
    };
    send(
        &twilio_config,
        env::var("CLIENT_NUMBER")?,
        "Server is starting up".to_string(),
    )
    .await?;
    let pool = sqlx::SqlitePool::connect(&env::var("DATABASE_URL")?).await?;
    let app = Router::new()
        .route("/", post(handle_incoming_sms))
        .layer(Extension(pool));
    let listener = tokio::net::TcpListener::bind(format!(
        "{}:{}",
        env::var("CALLBACK_IP")?,
        env::var("CALLBACK_PORT")?
    ))
    .await?;
    info!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}

// field names must be exact (including case) to match API
#[allow(non_snake_case)]
#[derive(serde::Deserialize, Default, Debug)]
struct SmsMessage {
    Body: String,
    From: String,
    NumMedia: Option<String>,
    MediaContentType0: Option<String>,
    MediaUrl0: Option<String>,
}

struct User {
    number: String,
    #[allow(dead_code)]
    name: String,
}

#[derive(Clone)]
struct Contact {
    #[allow(unused)]
    id: i64,
    contact_name: String,
    contact_number: String,
}

// Handler for incoming SMS messages
async fn handle_incoming_sms(
    Extension(pool): Extension<Pool<Sqlite>>,
    Form(message): Form<SmsMessage>,
) -> impl IntoResponse {
    let response = match process_message(&pool, message).await {
        Ok(response) => response,
        Err(error) => {
            error!("Error: {error:?}");
            "Internal Server Error!".to_string()
        }
    };
    debug!("Sending response: {response}");
    Html(format!(
        r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <Response>
        <Message>{response}</Message>
        </Response>
        "#
    ))
}

async fn process_message(pool: &Pool<Sqlite>, message: SmsMessage) -> anyhow::Result<String> {
    trace!("Received {message:?}");
    let SmsMessage {
        Body: body,
        From: from,
        NumMedia,
        MediaContentType0,
        MediaUrl0,
    } = message;
    debug!("Received from {from}: {body}");
    if NumMedia == Some("1".to_string())
        && MediaContentType0
            .map(|t| ["text/vcard", "text/x-vcard"].contains(&&t.as_str()))
            .unwrap_or(false)
    {
        let vcard_data = reqwest::get(&MediaUrl0.unwrap()).await?.text().await?;
        let reader = ical::VcardParser::new(vcard_data.as_bytes());
        let mut stats = ImportStats::default();

        for vcard in reader {
            match process_vcard(pool, &from, vcard).await {
                Ok(ImportResult::Added) => stats.added += 1,
                Ok(ImportResult::Updated) => stats.updated += 1,
                Ok(ImportResult::Unchanged) => stats.skipped += 1,
                Err(e) => stats.add_error(&e.to_string()),
            }
        }

        return Ok(stats.format_report());
    }

    let mut words = body.trim().split_ascii_whitespace();
    let command_word = words.next();
    let command = command_word.map(|word| Command::try_from(word));

    let Some(User {
        number, name: _, ..
    }) = query_as!(User, "select * from users where number = ?", from)
        .fetch_optional(pool)
        .await?
    else {
        return onboard_new_user(command, words, &from, pool).await;
    };

    let Some(command) = command else {
        return Ok(Command::h.hint());
    };

    let Ok(command) = command else {
        return Ok(format!(
            "We didn't recognize that command word: \"{}\".\n{}",
            command_word.unwrap(),
            Command::h.hint()
        ));
    };

    let response = match command {
        // I would use HELP for the help command, but Twilio intercepts and does not relay that
        Command::h => {
            let available_commands = format!(
                "Available commands:\n{}\n",
                all::<Command>()
                    .map(|c| format!("- {c}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            format!("{available_commands}\n{}", Command::info.hint())
        }
        Command::name => match process_name(words) {
            Ok(name) => {
                query!("update users set name = ? where number = ?", name, from)
                    .execute(pool)
                    .await?;
                format!("Your name has been updated to \"{name}\"")
            }
            Err(hint) => hint.to_string(),
        },
        Command::stop => {
            query!("delete from users where number = ?", number)
                .execute(pool)
                .await?;
            // They won't actually see this when using Twilio
            "You've been unsubscribed. Goodbye!".to_string()
        }
        Command::info => {
            let command_text = words.next();
            if let Some(command) = command_text.map(|word| Command::try_from(word)) {
                if let Ok(command) = command {
                    format!(
                        "{} to {}.{}",
                        command.usage(),
                        command.description(),
                        command.example()
                    )
                } else {
                    format!("Command \"{}\" not recognized", command_text.unwrap())
                }
            } else {
                Command::info.hint()
            }
        }
        Command::friends => {
            let contacts = query_as!(
                Contact,
                "SELECT id as \"id!\", contact_name, contact_number FROM contacts WHERE submitter_number = ? ORDER BY contact_name",
                from
            )
            .fetch_all(pool)
            .await?;

            if contacts.is_empty() {
                "You haven't added any contacts yet.".to_string()
            } else {
                let contact_list = contacts
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        format!(
                            "{}. {} ({})",
                            i + 1,
                            c.contact_name,
                            &c.contact_number[2..5]
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("Your contacts:\n{}", contact_list)
            }
        }
        Command::delete => {
            let name = words.collect::<Vec<_>>().join(" ");
            if name.is_empty() {
                Command::delete.hint()
            } else {
                handle_delete(pool, &from, &name).await?
            }
        }
        Command::confirm => {
            let num = words.next();
            match num {
                Some(num) => handle_confirm(pool, &from, num).await?,
                None => Command::confirm.hint(),
            }
        }
    };
    Ok(response)
}

async fn handle_delete(pool: &Pool<Sqlite>, from: &str, name: &str) -> anyhow::Result<String> {
    cleanup_pending_deletions();

    let like = format!("%{}%", name.to_lowercase());
    let contacts = query_as!(
        Contact,
        "SELECT id as \"id!\", contact_name, contact_number 
         FROM contacts 
         WHERE submitter_number = ? 
         AND LOWER(contact_name) LIKE ?
         ORDER BY contact_name",
        from,
        like
    )
    .fetch_all(pool)
    .await?;

    if contacts.is_empty() {
        return Ok(format!("No contacts found matching \"{}\"", name));
    }

    let list = contacts
        .iter()
        .enumerate()
        .map(|(i, c)| {
            format!(
                "{}. {} ({})",
                i + 1,
                c.contact_name,
                &c.contact_number[2..5]
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let response = format!(
        "Found these contacts matching \"{}\":\n{}\n\nTo delete, reply \"confirm NUM\", where NUM is a number from the list above.",
        name, list
    );

    // For each contact, generate a unique token and store the deletion request
    for (i, contact) in contacts.iter().enumerate() {
        let token = format!("{}:{}", from, i + 1);
        PENDING_DELETIONS.lock().unwrap().insert(
            token,
            PendingDeletion {
                contact_id: contact.id,
                timestamp: Instant::now(),
            },
        );
    }

    Ok(response)
}

async fn handle_confirm(pool: &Pool<Sqlite>, from: &str, num: &str) -> anyhow::Result<String> {
    let token = format!("{}:{}", from, num);

    // Get and remove the pending deletion in a single atomic operation
    let contact_id = {
        let mut deletions = PENDING_DELETIONS.lock().unwrap();
        match deletions.remove(&token) {
            Some(PendingDeletion {
                contact_id,
                timestamp,
            }) if timestamp.elapsed() <= DELETION_TIMEOUT => {
                // Remove all other pending deletions for this user
                deletions.retain(|k, _| !k.starts_with(&format!("{}:", from)));
                Some(contact_id)
            }
            _ => None,
        }
    };

    let Some(contact_id) = contact_id else {
        return Ok(
            "Invalid or expired deletion request. Please try again with \"delete NAME\"."
                .to_string(),
        );
    };

    // Fetch the contact details before deletion for the confirmation message
    let contact = query_as!(
        Contact,
        "SELECT id as \"id!\", contact_name, contact_number FROM contacts WHERE id = ? AND submitter_number = ?",
        contact_id,
        from
    )
    .fetch_optional(pool)
    .await?;

    let Some(contact) = contact else {
        return Ok("Contact no longer exists.".to_string());
    };

    // Delete the contact
    query!(
        "DELETE FROM contacts WHERE id = ? AND submitter_number = ?",
        contact_id,
        from
    )
    .execute(pool)
    .await?;

    Ok(format!(
        "Deleted contact: {} ({})",
        contact.contact_name,
        &contact.contact_number[2..5]
    ))
}

fn cleanup_pending_deletions() {
    PENDING_DELETIONS
        .lock()
        .unwrap()
        .retain(|_, deletion| deletion.timestamp.elapsed() <= DELETION_TIMEOUT);
}

enum ImportResult {
    Added,
    Updated,
    Unchanged,
}

async fn process_vcard(
    pool: &Pool<Sqlite>,
    from: &str,
    vcard: Result<VcardContact, ical::parser::ParserError>,
) -> anyhow::Result<ImportResult> {
    let card = vcard?;

    let name = card
        .properties
        .iter()
        .find(|p| p.name == "FN")
        .and_then(|p| p.value.as_ref())
        .ok_or_else(|| anyhow::anyhow!("No name provided"))?;

    let number = card
        .properties
        .iter()
        .find(|p| p.name == "TEL")
        .and_then(|p| p.value.as_ref())
        .ok_or_else(|| anyhow::anyhow!("No number provided"))?;

    // First check if contact exists with different name
    let existing = query!(
        "SELECT contact_name FROM contacts 
         WHERE submitter_number = ? AND contact_number = ?",
        from,
        number
    )
    .fetch_optional(pool)
    .await?;

    match existing {
        Some(row) if row.contact_name == *name => Ok(ImportResult::Unchanged),
        Some(_) => {
            query!(
                "UPDATE contacts 
                 SET contact_name = ?
                 WHERE submitter_number = ? AND contact_number = ?",
                name,
                from,
                number
            )
            .execute(pool)
            .await?;
            Ok(ImportResult::Updated)
        }
        None => {
            query!(
                "INSERT INTO contacts (submitter_number, contact_name, contact_number) 
                 VALUES (?, ?, ?)",
                from,
                name,
                number
            )
            .execute(pool)
            .await?;
            Ok(ImportResult::Added)
        }
    }
}

async fn onboard_new_user(
    command: Option<Result<Command, serde_json::Error>>,
    words: impl Iterator<Item = &str>,
    from: &str,
    pool: &Pool<Sqlite>,
) -> anyhow::Result<String> {
    let Some(Ok(Command::name)) = command else {
        return Ok(format!(
            "Greetings! This is Decision Bot (https://github.com/samcarey/decisionbot).\n\
            To participate:\n{}",
            Command::name.hint()
        ));
    };
    Ok(match process_name(words) {
        Ok(name) => {
            query!("insert into users (number, name) values (?, ?)", from, name)
                .execute(pool)
                .await?;
            format!("Hello, {name}! {}", Command::h.hint())
        }
        Err(hint) => hint.to_string(),
    })
}

fn process_name<'a>(words: impl Iterator<Item = &'a str>) -> Result<String> {
    let name = words.collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        bail!("{}", Command::name.usage());
    }
    const MAX_NAME_LEN: usize = 20;
    if name.len() > MAX_NAME_LEN {
        bail!(
            "That name is {} characters long.\n\
            Please shorten it to {MAX_NAME_LEN} characters or less.",
            name.len()
        );
    }
    Ok(name)
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
    trace!("Message sent with SID {}", message.sid.unwrap().unwrap());
    Ok(())
}
#[derive(Default)]
struct ImportStats {
    added: usize,
    updated: usize,
    skipped: usize,
    failed: usize,
    errors: std::collections::HashMap<String, usize>,
}

impl ImportStats {
    fn add_error(&mut self, error: &str) {
        *self.errors.entry(error.to_string()).or_insert(0) += 1;
        self.failed += 1;
    }

    fn format_report(&self) -> String {
        let mut report = format!(
            "Processed contacts: {} added, {} updated, {} unchanged, {} failed",
            self.added, self.updated, self.skipped, self.failed
        );

        if !self.errors.is_empty() {
            report.push_str("\nErrors encountered:");
            for (error, count) in &self.errors {
                report.push_str(&format!("\n- {} × {}", count, error));
            }
        }
        report
    }
}

struct PendingDeletion {
    contact_id: i64,
    timestamp: Instant,
}

static PENDING_DELETIONS: Lazy<Mutex<HashMap<String, PendingDeletion>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

const DELETION_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes
