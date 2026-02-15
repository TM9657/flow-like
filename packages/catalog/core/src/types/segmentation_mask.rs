use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Segmentation mask result from semantic/instance segmentation models
#[derive(Default, Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct SegmentationMask {
    /// Width of the mask
    pub width: u32,
    /// Height of the mask
    pub height: u32,
    /// Class index for each pixel (flattened, row-major order)
    /// For semantic segmentation: class_idx per pixel
    /// For instance segmentation: instance_id per pixel
    pub data: Vec<u8>,
    /// Number of classes in the segmentation
    pub num_classes: u32,
    /// Optional class labels mapping
    pub class_labels: Option<Vec<String>>,
}

impl SegmentationMask {
    /// Get class index at pixel (x, y)
    pub fn get_class_at(&self, x: u32, y: u32) -> Option<u8> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = (y * self.width + x) as usize;
        self.data.get(idx).copied()
    }

    /// Get label at pixel (x, y) if labels are available
    pub fn get_label_at(&self, x: u32, y: u32) -> Option<&str> {
        let class_idx = self.get_class_at(x, y)? as usize;
        self.class_labels
            .as_ref()
            .and_then(|labels| labels.get(class_idx).map(|s| s.as_str()))
    }
}

/// Instance segmentation result combining mask with detection info
#[derive(Default, Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct InstanceSegmentation {
    /// Bounding box coordinates (x1, y1, x2, y2)
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    /// Confidence score
    pub score: f32,
    /// Class index
    pub class_idx: i32,
    /// Optional class name
    pub class_name: Option<String>,
    /// Binary mask for this instance (within bounding box)
    /// Flattened, row-major, values 0 or 1
    pub mask: Vec<u8>,
    /// Mask width
    pub mask_width: u32,
    /// Mask height
    pub mask_height: u32,
}
