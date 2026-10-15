use std::collections::HashMap;

use flow_like::flow::oauth::{OAuthToken, refresh_on_hub};
use flow_like_types::{Result, anyhow};

/// Scheduled runs resolve saved grants just before execution, after profile selection.
pub(crate) async fn resolve_tokens(
    hub: &str,
    secure: bool,
    execution_token: Option<&str>,
    tokens: &HashMap<String, OAuthToken>,
) -> Result<(HashMap<String, OAuthToken>, Option<String>)> {
    let renewable: Vec<_> = tokens
        .values()
        .filter(|token| token.refresh_token.is_some())
        .collect();
    if renewable.is_empty() {
        anyhow::ensure!(
            tokens.values().all(|token| !token.is_expired()),
            "An OAuth access token expired; reconnect the account"
        );
        return Ok((tokens.clone(), None));
    }
    let origin = flow_like::hub::hub_origin(hub, secure)
        .ok_or_else(|| anyhow!("OAuth execution requires a hub"))?;
    for token in &renewable {
        token.ensure_hub(&origin, secure)?;
    }
    let authorization = match execution_token
        .filter(|value| value.trim_start_matches("Bearer ").starts_with("pat_"))
    {
        Some(pat) => pat.to_owned(),
        None => {
            let subject = renewable[0].hub_subject.as_deref().ok_or_else(|| {
                anyhow!("Reconnect this OAuth account to bind its saved grant to an account")
            })?;
            anyhow::ensure!(
                renewable
                    .iter()
                    .all(|token| token.hub_subject.as_deref() == Some(subject)),
                "Saved OAuth grants belong to different Hub accounts"
            );
            crate::execution_credentials::session_token(&origin, subject).ok_or_else(|| {
                anyhow!("Sign in to the OAuth grant's Hub account before running this event")
            })?
        }
    };
    let mut resolved = HashMap::with_capacity(tokens.len());
    for (provider, token) in tokens {
        let next = if token.refresh_token.is_some() {
            refresh_on_hub(&origin, secure, &authorization, provider, token, None).await?
        } else {
            anyhow::ensure!(
                !token.is_expired(),
                "An OAuth access token expired; reconnect the account"
            );
            token.clone()
        };
        resolved.insert(provider.clone(), next);
    }
    Ok((resolved, Some(authorization)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn events_without_oauth_still_run_without_a_hub() {
        let (tokens, authorization) = resolve_tokens("", true, None, &HashMap::new())
            .await
            .unwrap();
        assert!(tokens.is_empty());
        assert!(authorization.is_none());
    }

    #[tokio::test]
    async fn another_hub_is_rejected_before_a_saved_pat_can_be_sent() {
        let mut token = OAuthToken::new("access".into());
        token.refresh_token = Some("fl_oauth_saved".into());
        token.hub_origin = Some("https://original.example".into());
        token.hub_subject = Some("alice".into());
        let error = resolve_tokens(
            "http://127.0.0.1:1",
            false,
            Some("pat_secret"),
            &HashMap::from([("provider".into(), token)]),
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("another hub"));
    }

    #[tokio::test]
    async fn legacy_unbound_grants_require_reconnection_before_refresh() {
        let mut token = OAuthToken::new("access".into());
        token.refresh_token = Some("fl_oauth_saved".into());
        let error = resolve_tokens(
            "http://127.0.0.1:1",
            false,
            Some("pat_secret"),
            &HashMap::from([("provider".into(), token)]),
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("bind its saved grant"));
    }
}
