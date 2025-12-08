use flow_like::hub::MailConfig;
use flow_like_types::Result;

use super::{EmailMessage, MailClient};

pub struct SesMailClient {
    client: aws_sdk_sesv2::Client,
    from_email: String,
    from_name: String,
}

impl SesMailClient {
    pub async fn new(config: &MailConfig) -> Result<Self> {
        let aws_config = aws_config::load_from_env().await;
        let client = aws_sdk_sesv2::Client::new(&aws_config);

        Ok(Self {
            client,
            from_email: config.from_email.clone(),
            from_name: config.from_name.clone(),
        })
    }
}

#[async_trait::async_trait]
impl MailClient for SesMailClient {
    async fn send(&self, message: EmailMessage) -> Result<()> {
        use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message};

        let from_address = format!("{} <{}>", self.from_name, self.from_email);

        let mut body_builder = Body::builder();

        if let Some(html) = &message.body_html {
            body_builder =
                body_builder.html(Content::builder().data(html).charset("UTF-8").build()?);
        }

        if let Some(text) = &message.body_text {
            body_builder =
                body_builder.text(Content::builder().data(text).charset("UTF-8").build()?);
        }

        let email_content = EmailContent::builder()
            .simple(
                Message::builder()
                    .subject(
                        Content::builder()
                            .data(&message.subject)
                            .charset("UTF-8")
                            .build()?,
                    )
                    .body(body_builder.build())
                    .build(),
            )
            .build();

        let destination = Destination::builder().to_addresses(&message.to).build();

        self.client
            .send_email()
            .from_email_address(from_address)
            .destination(destination)
            .content(email_content)
            .send()
            .await
            .map_err(|e| flow_like_types::anyhow!("Failed to send email via SES: {}", e))?;

        Ok(())
    }

    fn from_email(&self) -> &str {
        &self.from_email
    }

    fn from_name(&self) -> &str {
        &self.from_name
    }
}
