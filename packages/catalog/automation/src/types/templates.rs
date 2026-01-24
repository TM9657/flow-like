use chrono::{DateTime, Utc};
use flow_like_catalog_core::{BoundingBox, FlowPath};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct TemplateRef {
    pub template_id: String,
    pub artifact_path: FlowPath,
    pub original_bbox: BoundingBox,
    pub scale_invariant: bool,
    pub grayscale: bool,
    pub created_at: DateTime<Utc>,
    pub description: Option<String>,
}

impl TemplateRef {
    pub fn new(
        template_id: impl Into<String>,
        artifact_path: FlowPath,
        original_bbox: BoundingBox,
    ) -> Self {
        Self {
            template_id: template_id.into(),
            artifact_path,
            original_bbox,
            scale_invariant: false,
            grayscale: true,
            created_at: Utc::now(),
            description: None,
        }
    }

    pub fn with_scale_invariant(mut self, scale_invariant: bool) -> Self {
        self.scale_invariant = scale_invariant;
        self
    }

    pub fn with_grayscale(mut self, grayscale: bool) -> Self {
        self.grayscale = grayscale;
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct MatchResult {
    pub found: bool,
    pub bbox: Option<BoundingBox>,
    pub confidence: f64,
    pub center: Option<(i32, i32)>,
    pub scale: Option<f64>,
    pub match_time_ms: u64,
}

impl MatchResult {
    pub fn not_found() -> Self {
        Self {
            found: false,
            bbox: None,
            confidence: 0.0,
            center: None,
            scale: None,
            match_time_ms: 0,
        }
    }

    pub fn found(bbox: BoundingBox, confidence: f64) -> Self {
        let center = Some((
            ((bbox.x1 + bbox.x2) / 2.0) as i32,
            ((bbox.y1 + bbox.y2) / 2.0) as i32,
        ));
        Self {
            found: true,
            bbox: Some(bbox),
            confidence,
            center,
            scale: Some(1.0),
            match_time_ms: 0,
        }
    }

    pub fn with_match_time(mut self, time_ms: u64) -> Self {
        self.match_time_ms = time_ms;
        self
    }

    pub fn with_scale(mut self, scale: f64) -> Self {
        self.scale = Some(scale);
        self
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct TemplateMatchOptions {
    pub threshold: f64,
    pub search_region: Option<BoundingBox>,
    pub scales: Option<Vec<f64>>,
    pub use_grayscale: bool,
    pub use_canny_edge: bool,
}

impl Default for TemplateMatchOptions {
    fn default() -> Self {
        Self {
            threshold: 0.8,
            search_region: None,
            scales: None,
            use_grayscale: true,
            use_canny_edge: false,
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct TemplateMatchAllOptions {
    pub threshold: f64,
    pub search_region: Option<BoundingBox>,
    pub max_hits: usize,
    pub non_max_suppression: bool,
    pub nms_threshold: f64,
}

impl Default for TemplateMatchAllOptions {
    fn default() -> Self {
        Self {
            threshold: 0.8,
            search_region: None,
            max_hits: 10,
            non_max_suppression: true,
            nms_threshold: 0.3,
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct ClickTemplateOptions {
    pub click_offset: Option<(i32, i32)>,
    pub retries: u32,
    pub retry_delay_ms: u64,
    pub threshold: f64,
}

impl Default for ClickTemplateOptions {
    fn default() -> Self {
        Self {
            click_offset: None,
            retries: 3,
            retry_delay_ms: 500,
            threshold: 0.8,
        }
    }
}

/// Simple result from rustautogui template matching
#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct TemplateMatchResult {
    pub found: bool,
    pub x: i32,
    pub y: i32,
    pub confidence: f64,
    pub template_path: String,
}
