use std::env;
use std::time::Duration;

use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use reqwest::blocking::Client;

#[derive(Debug, Clone, PartialEq)]
pub struct DeliveryOptions {
    pub send_email: bool,
    pub send_pushover: bool,
    pub subject: String,
}

impl Default for DeliveryOptions {
    fn default() -> Self {
        Self {
            send_email: false,
            send_pushover: false,
            subject: "Daily betting recommendation".to_string(),
        }
    }
}

pub fn deliver_report(body: &str, options: &DeliveryOptions) -> Result<Vec<String>, String> {
    let mut delivered = Vec::new();

    if options.send_email {
        send_email(&options.subject, body)?;
        delivered.push("email".to_string());
    }

    if options.send_pushover {
        send_pushover(&options.subject, body)?;
        delivered.push("pushover".to_string());
    }

    Ok(delivered)
}

fn send_email(subject: &str, body: &str) -> Result<(), String> {
    let host = required_env("BETTING_SMTP_HOST")?;
    let port = optional_env("BETTING_SMTP_PORT")
        .map(|raw| {
            raw.parse::<u16>()
                .map_err(|_| format!("BETTING_SMTP_PORT must be a number, got {raw}"))
        })
        .transpose()?
        .unwrap_or(587);
    let username = required_env("BETTING_SMTP_USERNAME")?;
    let password = required_env("BETTING_SMTP_PASSWORD")?;
    let from = required_env("BETTING_EMAIL_FROM")?;
    let to = required_env("BETTING_EMAIL_TO")?;

    let email = Message::builder()
        .from(
            from.parse()
                .map_err(|error| format!("BETTING_EMAIL_FROM parse failed: {error}"))?,
        )
        .to(to
            .parse()
            .map_err(|error| format!("BETTING_EMAIL_TO parse failed: {error}"))?)
        .subject(subject)
        .body(body.to_string())
        .map_err(|error| format!("email build failed: {error}"))?;

    let credentials = Credentials::new(username, password);
    let transport = SmtpTransport::relay(&host)
        .map_err(|error| format!("SMTP relay setup failed: {error}"))?
        .port(port)
        .credentials(credentials)
        .build();

    transport
        .send(&email)
        .map_err(|error| format!("SMTP send failed: {error}"))?;

    Ok(())
}

fn send_pushover(subject: &str, body: &str) -> Result<(), String> {
    let token = required_env("BETTING_PUSHOVER_TOKEN")?;
    let user = required_env("BETTING_PUSHOVER_USER")?;
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("betting-daily-agent/0.1 by local-user")
        .build()
        .map_err(|error| format!("failed to build Pushover client: {error}"))?;
    let chunks = split_for_pushover(body);

    for (index, message) in chunks.iter().enumerate() {
        let title = if chunks.len() == 1 {
            subject.to_string()
        } else {
            format!("{subject} ({}/{})", index + 1, chunks.len())
        };
        let response = client
            .post("https://api.pushover.net/1/messages.json")
            .form(&[
                ("token", token.as_str()),
                ("user", user.as_str()),
                ("title", title.as_str()),
                ("message", message.as_str()),
            ])
            .send()
            .map_err(|error| format!("Pushover request failed: {error}"))?;

        if !response.status().is_success() {
            return Err(format!("Pushover returned {}", response.status()));
        }
    }

    Ok(())
}

fn split_for_pushover(body: &str) -> Vec<String> {
    const LIMIT: usize = 900;
    if body.len() <= LIMIT {
        return vec![body.to_string()];
    }

    let mut chunks = Vec::new();
    let mut current = String::new();
    for line in body.lines() {
        let projected_len = current.len() + line.len() + 1;
        if projected_len > LIMIT && !current.is_empty() {
            chunks.push(current.trim_end().to_string());
            current.clear();
        }

        if line.len() > LIMIT {
            for piece in line.as_bytes().chunks(LIMIT) {
                if !current.is_empty() {
                    chunks.push(current.trim_end().to_string());
                    current.clear();
                }
                current.push_str(&String::from_utf8_lossy(piece));
            }
        } else {
            current.push_str(line);
            current.push('\n');
        }
    }

    if !current.trim().is_empty() {
        chunks.push(current.trim_end().to_string());
    }

    chunks
}

fn required_env(name: &str) -> Result<String, String> {
    env::var(name)
        .map_err(|_| format!("{name} is required"))
        .and_then(|value| {
            if value.trim().is_empty() {
                Err(format!("{name} is required"))
            } else {
                Ok(value)
            }
        })
}

fn optional_env(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}
