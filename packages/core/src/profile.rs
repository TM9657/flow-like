use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::{
    bit::{Bit, BitModelPreference, BitTypes},
    hub::Hub,
    utils::http::HTTPClient,
};
use anyhow::Result;
use futures::future::join_all;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::task;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Hash, PartialEq, Eq)]
pub struct Profile {
    #[serde(default = "cuid2::create_id")]
    pub id: String,
    pub name: String,
    pub description: String,
    pub thumbnail: String,
    #[serde(default)]
    pub hub: String,
    #[serde(default)]
    pub hubs: Vec<String>,
    pub bits: Vec<(String, String)>,
    pub updated: String,
    pub created: String,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            id: cuid2::create_id(),
            name: "".to_string(),
            description: "".to_string(),
            thumbnail: "".to_string(),
            hub: "".to_string(),
            hubs: vec![],
            bits: vec![],
            updated: "".to_string(),
            created: "".to_string(),
        }
    }
}

impl Profile {
    /// Gets the best model based on the preference
    /// For remote we are also looking on hubs for available models (for recommendations, for example)
    pub async fn get_best_model(
        &self,
        preference: &BitModelPreference,
        multimodal: bool,
        remote: bool,
        http_client: Arc<HTTPClient>,
    ) -> Result<Bit> {
        let mut best_bit = (0.0, None);

        if !remote {
            for (hub, bit_id) in &self.bits {
                let hub = Hub::new(hub, http_client.clone()).await?;
                let bit = hub.get_bit_by_id(bit_id).await?;
                if multimodal && !bit.is_multimodal() {
                    continue;
                }
                if let Ok(score) = bit.score(preference) {
                    println!("Score: {} for {}", score, bit.meta.get("en").unwrap().name);
                    if best_bit.1.is_none() || (score > best_bit.0) {
                        best_bit = (score, Some(bit.clone()));
                    }
                }
            }

            if let Some(bit) = best_bit.1 {
                return Ok(bit);
            }
        }

        let preference = preference.parse();
        let available_hubs = self.get_available_hubs(http_client).await?;
        let mut bits: HashMap<String, Bit> = HashMap::new();
        for hub in available_hubs {
            match hub.get_bits_of_type(&BitTypes::Vlm).await {
                Ok(models) => {
                    bits.extend(models.into_iter().map(|bit| (bit.id.clone(), bit.clone())));
                }
                Err(_) => {
                    continue;
                }
            };

            match hub.get_bits_of_type(&BitTypes::Llm).await {
                Ok(models) => {
                    bits.extend(models.into_iter().map(|bit| (bit.id.clone(), bit.clone())));
                }
                Err(_) => {
                    continue;
                }
            };
        }

        for (_, bit) in bits {
            if multimodal && !bit.is_multimodal() {
                continue;
            }

            if let Ok(score) = bit.score(&preference) {
                if best_bit.1.is_none() || score > best_bit.0 {
                    best_bit = (score, Some(bit.clone()));
                }
            }
        }

        match best_bit.1 {
            Some(bit) => Ok(bit),
            None => Err(anyhow::anyhow!("No Model found")),
        }
    }

    pub async fn get_available_bits_of_type(
        &self,
        bit_type: &BitTypes,
        http_client: Arc<HTTPClient>,
    ) -> Result<Vec<Bit>> {
        let hubs = self.get_available_hubs(http_client).await?;
        let mut bits: HashMap<String, Bit> = HashMap::new();
        for hub in hubs {
            let hub_bits = hub.get_bits_of_type(bit_type).await;
            let hub_bits = match hub_bits {
                Ok(models) => models,
                Err(err) => {
                    println!("Models not found: {}", err);
                    continue;
                }
            };
            for bit in hub_bits {
                if !bits.contains_key(&bit.id) {
                    bits.insert(bit.id.clone(), bit.clone());
                }
            }
        }
        let bits = bits.into_values().collect();
        Ok(bits)
    }

    pub async fn get_available_bits(&self, http_client: Arc<HTTPClient>) -> Result<Vec<Bit>> {
        let hubs = self.get_available_hubs(http_client).await?;
        let mut bits: HashMap<String, Bit> = HashMap::new();
        for hub in hubs {
            let hub_bits = hub.get_bits().await;
            let hub_bits = match hub_bits {
                Ok(models) => models,
                Err(err) => {
                    println!("Models not found: {}", err);
                    vec![]
                }
            };
            for bit in hub_bits {
                if !bits.contains_key(&bit.id) {
                    bits.insert(bit.id.clone(), bit.clone());
                }
            }
        }

        let bits = bits.into_values().collect();
        Ok(bits)
    }

    pub async fn get_bit(
        &self,
        bit: (String, String),
        http_client: Arc<HTTPClient>,
    ) -> Result<Bit> {
        let (hub_id, bit_id) = bit;
        let hub = Hub::new(&hub_id, http_client).await?;
        hub.get_bit_by_id(&bit_id).await
    }

    pub async fn find_bit(
        &self,
        bit_id: &str,
        http_client: Arc<HTTPClient>,
    ) -> Result<Bit> {
        let hubs = self.get_available_hubs(http_client).await?;
        for hub in hubs {
            let bit = hub.get_bit_by_id(bit_id).await;
            if let Ok(bit) = bit {
                return Ok(bit);
            }
        }
        Err(anyhow::anyhow!("Bit not found"))
    }

    pub async fn get_available_hubs(&self, http_client: Arc<HTTPClient>) -> Result<Vec<Hub>> {
        let mut hubs = HashSet::new();
        for hub in &self.hubs {
            hubs.insert(hub.clone());
        }

        self.bits.iter().for_each(|(hub, _)| {
            hubs.insert(hub.clone());
        });

        let hub_futures: Vec<_> = hubs
            .iter()
            .map(|hub| {
                let hub = hub.clone();
                let http_client = http_client.clone();
                task::spawn(async move { Hub::new(&hub, http_client).await })
            })
            .collect();

        let results = join_all(hub_futures).await;
        let built_hubs = results
            .into_iter()
            .filter_map(|f| f.ok())
            .flatten()
            .collect();

        Ok(built_hubs)
    }

    pub async fn add_bit(&mut self, bit: &Bit) {
        let bit_exists = self.bits.iter().any(|(_, bit_id)| bit_id == &bit.id);
        if bit_exists {
            return;
        }
        self.bits.push((bit.hub.clone(), bit.id.clone()));
    }

    pub fn remove_bit(&mut self, bit: &Bit) {
        self.bits.retain(|(_, bit_id)| bit_id != &bit.id);
    }
}
