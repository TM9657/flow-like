use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A single keypoint in a pose skeleton
#[derive(Default, Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct Keypoint {
    /// Keypoint index in the skeleton
    pub index: u32,
    /// X coordinate (in image space)
    pub x: f32,
    /// Y coordinate (in image space)
    pub y: f32,
    /// Confidence score for this keypoint (0.0-1.0)
    pub confidence: f32,
    /// Optional keypoint name (e.g., "left_shoulder", "right_knee")
    pub name: Option<String>,
}

impl Keypoint {
    /// Check if keypoint is visible based on confidence threshold
    pub fn is_visible(&self, threshold: f32) -> bool {
        self.confidence >= threshold
    }
}

/// Connection between two keypoints in a skeleton
#[derive(Default, Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct SkeletonConnection {
    /// Index of the first keypoint
    pub from_idx: u32,
    /// Index of the second keypoint
    pub to_idx: u32,
}

/// A complete pose detection result (skeleton)
#[derive(Default, Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct PoseDetection {
    /// All keypoints in this pose
    pub keypoints: Vec<Keypoint>,
    /// Overall confidence score for the pose detection
    pub score: f32,
    /// Optional bounding box (x1, y1, x2, y2) around the detected person
    pub bbox: Option<(f32, f32, f32, f32)>,
    /// Skeleton connections (which keypoints connect to which)
    pub connections: Vec<SkeletonConnection>,
}

impl PoseDetection {
    /// Get keypoint by index
    pub fn get_keypoint(&self, index: u32) -> Option<&Keypoint> {
        self.keypoints.iter().find(|k| k.index == index)
    }

    /// Get keypoint by name
    pub fn get_keypoint_by_name(&self, name: &str) -> Option<&Keypoint> {
        self.keypoints.iter().find(|k| k.name.as_deref() == Some(name))
    }

    /// Get all visible keypoints above confidence threshold
    pub fn visible_keypoints(&self, threshold: f32) -> Vec<&Keypoint> {
        self.keypoints
            .iter()
            .filter(|k| k.is_visible(threshold))
            .collect()
    }
}

/// Standard COCO keypoint names (17 keypoints)
pub const COCO_KEYPOINT_NAMES: [&str; 17] = [
    "nose",
    "left_eye",
    "right_eye",
    "left_ear",
    "right_ear",
    "left_shoulder",
    "right_shoulder",
    "left_elbow",
    "right_elbow",
    "left_wrist",
    "right_wrist",
    "left_hip",
    "right_hip",
    "left_knee",
    "right_knee",
    "left_ankle",
    "right_ankle",
];

/// Standard COCO skeleton connections
pub const COCO_SKELETON_CONNECTIONS: [(u32, u32); 19] = [
    (0, 1),   // nose -> left_eye
    (0, 2),   // nose -> right_eye
    (1, 3),   // left_eye -> left_ear
    (2, 4),   // right_eye -> right_ear
    (5, 6),   // left_shoulder -> right_shoulder
    (5, 7),   // left_shoulder -> left_elbow
    (7, 9),   // left_elbow -> left_wrist
    (6, 8),   // right_shoulder -> right_elbow
    (8, 10),  // right_elbow -> right_wrist
    (5, 11),  // left_shoulder -> left_hip
    (6, 12),  // right_shoulder -> right_hip
    (11, 12), // left_hip -> right_hip
    (11, 13), // left_hip -> left_knee
    (13, 15), // left_knee -> left_ankle
    (12, 14), // right_hip -> right_knee
    (14, 16), // right_knee -> right_ankle
    (0, 5),   // nose -> left_shoulder (optional neck connection)
    (0, 6),   // nose -> right_shoulder (optional neck connection)
    (3, 5),   // left_ear -> left_shoulder (optional)
];
