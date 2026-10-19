use super::{StudioForecastRequest, StudioForecastResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MAX_SNAPSHOT_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_SAVED_FORECASTS: usize = 1000;
pub const SAVED_FORECASTS_TABLE: &str = "__studio_forecasts__";

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateStudioForecast {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub request: StudioForecastRequest,
    pub result: StudioForecastResult,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct SavedStudioForecast {
    pub id: String,
    pub app_id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_by: String,
    pub created_at: String,
    pub request: StudioForecastRequest,
    pub result: StudioForecastResult,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct SavedStudioForecastSummary {
    pub id: String,
    pub app_id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_by: String,
    pub created_at: String,
    pub model_bit_id: String,
    pub horizon: usize,
    pub run_id: String,
}

#[cfg(feature = "studio")]
mod persistence;
#[cfg(feature = "studio")]
pub use persistence::{create, delete, get, list};
#[cfg(all(test, feature = "studio"))]
mod tests;
#[cfg(any(feature = "studio", test))]
mod validation;
