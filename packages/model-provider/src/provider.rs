use flow_like_types::json::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
pub struct ModelProvider {
    pub provider_name: String,
    pub model_id: Option<String>,
    pub version: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelProviderConfiguration {
    pub openai_config: Option<OpenAIConfig>,
    pub bedrock_config: Option<BedrockConfig>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OpenAIConfig {
    pub api_key: Option<String>,
    pub endpoint: Option<String>,
    pub organization: Option<String>,
    pub proxy: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BedrockConfig {
    pub api_key: String,
    pub api_base: Option<String>,
    pub model_id: String,
}
