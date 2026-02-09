use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Classification prediction result from ML models
#[derive(Default, Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct ClassPrediction {
    /// Class index (0-based)
    pub class_idx: u32,
    /// Confidence score (typically 0.0-1.0)
    pub score: f32,
    /// Optional human-readable class label
    pub label: Option<String>,
}

impl ClassPrediction {
    /// Create a new prediction with just class index and score
    pub fn new(class_idx: u32, score: f32) -> Self {
        Self {
            class_idx,
            score,
            label: None,
        }
    }

    /// Create a new prediction with label
    pub fn with_label(class_idx: u32, score: f32, label: impl Into<String>) -> Self {
        Self {
            class_idx,
            score,
            label: Some(label.into()),
        }
    }
}
