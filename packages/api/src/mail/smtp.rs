use flow_like::hub::{MailConfig, SmtpSettings};
use flow_like_types::Result;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{MultiPart, SinglePart, header::ContentType},
    transport::smtp::authentication::Credentials,
};

use super::{EmailMessage, MailClient};

pub struct SmtpMailClient {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from_email: String,
    from_name: String,
}

impl SmtpMailClient {
    pub fn new(config: &MailConfig, smtp: &SmtpSettings) -> Result<Self> {
        let host = std::env::var(&smtp.host_env)
            .map_err(|_| flow_like_types::anyhow!("SMTP host env var {} not set", smtp.host_env))?;
        let port: u16 = std::env::var(&smtp.port_env)
            .map_err(|_| flow_like_types::anyhow!("SMTP port env var {} not set", smtp.port_env))?
            .parse()
            .map_err(|_| flow_like_types::anyhow!("Invalid SMTP port"))?;
        let username = std::env::var(&smtp.username_env).map_err(|_| {
            flow_like_types::anyhow!("SMTP username env var {} not set", smtp.username_env)
        })?;
        let password = std::env::var(&smtp.password_env).map_err(|_| {
            flow_like_types::anyhow!("SMTP password env var {} not set", smtp.password_env)
        })?;

        let creds = Credentials::new(username, password);

        let transport = AsyncSmtpTransport::<Tokio1Executor>::relay(&host)
            .map_err(|e| flow_like_types::anyhow!("Failed to create SMTP transport: {}", e))?
            .port(port)
            .credentials(creds)
            .build();

        Ok(Self {
            transport,
            from_email: config.from_email.clone(),
            from_name: config.from_name.clone(),
        })
    }
}

#[async_trait::async_trait]
impl MailClient for SmtpMailClient {
    async fn send(&self, message: EmailMessage) -> Result<()> {
        let from_address = format!("{} <{}>", self.from_name, self.from_email);

        let email_builder = Message::builder()
            .from(
                from_address
                    .parse()
                    .map_err(|e| flow_like_types::anyhow!("Invalid from address: {}", e))?,
            )
            .to(message
                .to
                .parse()
                .map_err(|e| flow_like_types::anyhow!("Invalid to address: {}", e))?)
            .subject(&message.subject);

        let email = match (&message.body_html, &message.body_text) {
            (Some(html), Some(text)) => email_builder
                .multipart(
                    MultiPart::alternative()
                        .singlepart(
                            SinglePart::builder()
                                .header(ContentType::TEXT_PLAIN)
                                .body(text.clone()),
                        )
                        .singlepart(
                            SinglePart::builder()
                                .header(ContentType::TEXT_HTML)
                                .body(html.clone()),
                        ),
                )
                .map_err(|e| flow_like_types::anyhow!("Failed to build email: {}", e))?,
            (Some(html), None) => email_builder
                .header(ContentType::TEXT_HTML)
                .body(html.clone())
                .map_err(|e| flow_like_types::anyhow!("Failed to build email: {}", e))?,
            (None, Some(text)) => email_builder
                .header(ContentType::TEXT_PLAIN)
                .body(text.clone())
                .map_err(|e| flow_like_types::anyhow!("Failed to build email: {}", e))?,
            (None, None) => {
                return Err(flow_like_types::anyhow!(
                    "Email must have either HTML or text body"
                ));
            }
        };

        self.transport
            .send(email)
            .await
            .map_err(|e| flow_like_types::anyhow!("Failed to send email via SMTP: {}", e))?;

        Ok(())
    }

    fn from_email(&self) -> &str {
        &self.from_email
    }

    fn from_name(&self) -> &str {
        &self.from_name
    }
}
