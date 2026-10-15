use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Token passed from the frontend after OAuth authentication
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct OAuthToken {
    /// The access token for API calls
    pub access_token: String,
    /// Optional refresh token for obtaining new access tokens
    pub refresh_token: Option<String>,
    /// Unix timestamp when the access token expires
    pub expires_at: Option<u64>,
    /// The token type (usually "Bearer")
    pub token_type: Option<String>,
    /// Hub and account that own the canonical refresh grant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hub_origin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hub_subject: Option<String>,
}

impl OAuthToken {
    pub fn new(access_token: String) -> Self {
        OAuthToken {
            access_token,
            refresh_token: None,
            expires_at: None,
            token_type: Some("Bearer".to_string()),
            hub_origin: None,
            hub_subject: None,
        }
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            // Consider expired if less than 60 seconds remaining
            expires_at <= now + 60
        } else {
            false
        }
    }

    pub fn bearer_header(&self) -> String {
        format!(
            "{} {}",
            self.token_type.as_deref().unwrap_or("Bearer"),
            self.access_token
        )
    }

    #[cfg(feature = "hub")]
    pub fn ensure_hub(&self, hub: &str, secure: bool) -> flow_like_types::Result<()> {
        let origin = self.hub_origin.as_deref().ok_or_else(|| {
            flow_like_types::anyhow!(
                "Reconnect this OAuth account to bind its saved grant to a hub"
            )
        })?;
        if crate::hub::hub_api_url(origin, true, &[])? != crate::hub::hub_api_url(hub, secure, &[])?
        {
            flow_like_types::bail!("The OAuth grant belongs to another hub");
        }
        Ok(())
    }
}

/// Resolve a grant through its Hub so every resource and execution shares its rotation owner.
#[cfg(feature = "hub")]
pub async fn refresh_on_hub(
    hub: &str,
    secure: bool,
    authorization: &str,
    provider: &str,
    token: &OAuthToken,
    scope: Option<&str>,
) -> flow_like_types::Result<OAuthToken> {
    use flow_like_types::{bail, reqwest};
    let refresh = token.refresh_token.as_deref().ok_or_else(|| {
        flow_like_types::anyhow!("Reconnect this OAuth account to renew its access token")
    })?;
    if authorization.is_empty() {
        bail!("OAuth renewal requires Hub authentication");
    }
    let url = crate::hub::hub_api_url(hub, secure, &["oauth", "refresh", provider])?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let response = client
        .post(url)
        .bearer_auth(
            authorization
                .strip_prefix("Bearer ")
                .unwrap_or(authorization),
        )
        .json(&serde_json::json!({ "refresh_token": refresh, "scope": scope }))
        .send()
        .await?;
    if !response.status().is_success() {
        bail!("OAuth renewal was refused ({})", response.status());
    }
    #[derive(Deserialize)]
    struct Response {
        access_token: String,
        refresh_token: Option<String>,
        expires_in: Option<u64>,
        token_type: Option<String>,
    }
    let response: Response = response.json().await?;
    if response.access_token.is_empty() {
        bail!("OAuth renewal returned no access token");
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    Ok(OAuthToken {
        access_token: response.access_token,
        refresh_token: response
            .refresh_token
            .or_else(|| token.refresh_token.clone()),
        expires_at: response
            .expires_in
            .map(|duration| now.saturating_add(duration)),
        token_type: response.token_type.or_else(|| token.token_type.clone()),
        hub_origin: token.hub_origin.clone(),
        hub_subject: token.hub_subject.clone(),
    })
}

/// Collection of OAuth tokens keyed by provider ID
pub type OAuthTokens = HashMap<String, OAuthToken>;

#[cfg(all(test, feature = "hub"))]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn legacy_tokens_deserialize_but_unbound_grants_cannot_cross_hubs() {
        let mut token: OAuthToken =
            serde_json::from_str(r#"{"access_token":"access","refresh_token":"fl_oauth_saved"}"#)
                .unwrap();
        assert!(token.ensure_hub("https://first.example", true).is_err());
        token.hub_origin = Some("https://first.example".into());
        assert!(token.ensure_hub("first.example", true).is_ok());
        assert!(
            token
                .ensure_hub("https://first.example/api/v1", true)
                .is_ok()
        );
        assert!(token.ensure_hub("first.example", false).is_err());
        assert!(token.ensure_hub("https://second.example", true).is_err());
    }

    fn mock_hub(
        status: &str,
        body: &str,
        headers: &str,
    ) -> (String, std::thread::JoinHandle<String>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}",
            body.len()
        );
        let task = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0u8; 4096];
            loop {
                let read = socket.read(&mut buffer).unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
                let text = String::from_utf8_lossy(&request);
                if let Some((headers, body)) = text.split_once("\r\n\r\n") {
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_lowercase()
                                .strip_prefix("content-length:")
                                .map(|value| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if body.len() >= length {
                        break;
                    }
                }
            }
            socket.write_all(response.as_bytes()).unwrap();
            String::from_utf8(request).unwrap()
        });
        (format!("http://{address}"), task)
    }

    #[tokio::test]
    async fn scoped_refresh_sends_the_canonical_handle_only_to_the_hub() {
        let (hub, request) = mock_hub(
            "200 OK",
            r#"{"access_token":"resource-access","expires_in":3600,"token_type":"Bearer"}"#,
            "",
        );
        let mut token = OAuthToken::new("old-access".into());
        token.refresh_token = Some("fl_oauth_saved".into());
        token.hub_origin = Some(hub.clone());
        token.hub_subject = Some("alice".into());
        let refreshed = refresh_on_hub(
            &hub,
            false,
            "pat_owner",
            "microsoft_dataverse",
            &token,
            Some("https://org.crm.dynamics.com/user_impersonation"),
        )
        .await
        .unwrap();
        assert_eq!(refreshed.access_token, "resource-access");
        assert_eq!(refreshed.refresh_token, token.refresh_token);
        assert_eq!(refreshed.hub_subject.as_deref(), Some("alice"));
        assert!(!refreshed.is_expired());
        let request = request.join().unwrap();
        assert!(request.starts_with("POST /api/v1/oauth/refresh/microsoft_dataverse "));
        assert!(
            request
                .to_lowercase()
                .contains("authorization: bearer pat_owner\r\n")
        );
        let body: serde_json::Value =
            serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(body["refresh_token"], "fl_oauth_saved");
        assert_eq!(
            body["scope"],
            "https://org.crm.dynamics.com/user_impersonation"
        );
    }

    #[tokio::test]
    async fn refresh_refuses_redirects_without_forwarding_credentials() {
        let (hub, request) = mock_hub(
            "302 Found",
            "",
            "Location: http://127.0.0.1:1/provider-token\r\n",
        );
        let mut token = OAuthToken::new("old-access".into());
        token.refresh_token = Some("fl_oauth_saved".into());
        let error = refresh_on_hub(&hub, false, "pat_owner", "provider", &token, None)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("302"));
        request.join().unwrap();
    }
}
