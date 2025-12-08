use std::sync::Arc;

use flow_like::hub::{MailConfig, MailProviderType};
use flow_like_types::Result;

#[cfg(feature = "sendgrid")]
mod sendgrid;
#[cfg(feature = "ses")]
mod ses;
#[cfg(feature = "smtp")]
mod smtp;
pub mod templates;

#[cfg(feature = "sendgrid")]
pub use sendgrid::SendgridMailClient;
#[cfg(feature = "ses")]
pub use ses::SesMailClient;
#[cfg(feature = "smtp")]
pub use smtp::SmtpMailClient;

#[derive(Clone)]
pub struct EmailMessage {
    pub to: String,
    pub subject: String,
    pub body_html: Option<String>,
    pub body_text: Option<String>,
}

#[async_trait::async_trait]
pub trait MailClient: Send + Sync {
    async fn send(&self, message: EmailMessage) -> Result<()>;
    fn from_email(&self) -> &str;
    fn from_name(&self) -> &str;
}

pub type DynMailClient = Arc<dyn MailClient>;

pub async fn create_mail_client(config: &MailConfig) -> Result<DynMailClient> {
    match config.provider {
        MailProviderType::Ses => {
            #[cfg(feature = "ses")]
            {
                let client = SesMailClient::new(config).await?;
                Ok(Arc::new(client))
            }
            #[cfg(not(feature = "ses"))]
            {
                Err(flow_like_types::anyhow!("SES feature not enabled"))
            }
        }
        MailProviderType::Smtp => {
            #[cfg(feature = "smtp")]
            {
                let smtp_settings = config.smtp.as_ref().ok_or_else(|| {
                    flow_like_types::anyhow!("SMTP settings required for SMTP provider")
                })?;
                let client = SmtpMailClient::new(config, smtp_settings)?;
                Ok(Arc::new(client))
            }
            #[cfg(not(feature = "smtp"))]
            {
                Err(flow_like_types::anyhow!("SMTP feature not enabled"))
            }
        }
        MailProviderType::Sendgrid => {
            #[cfg(feature = "sendgrid")]
            {
                let sendgrid_settings = config.sendgrid.as_ref().ok_or_else(|| {
                    flow_like_types::anyhow!("Sendgrid settings required for Sendgrid provider")
                })?;
                let client = SendgridMailClient::new(config, sendgrid_settings)?;
                Ok(Arc::new(client))
            }
            #[cfg(not(feature = "sendgrid"))]
            {
                Err(flow_like_types::anyhow!("Sendgrid feature not enabled"))
            }
        }
    }
}
