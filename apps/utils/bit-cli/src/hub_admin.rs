use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use flow_like::{
    bit::{Bit, Metadata},
    flow_like_types::reqwest::{Client, RequestBuilder, Response},
};
use futures::StreamExt;
use serde_json::Value;

pub struct Admin {
    base: String,
    token: String,
    client: Client,
}

impl Admin {
    pub fn new(hub_domain: &str, token: String) -> Result<Self> {
        let origin = hub_domain.trim().trim_end_matches('/');
        if origin.is_empty() {
            bail!("no hub to publish to, pass --hub <DOMAIN>");
        }
        let origin = if origin.contains("://") {
            origin.to_string()
        } else {
            format!("https://{origin}")
        };

        Ok(Self {
            base: format!("{origin}/api/v1"),
            token,
            client: Client::builder()
                // The hub mirrors the artifact before it answers, so the
                // request outlives an ordinary API call by hours.
                .timeout(Duration::from_secs(60 * 60 * 4))
                .connect_timeout(Duration::from_secs(30))
                .build()?,
        })
    }

    fn authorized(&self, builder: RequestBuilder) -> RequestBuilder {
        builder.bearer_auth(&self.token)
    }

    /// The admin upsert answers as an event stream: progress frames while the
    /// hub fetches and hashes the artifact, then the stored bit.
    pub async fn upsert_bit(
        &self,
        bit: &Value,
        mut on_progress: impl FnMut(&Value),
    ) -> Result<Bit> {
        let bit_id = bit
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("bit payload carries no id"))?;

        let response = self
            .authorized(self.client.put(format!("{}/admin/bit/{bit_id}", self.base)))
            .json(bit)
            .send()
            .await?;
        let response = success(response, &format!("upsert of bit {bit_id}")).await?;

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut stored = None;

        while let Some(chunk) = stream.next().await {
            buffer.push_str(&String::from_utf8_lossy(&chunk?));

            while let Some(index) = buffer.find("\n\n") {
                let frame: String = buffer.drain(..index + 2).collect();
                let Some((event, data)) = parse_frame(&frame) else {
                    continue;
                };

                match event.as_str() {
                    "progress" => on_progress(&data),
                    "done" => stored = Some(serde_json::from_value::<Bit>(data)?),
                    "error" => bail!(
                        "the hub rejected bit {bit_id}: {}",
                        data.get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("no reason given")
                    ),
                    _ => {}
                }
            }

            if stored.is_some() {
                break;
            }
        }

        stored
            .ok_or_else(|| anyhow!("the hub closed the stream for bit {bit_id} without storing it"))
    }

    /// The user the token acts as, together with the global permission bits
    /// that decide whether it may write to the catalog.
    pub async fn whoami(&self) -> Result<Value> {
        let response = self
            .authorized(self.client.get(format!("{}/user/info", self.base)))
            .send()
            .await?;
        let response = success(response, "identity check").await?;
        Ok(response.json().await?)
    }

    pub async fn push_meta(&self, bit_id: &str, language: &str, meta: &Metadata) -> Result<()> {
        let response = self
            .authorized(
                self.client
                    .put(format!("{}/admin/bit/{bit_id}/{language}", self.base)),
            )
            .json(meta)
            .send()
            .await?;
        success(
            response,
            &format!("metadata push for bit {bit_id} ({language})"),
        )
        .await?;
        Ok(())
    }

    pub async fn delete_bit(&self, bit_id: &str) -> Result<()> {
        let response = self
            .authorized(
                self.client
                    .delete(format!("{}/admin/bit/{bit_id}", self.base)),
            )
            .send()
            .await?;
        success(response, &format!("deletion of bit {bit_id}")).await?;
        Ok(())
    }
}

async fn success(response: Response, operation: &str) -> Result<Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    let body = response.text().await.unwrap_or_default();
    let hint = match status.as_u16() {
        401 => " — the token was not accepted",
        403 => " — the token's user lacks the WriteBits permission",
        _ => "",
    };
    bail!(
        "hub answered {status} to the {operation}{hint}: {}",
        body.trim()
    )
}

fn parse_frame(frame: &str) -> Option<(String, Value)> {
    let mut event = "message".to_string();
    let mut data = String::new();

    for line in frame.lines() {
        if let Some(value) = line.strip_prefix("event:") {
            event = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("data:") {
            data.push_str(value.trim_start());
        }
    }

    if data.is_empty() {
        return None;
    }

    Some((event, serde_json::from_str(&data).ok()?))
}
