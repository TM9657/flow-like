use std::collections::{BTreeMap, HashMap, HashSet};

use anyhow::{Result, anyhow, bail};
use flow_like::{
    bit::{Bit, Metadata},
    flow_like_types::create_id,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// One file describes a bit and everything it depends on. Entries carry the
/// fields of a `Bit` plus two authoring conveniences: `ref`, a local alias
/// other entries point at as `@alias`, and `meta`, the per-language metadata
/// the hub stores through its own endpoint.
#[derive(Deserialize, Serialize)]
pub struct Spec {
    pub bits: Vec<Value>,
}

pub struct Planned {
    pub id: String,
    pub alias: Option<String>,
    pub bit_type: String,
    pub payload: Value,
    pub dependencies: Vec<Dependency>,
    pub meta: BTreeMap<String, Metadata>,
}

/// A dependency is stored as `hub:id`, and the hub half is only known once the
/// dependency itself has been written, so the reference is resolved at push
/// time rather than while planning.
pub enum Dependency {
    Local(String),
    Remote(String),
}

impl Dependency {
    pub fn describe(&self) -> String {
        match self {
            Dependency::Local(id) => id.clone(),
            Dependency::Remote(reference) => reference.clone(),
        }
    }
}

pub fn load(path: &std::path::Path) -> Result<Spec> {
    let raw = std::fs::read_to_string(path)
        .map_err(|err| anyhow!("could not read the spec at {}: {err}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|err| anyhow!("could not parse the spec at {}: {err}", path.display()))
}

/// Resolve aliases, fill in ids and order the entries so a dependency is
/// always stored before the bit that names it.
pub fn plan(spec: &Spec, hub: &str) -> Result<Vec<Planned>> {
    let entries = spec
        .bits
        .iter()
        .map(|entry| {
            entry
                .as_object()
                .cloned()
                .ok_or_else(|| anyhow!("every entry under \"bits\" must be an object"))
        })
        .collect::<Result<Vec<_>>>()?;

    let mut aliases: HashMap<String, String> = HashMap::new();
    let mut ids = Vec::new();
    for entry in &entries {
        let id = match entry.get("id").and_then(Value::as_str) {
            Some(id) if !id.trim().is_empty() => id.trim().to_string(),
            _ => create_id(),
        };
        if let Some(alias) = entry.get("ref").and_then(Value::as_str)
            && aliases.insert(alias.to_string(), id.clone()).is_some()
        {
            bail!("the alias @{alias} is used by more than one entry");
        }
        ids.push(id);
    }

    let allowed = bit_fields();
    let mut planned = Vec::new();
    for (entry, id) in entries.iter().zip(&ids) {
        let alias = entry.get("ref").and_then(Value::as_str).map(str::to_string);

        let mut payload = entry.clone();
        let meta = take_meta(&mut payload)?;
        payload.remove("ref");
        payload.insert("id".to_string(), Value::String(id.clone()));

        for key in payload.keys() {
            if !allowed.contains(key.as_str()) {
                bail!("entry {id} carries the unknown field \"{key}\"");
            }
        }

        let dependencies = resolve_dependencies(&payload, &aliases, id)?;
        payload.remove("dependencies");

        if payload
            .get("hub")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            payload.insert("hub".to_string(), Value::String(hub.to_string()));
        }

        let payload = merge(serde_json::to_value(Bit::default())?, &payload);
        let bit: Bit = serde_json::from_value(payload.clone())
            .map_err(|err| anyhow!("entry {id} is not a valid bit: {err}"))?;

        planned.push(Planned {
            id: id.clone(),
            alias,
            bit_type: serde_json::to_value(&bit.bit_type)?
                .as_str()
                .unwrap_or("Other")
                .to_string(),
            payload,
            dependencies,
            meta,
        });
    }

    order(planned)
}

/// Build a spec out of bits the hub already stores, so an existing entry is
/// the starting point for the next one.
pub fn from_bits(bits: &[Bit]) -> Spec {
    let known: HashSet<&str> = bits.iter().map(|bit| bit.id.as_str()).collect();
    let aliases: HashMap<&str, String> = bits
        .iter()
        .enumerate()
        .map(|(index, bit)| (bit.id.as_str(), alias_for(bit, index)))
        .collect();

    let entries = bits
        .iter()
        .map(|bit| {
            let mut entry = Map::new();
            entry.insert(
                "ref".to_string(),
                Value::String(aliases[bit.id.as_str()].clone()),
            );

            let mut body = serde_json::to_value(bit)
                .ok()
                .and_then(|value| value.as_object().cloned())
                .unwrap_or_default();
            body.remove("meta");
            for computed in [
                "hash",
                "dependency_tree_hash",
                "created",
                "updated",
                "model_evaluation",
            ] {
                body.remove(computed);
            }
            if let Some(Value::Array(dependencies)) = body.get_mut("dependencies") {
                for dependency in dependencies.iter_mut() {
                    let Some(reference) = dependency.as_str() else {
                        continue;
                    };
                    let id = reference.split_once(':').map_or(reference, |(_, id)| id);
                    if known.contains(id) {
                        *dependency = Value::String(format!("@{}", aliases[id]));
                    }
                }
            }
            entry.extend(body);

            let meta: BTreeMap<_, _> = bit.meta.iter().collect();
            if !meta.is_empty()
                && let Ok(meta) = serde_json::to_value(meta)
            {
                entry.insert("meta".to_string(), meta);
            }

            Value::Object(entry)
        })
        .collect();

    Spec { bits: entries }
}

fn alias_for(bit: &Bit, index: usize) -> String {
    let base = serde_json::to_value(&bit.bit_type)
        .ok()
        .and_then(|value| value.as_str().map(str::to_lowercase))
        .unwrap_or_else(|| "bit".to_string());
    format!("{base}-{index}")
}

fn take_meta(entry: &mut Map<String, Value>) -> Result<BTreeMap<String, Metadata>> {
    let Some(meta) = entry.remove("meta") else {
        return Ok(BTreeMap::new());
    };
    let Value::Object(languages) = meta else {
        bail!("\"meta\" must map a language code to its metadata");
    };

    let base = serde_json::to_value(Metadata::default())?;
    languages
        .into_iter()
        .map(|(language, value)| {
            let Value::Object(fields) = value else {
                bail!("the metadata for {language} must be an object");
            };
            let merged = merge(base.clone(), &fields);
            let metadata: Metadata = serde_json::from_value(merged)
                .map_err(|err| anyhow!("the metadata for {language} is invalid: {err}"))?;
            Ok((language, metadata))
        })
        .collect()
}

fn resolve_dependencies(
    entry: &Map<String, Value>,
    aliases: &HashMap<String, String>,
    id: &str,
) -> Result<Vec<Dependency>> {
    let Some(dependencies) = entry.get("dependencies") else {
        return Ok(Vec::new());
    };
    let Value::Array(dependencies) = dependencies else {
        bail!("the dependencies of {id} must be a list");
    };

    dependencies
        .iter()
        .map(|dependency| {
            let Some(dependency) = dependency.as_str() else {
                bail!("every dependency of {id} must be a bit id or an @alias");
            };
            match dependency.strip_prefix('@') {
                Some(alias) => aliases
                    .get(alias)
                    .map(|id| Dependency::Local(id.clone()))
                    .ok_or_else(|| {
                        anyhow!("{id} depends on @{alias}, which the spec never defines")
                    }),
                None => Ok(Dependency::Remote(dependency.to_string())),
            }
        })
        .collect()
}

fn order(planned: Vec<Planned>) -> Result<Vec<Planned>> {
    let mut pending = planned;
    let mut stored: HashSet<String> = HashSet::new();
    let mut ordered = Vec::new();

    while !pending.is_empty() {
        let (ready, waiting): (Vec<_>, Vec<_>) = pending.into_iter().partition(|entry| {
            entry
                .dependencies
                .iter()
                .all(|dependency| match dependency {
                    Dependency::Local(id) => stored.contains(id),
                    Dependency::Remote(_) => true,
                })
        });

        if ready.is_empty() {
            bail!(
                "the spec has a dependency cycle between {}",
                waiting
                    .iter()
                    .map(|entry| entry.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        stored.extend(ready.iter().map(|entry| entry.id.clone()));
        ordered.extend(ready);
        pending = waiting;
    }

    Ok(ordered)
}

fn merge(base: Value, overlay: &Map<String, Value>) -> Value {
    let mut merged = base.as_object().cloned().unwrap_or_default();
    for (key, value) in overlay {
        merged.insert(key.clone(), value.clone());
    }
    Value::Object(merged)
}

fn bit_fields() -> HashSet<&'static str> {
    static FIELDS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    FIELDS
        .get_or_init(|| {
            serde_json::to_value(Bit::default())
                .ok()
                .and_then(|value| {
                    value
                        .as_object()
                        .map(|fields| fields.keys().cloned().collect())
                })
                .unwrap_or_default()
        })
        .iter()
        .map(String::as_str)
        .collect()
}
