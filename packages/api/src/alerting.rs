use crate::{
    mail::EmailMessage,
    state::State,
};
use flow_like_types::{Result, bail};

#[tracing::instrument(name = "Send alert email", skip(state, subject, body_text))]
pub async fn send_alert_email(
    state: &State,
    subject: impl Into<String>,
    body_text: impl Into<String>,
) -> Result<()> {
    let Some(alerting) = &state.platform_config.alerting else {
        bail!("Alerting not configured")
    };

    let Some(mail_client) = &state.mail_client else {
        bail!("Mail client not configured")
    };

    let email = EmailMessage {
        to: alerting.mail.clone(),
        subject: subject.into(),
        body_html: None,
        body_text: Some(body_text.into()),
    };

    mail_client.send(email).await
}
