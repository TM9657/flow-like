use std::collections::HashMap;

use aws_config::SdkConfig;
use flow_like_types::{
    Value,
    json::{Deserialize, Serialize},
    rand::{self, Rng},
};
use schemars::JsonSchema;

pub mod openai;

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
pub struct ModelProvider {
    pub provider_name: String,
    pub model_id: Option<String>,
    pub version: Option<String>,
    pub params: Option<HashMap<String, Value>>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
pub struct EmbeddingModelProvider {
    pub languages: Vec<String>,
    pub vector_length: u32,
    pub input_length: u32,
    pub prefix: Prefix,
    pub pooling: Pooling,
    pub provider: ModelProvider,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct ImageEmbeddingModelProvider {
    pub languages: Vec<String>,
    pub vector_length: u32,
    pub pooling: Pooling,
    pub provider: ModelProvider,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
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

#[derive(Clone, Default, Debug)]
pub struct ModelProviderConfiguration {
    pub openai_config: Vec<OpenAIConfig>,
    pub bedrock_config: Vec<BedrockConfig>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OpenAIConfig {
    pub api_key: Option<String>,
    pub endpoint: Option<String>,
    pub organization: Option<String>,
    pub proxy: Option<String>,
}

#[derive(Clone, Debug)]
pub struct BedrockConfig {
    pub config: SdkConfig,
}

pub fn random_provider<T>(vec: &[T]) -> flow_like_types::Result<T>
where
    T: Clone,
{
    if vec.is_empty() {
        return Err(flow_like_types::anyhow!("No Provider found"));
    }

    let index = {
        let mut rng = rand::rng();
        rng.random_range(0..vec.len())
    };
    Ok(vec[index].clone())
}
