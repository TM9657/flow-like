use std::collections::HashMap;

use anyhow::{Result, anyhow};
use flow_like::{
    bit::{Bit, BitTypes},
    hub::{BitSearchQuery, Hub},
};

/// The hub caps a single search page, so a full catalog listing is paged.
const PAGE_SIZE: u64 = 100;
const MAX_PAGES: u64 = 100;

pub fn parse_bit_type(input: &str) -> Result<BitTypes> {
    let variants = bit_type_variants();
    let wanted = normalize(input);
    let matched = variants
        .iter()
        .find(|variant| normalize(variant) == wanted)
        .ok_or_else(|| {
            anyhow!(
                "unknown bit type {input}, expected one of {}",
                variants.join(", ")
            )
        })?;

    serde_json::from_value(matched.as_str().into())
        .map_err(|err| anyhow!("could not read bit type {matched}: {err}"))
}

/// The accepted values come from the schema the type itself derives, so a new
/// variant never has to be mirrored here.
fn bit_type_variants() -> Vec<String> {
    schemars::schema_for!(BitTypes)
        .as_value()
        .get("enum")
        .and_then(serde_json::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn normalize(input: &str) -> String {
    input
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect()
}

pub fn bit_type_name(bit: &Bit) -> String {
    serde_json::to_value(&bit.bit_type)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "Other".to_string())
}

pub fn bit_name(bit: &Bit) -> String {
    bit.meta
        .get("en")
        .or_else(|| bit.meta.values().next())
        .map(|meta| meta.name.clone())
        .unwrap_or_else(|| bit.id.clone())
}

pub async fn search(hub: &Hub, query: &BitSearchQuery) -> Result<Vec<Bit>> {
    hub.search_bit(query)
        .await
        .map_err(|err| anyhow!("catalog search failed: {err}"))
}

/// Every catalog bit keyed by artifact hash, so store directories can be named.
pub async fn index_by_hash(hub: &Hub) -> Result<HashMap<String, Bit>> {
    let mut index = HashMap::new();

    for page in 0..MAX_PAGES {
        let query = BitSearchQuery::builder()
            .with_limit(PAGE_SIZE)
            .with_offset(page * PAGE_SIZE);
        let bits = search(hub, &query).await?;
        let received = bits.len() as u64;

        for bit in bits {
            index.entry(bit.hash.clone()).or_insert(bit);
        }

        if received < PAGE_SIZE {
            break;
        }
    }

    Ok(index)
}

pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}
