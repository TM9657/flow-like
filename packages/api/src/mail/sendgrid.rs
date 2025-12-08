use flow_like::hub::{MailConfig, SendgridSettings};
use flow_like_types::Result;
use reqwest::Client;
use serde::Serialize;

use super::{EmailMessage, MailClient};

pub struct SendgridMailClient {
    client: Client,
    api_key: String,
    from_email: String,
    from_name: String,
}

#[derive(Serialize)]
struct SendgridEmail {
    personalizations: Vec<Personalization>,
    from: EmailAddress,
    subject: String,
    content: Vec<Content>,
}

#[derive(Serialize)]
struct Personalization {
    to: Vec<EmailAddress>,
}

#[derive(Serialize)]
struct EmailAddress {
    email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Serialize)]
struct Content {
    #[serde(rename = "type")]
    content_type: String,
    value: String,
}

impl SendgridMailClient {
    pub fn new(config: &MailConfig, sendgrid: &SendgridSettings) -> Result<Self> {
        let api_key = std::env::var(&sendgrid.api_key_env).map_err(|_| {
            flow_like_types::anyhow!("Sendgrid API key env var {} not set", sendgrid.api_key_env)
        })?;

        Ok(Self {
            client: Client::new(),
            api_key,
            from_email: config.from_email.clone(),
            from_name: config.from_name.clone(),
        })
    }
}

#[async_trait::async_trait]
impl MailClient for SendgridMailClient {
    async fn send(&self, message: EmailMessage) -> Result<()> {
        let mut content = Vec::new();

        if let Some(text) = &message.body_text {
            content.push(Content {
                content_type: "text/plain".to_string(),
                value: text.clone(),
            });
        }

        if let Some(html) = &message.body_html {
            content.push(Content {
                content_type: "text/html".to_string(),
                value: html.clone(),
            });
        }

        if content.is_empty() {
            return Err(flow_like_types::anyhow!(
                "Email must have either HTML or text body"
            ));
        }

        let email = SendgridEmail {
            personalizations: vec![Personalization {
                to: vec![EmailAddress {
                    email: message.to.clone(),
                    name: None,
                }],
            }],
            from: EmailAddress {
                email: self.from_email.clone(),
                name: Some(self.from_name.clone()),
            },
            subject: message.subject.clone(),
            content,
        };

        let response = self
            .client
            .post("https://api.sendgrid.com/v3/mail/send")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&email)
            .send()
            .await
            .map_err(|e| flow_like_types::anyhow!("Failed to send email via Sendgrid: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(flow_like_types::anyhow!(
                "Sendgrid API error: {} - {}",
                status,
                body
            ));
        }

        Ok(())
    }

    fn from_email(&self) -> &str {
        &self.from_email
    }

    fn from_name(&self) -> &str {
        &self.from_name
    }
}
