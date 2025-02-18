use crate::state::FlowLikeState;
use crate::utils::compression::{compress_to_file, from_compressed};
use crate::utils::download::download_bit;
use crate::utils::local_object_store::LocalObjectStore;
use futures::future::BoxFuture;
use futures::FutureExt;
use object_store::path::Path;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct BitMeta {
    pub name: String,
    pub description: String,
    pub long_description: String,
    pub tags: Vec<String>,
    pub use_case: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
pub enum BitTypes {
    Llm,
    Vlm,
    Embedding,
    ImageEmbedding,
    File,
    Media,
    Template,
    Tokenizer,
    TokenizerConfig,
    SpecialTokensMap,
    Config,
    Course,
    PreprocessorConfig,
    Projection,
    Project,
    Board,
    Other,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
pub enum BitProvider {
    Local,
    AzureOpenAI,
    Bedrock,
    OpenAI,
    Anthropic,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
pub struct BitProviderModel {
    pub provider_name: BitProvider,
    pub model_id: Option<String>,
    pub version: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, Default)]
pub struct BitModelPreference {
    pub cost_weight: Option<f32>,
    pub speed_weight: Option<f32>,
    pub reasoning_weight: Option<f32>,
    pub creativity_weight: Option<f32>,
    pub factfulness_weight: Option<f32>,
    pub function_calling_weight: Option<f32>,
    pub safety_weight: Option<f32>,
    pub openness_weight: Option<f32>,
    pub multilinguality_weight: Option<f32>,
    pub coding_weight: Option<f32>,
    pub model_hint: Option<String>,
}

fn enforce_bound(weight: &mut Option<f32>) {
    if let Some(w) = weight {
        *w = w.clamp(0.0, 1.0);
    }
}

impl BitModelPreference {
    fn normalize_weight(weight: &mut Option<f32>) {
        if let Some(w) = weight {
            if *w <= 0.0 {
                *weight = None;
            } else if *w > 1.0 {
                *weight = Some(1.0);
            }
        }
    }

    pub fn enforce_bounds(&mut self) {
        enforce_bound(&mut self.cost_weight);
        enforce_bound(&mut self.speed_weight);
        enforce_bound(&mut self.reasoning_weight);
        enforce_bound(&mut self.creativity_weight);
        enforce_bound(&mut self.factfulness_weight);
        enforce_bound(&mut self.function_calling_weight);
        enforce_bound(&mut self.safety_weight);
        enforce_bound(&mut self.openness_weight);
        enforce_bound(&mut self.multilinguality_weight);
        enforce_bound(&mut self.coding_weight);
    }

    pub fn parse(&self) -> Self {
        let mut cloned = self.clone();
        cloned.inner_parse();
        cloned
    }

    fn inner_parse(&mut self) {
        Self::normalize_weight(&mut self.cost_weight);
        Self::normalize_weight(&mut self.speed_weight);
        Self::normalize_weight(&mut self.reasoning_weight);
        Self::normalize_weight(&mut self.creativity_weight);
        Self::normalize_weight(&mut self.factfulness_weight);
        Self::normalize_weight(&mut self.function_calling_weight);
        Self::normalize_weight(&mut self.safety_weight);
        Self::normalize_weight(&mut self.openness_weight);
        Self::normalize_weight(&mut self.multilinguality_weight);
        Self::normalize_weight(&mut self.coding_weight);

        if let Some(model_hint) = &self.model_hint {
            if model_hint.is_empty() {
                self.model_hint = None;
            }
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct BitModelClassification {
    cost: f32,
    speed: f32,
    reasoning: f32,
    creativity: f32,
    factfulness: f32,
    function_calling: f32,
    safety: f32,
    openness: f32,
    multilinguality: f32,
    coding: f32,
}

impl BitModelClassification {
    fn name_similarity(&self, hint: &str, bit: &Bit) -> anyhow::Result<f32> {
        let mut similarity: f32 = 0.0;

        if bit.meta.is_empty() {
            return Err(anyhow::anyhow!("No meta data found"));
        }

        for meta in bit.meta.values() {
            let local_similarity = strsim::jaro_winkler(&meta.name, hint) as f32;
            if local_similarity > similarity {
                similarity = local_similarity;
            }
        }

        let provider = bit.try_to_provider();
        if provider.is_none() {
            return Ok(similarity);
        }

        let provider = provider.unwrap();
        if let Some(model_id) = provider.model_id {
            let local_similarity = strsim::jaro_winkler(&model_id, hint) as f32;
            if local_similarity > similarity {
                similarity = local_similarity;
            }
        }

        Ok(similarity)
    }

    /// Calculates the score of the model in a range from 0 to 1 based on the provided preference
    pub fn score(&self, preference: &BitModelPreference, bit: &Bit) -> f32 {
        let mut total_score = 0.0;
        let name_similarity_weight = 0.2;

        // Map weights to model fields dynamically
        let field_weight_pairs = vec![
            (preference.cost_weight, self.cost),
            (preference.speed_weight, self.speed),
            (preference.reasoning_weight, self.reasoning),
            (preference.creativity_weight, self.creativity),
            (preference.factfulness_weight, self.factfulness),
            (preference.function_calling_weight, self.function_calling),
            (preference.safety_weight, self.safety),
            (preference.openness_weight, self.openness),
            (preference.multilinguality_weight, self.multilinguality),
            (preference.coding_weight, self.coding),
        ];

        // Calculate the weighted sum
        let mut total_weight: f32 = field_weight_pairs.iter().filter_map(|(w, _)| *w).sum();
        if total_weight == 0.0 {
            return 0.0; // Avoid division by zero
        }

        if let Some(hint) = &preference.model_hint {
            if let Ok(name_similarity) = self.name_similarity(hint, bit) {
                if name_similarity > 0.8 {
                    total_score += name_similarity_weight * name_similarity;
                    total_weight += name_similarity_weight;
                }
            }
        }

        for (weight, value) in field_weight_pairs {
            if let Some(w) = weight {
                total_score += w * value;
            }
        }

        total_score / total_weight
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct Bit {
    pub id: String,
    #[serde(rename = "type")]
    pub bit_type: BitTypes,
    pub meta: std::collections::HashMap<String, BitMeta>,
    pub authors: Vec<String>,
    pub repository: Option<String>,
    pub download_link: Option<String>,
    pub file_name: Option<String>,
    pub hash: String,
    pub size: Option<u64>,
    pub hub: String,
    pub parameters: Value,
    pub icon: String,
    pub version: String,
    pub license: String,
    pub dependencies: Vec<(String, String)>,
    pub dependency_tree_hash: String,
    pub created: String,
    pub updated: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct EmbeddingModelParameters {
    pub languages: Vec<String>,
    pub vector_length: u32,
    pub input_length: u32,
    pub prefix: Prefix,
    pub pooling: Pooling,
    pub provider: BitProviderModel,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct Prefix {
    pub query: String,
    pub paragraph: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
pub enum Pooling {
    CLS,
    Mean,
    None,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct ImageEmbeddingModelParameters {
    pub languages: Vec<String>,
    pub vector_length: u32,
    pub pooling: Pooling,
    pub provider: BitProviderModel,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct LLMParameters {
    pub context_length: u32,
    pub provider: BitProviderModel,
    pub model_classification: BitModelClassification,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct VLMParameters {
    pub context_length: u32,
    pub provider: BitProviderModel,
    pub model_classification: BitModelClassification,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct BitPack {
    pub bits: Vec<Bit>,
}

fn collect_dependencies<'a>(
    bit: &'a Bit,
    state: Arc<Mutex<FlowLikeState>>,
    visited: &'a mut HashSet<String>,
    hubs: &'a mut HashMap<String, crate::hub::Hub>,
    dependencies: &'a mut Vec<Bit>,
) -> BoxFuture<'a, ()> {
    async move {
        let http_client = state.lock().await.http_client.clone();
        let bit_id = bit.id.clone();
        if visited.contains(&bit_id) {
            return;
        }
        visited.insert(bit_id.clone());

        dependencies.push(bit.clone());

        for (hub_domain, dependency_id) in bit.dependencies.iter() {
            if !hubs.contains_key(hub_domain) {
                let hub =
                    crate::hub::Hub::new(&format!("https://{hub_domain}"), http_client.clone())
                        .await
                        .unwrap();
                hubs.insert(hub_domain.to_string(), hub);
            }
            let hub = hubs.get(hub_domain).unwrap();
            if let Ok(dependency) = hub.get_bit_by_id(dependency_id).await {
                collect_dependencies(&dependency, state.clone(), visited, hubs, dependencies).await;
            }
        }
    }
    .boxed()
}

impl BitPack {
    pub async fn get_installed(
        &self,
        state: Arc<Mutex<FlowLikeState>>,
    ) -> anyhow::Result<Vec<Bit>> {
        let bits_store = FlowLikeState::bit_store(&state).await?.as_generic();

        let mut installed_bits = vec![];
        for bit in self.bits.iter() {
            let file_name = match bit.file_name.clone() {
                Some(file_name) => Some(file_name),
                None => continue,
            };
            let file_name = file_name.unwrap();
            let bit_path = Path::from(bit.hash.clone()).child(file_name);
            let meta = match bits_store.head(&bit_path).await {
                Ok(meta) => meta,
                Err(_) => continue,
            };

            let size = meta.size as u64;
            if size != bit.size.unwrap_or(0) {
                continue;
            }
            installed_bits.push(bit.clone());
        }
        Ok(installed_bits)
    }

    pub async fn download(&self, state: Arc<Mutex<FlowLikeState>>) -> anyhow::Result<Vec<Bit>> {
        let mut deduplicated_bits = vec![];
        let mut deduplication_helper = HashSet::new();
        self.bits.iter().for_each(|bit| {
            if deduplication_helper.contains(&bit.hash)
                || bit.download_link.is_none()
                || bit.size.is_none()
                || bit.file_name.is_none()
            {
                return;
            }

            if bit.size.unwrap() == 0 {
                return;
            }

            deduplicated_bits.push(bit.clone());
            deduplication_helper.insert(bit.hash.clone());
        });

        let download_futures: Vec<_> = deduplicated_bits
            .iter()
            .map(|bit| download_bit(bit, state.clone(), 3))
            .collect();

        let results = futures::future::join_all(download_futures).await;

        for result in results {
            match result {
                Ok(_) => println!("Download succeeded"),
                Err(e) => eprintln!("Download failed: {}", e),
            }
        }

        Ok(deduplicated_bits)
    }

    pub fn size(&self) -> u64 {
        let mut size = 0;
        for bit in self.bits.iter() {
            if bit.size.is_some() {
                size += bit.size.unwrap();
            }
        }
        size
    }

    pub async fn is_installed(&self, state: Arc<Mutex<FlowLikeState>>) -> anyhow::Result<bool> {
        let bits_store = FlowLikeState::bit_store(&state).await?.as_generic();
        let mut installed = true;
        for bit in self.bits.iter() {
            let file_name = match bit.file_name.clone() {
                Some(file_name) => file_name,
                None => {
                    installed = false;
                    break;
                }
            };
            let bit_path = Path::from(bit.hash.clone()).child(file_name);
            let metadata = match bits_store.head(&bit_path).await {
                Ok(metadata) => metadata,
                Err(_) => {
                    installed = false;
                    break;
                }
            };
            if metadata.size as u64 != bit.size.unwrap_or(0) {
                installed = false;
                break;
            }
        }
        Ok(installed)
    }
}

impl Bit {
    pub fn try_to_llm(&self) -> Option<LLMParameters> {
        if self.bit_type == BitTypes::Llm {
            let parameters = serde_json::from_value::<LLMParameters>(self.parameters.clone());
            if parameters.is_err() {
                return None;
            }
            return Some(parameters.unwrap());
        }
        None
    }

    pub fn try_to_vlm(&self) -> Option<VLMParameters> {
        if self.bit_type == BitTypes::Vlm {
            let parameters = serde_json::from_value::<VLMParameters>(self.parameters.clone());
            if parameters.is_err() {
                return None;
            }
            return Some(parameters.unwrap());
        }
        None
    }

    pub fn score(&self, preference: &BitModelPreference) -> anyhow::Result<f32> {
        if let Some(parameters) = self.try_to_llm() {
            return Ok(parameters.model_classification.score(preference, self));
        }

        if let Some(parameters) = self.try_to_vlm() {
            return Ok(parameters.model_classification.score(preference, self));
        }

        Err(anyhow::anyhow!("Not a Model"))
    }

    pub fn try_to_embedding(&self) -> Option<EmbeddingModelParameters> {
        if self.bit_type == BitTypes::Embedding {
            let parameters =
                serde_json::from_value::<EmbeddingModelParameters>(self.parameters.clone());
            if parameters.is_err() {
                return None;
            }
            return Some(parameters.unwrap());
        }
        None
    }

    pub fn try_to_image_embedding(&self) -> Option<ImageEmbeddingModelParameters> {
        if self.bit_type == BitTypes::ImageEmbedding {
            let parameters =
                serde_json::from_value::<ImageEmbeddingModelParameters>(self.parameters.clone());
            if parameters.is_err() {
                return None;
            }
            return Some(parameters.unwrap());
        }
        None
    }

    pub fn try_to_provider(&self) -> Option<BitProviderModel> {
        if let Some(parameters) = self.try_to_llm() {
            return Some(parameters.provider);
        }

        if let Some(parameters) = self.try_to_vlm() {
            return Some(parameters.provider);
        }

        None
    }

    pub fn try_to_context_length(&self) -> Option<u32> {
        if let Some(parameters) = self.try_to_llm() {
            return Some(parameters.context_length);
        }

        if let Some(parameters) = self.try_to_vlm() {
            return Some(parameters.context_length);
        }

        None
    }

    pub async fn dependencies(&self, state: Arc<Mutex<FlowLikeState>>) -> anyhow::Result<BitPack> {
        let bits_store = FlowLikeState::bit_store(&state).await?.as_generic();

        let mut dependencies = vec![];
        let cache_dir =
            Path::from("deps-cache").child(format!("bit-deps-{}.bin", self.dependency_tree_hash));

        let metadata = bits_store.head(&cache_dir).await;

        if metadata.is_ok() {
            let file = from_compressed::<BitPack>(bits_store.clone(), cache_dir.clone()).await;
            if let Ok(file) = file {
                return Ok(file);
            }
        }

        let mut visited = HashSet::new();
        let mut hubs = HashMap::new();
        let http_client = state.lock().await.http_client.clone();
        for (hub_domain, dependency_id) in self.dependencies.iter() {
            if !hubs.contains_key(hub_domain) {
                let hub =
                    crate::hub::Hub::new(&format!("https://{hub_domain}"), http_client.clone())
                        .await
                        .unwrap();
                hubs.insert(hub_domain.to_string(), hub);
            }
            let hub = hubs.get(hub_domain).unwrap();
            if let Ok(dependency) = hub.get_bit_by_id(dependency_id).await {
                collect_dependencies(
                    &dependency,
                    state.clone(),
                    &mut visited,
                    &mut hubs,
                    &mut dependencies,
                )
                .await;
            }
        }

        let bit_pack = BitPack { bits: dependencies };
        let res = compress_to_file(bits_store, cache_dir, &bit_pack).await;
        if res.is_err() {
            println!(
                "Failed to compress dependencies for {}, err: {}",
                self.id,
                res.err().unwrap()
            );
        }

        Ok(bit_pack)
    }

    pub async fn pack(&self, state: Arc<Mutex<FlowLikeState>>) -> anyhow::Result<BitPack> {
        let mut dependencies = self.dependencies(state).await?;
        dependencies.bits.push(self.clone());
        Ok(dependencies)
    }

    pub async fn is_installed(&self, state: Arc<Mutex<FlowLikeState>>) -> anyhow::Result<bool> {
        let pack = self.pack(state.clone()).await?;
        pack.is_installed(state).await
    }

    pub fn is_multimodal(&self) -> bool {
        self.bit_type == BitTypes::Vlm || self.bit_type == BitTypes::ImageEmbedding
    }

    pub fn to_path(&self, file_system: &Arc<LocalObjectStore>) -> Option<PathBuf> {
        let file_name = self.file_name.clone()?;
        let bit_path = Path::from(self.hash.clone()).child(file_name);
        let path = file_system.path_to_filesystem(&bit_path).ok()?;
        Some(path)
    }
}
