use flow_like::{models::llm::ExecutionSettings, profile::Profile as FlowLikeProfile};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionMode {
    Straight,
    Step,
    SimpleBezier,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
pub struct FlowSettings {
    pub connection_mode: ConnectionMode,
}

impl Default for FlowSettings {
    fn default() -> Self {
        FlowSettings {
            connection_mode: ConnectionMode::SimpleBezier,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
pub struct UserProfile {
    #[serde(default)]
    pub hub_profile: FlowLikeProfile,

    #[serde(default)]
    pub execution_settings: ExecutionSettings,

    #[serde(default = "Vec::new")]
    pub vaults: Vec<String>,

    #[serde(default = "Vec::new")]
    pub apps: Vec<String>,

    #[serde(default)]
    pub flow_settings: FlowSettings,

    pub updated: String,
    pub created: String,
}

impl UserProfile {
    pub fn new(profile: FlowLikeProfile) -> Self {
        UserProfile {
            hub_profile: profile,
            execution_settings: ExecutionSettings::new(),
            vaults: Vec::new(),
            apps: Vec::new(),
            flow_settings: FlowSettings::default(),
            updated: String::new(),
            created: String::new(),
        }
    }
}
