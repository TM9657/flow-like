use flow_like_types::{Result, anyhow, bail, reqwest::Url};

/// Validates the destination before a media provider resolves Google credentials.
/// Only the global and regional Vertex API hosts may receive those credentials.
pub fn validate_vertex_endpoint(endpoint: &str) -> Result<String> {
    let url = Url::parse(endpoint).map_err(|_| anyhow!("Invalid Vertex endpoint URL"))?;
    let trusted_host = url.host_str().is_some_and(|host| {
        host == "aiplatform.googleapis.com"
            || host
                .strip_suffix("-aiplatform.googleapis.com")
                .is_some_and(|region| {
                    !region.is_empty()
                        && region.starts_with(|c: char| c.is_ascii_alphanumeric())
                        && region.ends_with(|c: char| c.is_ascii_alphanumeric())
                        && region
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                })
    });
    if url.scheme() != "https"
        || !trusted_host
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        bail!(
            "Vertex endpoint must use HTTPS on aiplatform.googleapis.com or a regional \
             <location>-aiplatform.googleapis.com host, with port 443 and no user info, query, or fragment"
        );
    }
    Ok(url.as_str().trim_end_matches('/').to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_global_and_regional_vertex_endpoints() {
        for endpoint in [
            "https://aiplatform.googleapis.com/v1",
            "https://aiplatform.googleapis.com/v1beta1",
            "https://us-central1-aiplatform.googleapis.com/v1",
            "https://europe-west4-aiplatform.googleapis.com/v1beta1",
            "https://asia-northeast1-aiplatform.googleapis.com/v1",
        ] {
            assert_eq!(validate_vertex_endpoint(endpoint).unwrap(), endpoint);
        }
        assert_eq!(
            validate_vertex_endpoint("https://US-CENTRAL1-AIPLATFORM.GOOGLEAPIS.COM:443/v1/")
                .unwrap(),
            "https://us-central1-aiplatform.googleapis.com/v1"
        );
    }

    #[test]
    fn refuses_untrusted_or_ambiguous_vertex_endpoints() {
        for endpoint in [
            "https://attacker.example/v1",
            "http://us-central1-aiplatform.googleapis.com/v1",
            "ftp://aiplatform.googleapis.com/v1",
            "//aiplatform.googleapis.com/v1",
            "https://127.0.0.1/v1",
            "https://[::1]/v1",
            "https://169.254.169.254/v1",
            "https://aiplatform.googleapis.com.attacker.example/v1",
            "https://us-central1-aiplatform.googleapis.com.attacker.example/v1",
            "https://attacker.example/aiplatform.googleapis.com/v1",
            "https://attacker.aiplatform.googleapis.com/v1",
            "https://attacker.us-central1-aiplatform.googleapis.com/v1",
            "https://not-aiplatform.googleapis.com.attacker.example/v1",
            "https://storage.googleapis.com/v1",
            "https://aiplatform.googleapis.com@attacker.example/v1",
            "https://user:secret@aiplatform.googleapis.com/v1",
            "https://user@aiplatform.googleapis.com/v1",
            "https://aiplatform.googleapis.com:8443/v1",
            "https://aiplatform.googleapis.com/v1?key=secret",
            "https://aiplatform.googleapis.com/v1#secret",
            "https://aiplatform.googleapis.com./v1",
            "https://-aiplatform.googleapis.com/v1",
            "https://-us-central1-aiplatform.googleapis.com/v1",
            "https://us-central1_-aiplatform.googleapis.com/v1",
            "not a URL",
        ] {
            let error = validate_vertex_endpoint(endpoint).unwrap_err();
            assert!(!error.to_string().contains("secret"));
        }
    }

    #[test]
    fn refuses_host_injection_in_default_vertex_endpoints() {
        for location in [
            "attacker.example/",
            "attacker.example?",
            "attacker.example#",
            "aiplatform.googleapis.com@attacker.example/",
            "attacker.example\\",
        ] {
            let endpoint = format!("https://{location}-aiplatform.googleapis.com/v1");
            assert!(validate_vertex_endpoint(&endpoint).is_err(), "{endpoint}");
        }
    }
}
