use serde::{Deserialize, Serialize};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid training input: {0}")]
    Invalid(String),
    #[error("Backend unavailable: {0}")]
    Backend(String),
    #[error("Training cancelled; latest completed batch is checkpointed")]
    Cancelled,
    #[error("Model record: {0}")]
    Record(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(
    Clone, Debug, Default, Serialize, Deserialize, schemars::JsonSchema, PartialEq, Eq, Hash,
)]
#[serde(tag = "backend", rename_all = "snake_case")]
pub enum BackendChoice {
    #[default]
    Auto,
    Cpu,
    Wgpu,
    WgpuAdapter {
        kind: WgpuAdapterKind,
        device: usize,
    },
    Cuda {
        device: usize,
    },
    Rocm {
        device: usize,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, schemars::JsonSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum WgpuAdapterKind {
    Discrete,
    Integrated,
    Virtual,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, schemars::JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Objective {
    Classification,
    Regression,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
#[serde(tag = "architecture", rename_all = "snake_case")]
pub enum Recipe {
    Mlp {
        input_features: usize,
        hidden: usize,
        outputs: usize,
        objective: Objective,
    },
    Lstm {
        input_features: usize,
        hidden: usize,
        outputs: usize,
        objective: Objective,
    },
    Gru {
        input_features: usize,
        hidden: usize,
        outputs: usize,
        objective: Objective,
    },
    Cnn1d {
        input_features: usize,
        hidden: usize,
        outputs: usize,
        objective: Objective,
    },
    Tcn {
        input_features: usize,
        hidden: usize,
        levels: usize,
        outputs: usize,
        objective: Objective,
    },
    DenseAutoencoder {
        input_features: usize,
        hidden: usize,
        latent: usize,
    },
    Conv1dAutoencoder {
        input_features: usize,
        hidden: usize,
        latent: usize,
    },
    LstmAutoencoder {
        input_features: usize,
        hidden: usize,
        latent: usize,
    },
    CnnLstm {
        input_channels: usize,
        cnn_channels: usize,
        hidden: usize,
        outputs: usize,
        objective: Objective,
    },
    ImageSensorFusion {
        input_channels: usize,
        height: usize,
        width: usize,
        sensor_features: usize,
        cnn_channels: usize,
        hidden: usize,
        outputs: usize,
        objective: Objective,
    },
    ResNet18 {
        input_channels: usize,
        classes: usize,
        base_channels: usize,
    },
    DinoV2 {
        config: crate::DinoV2Config,
    },
    RtdetrV2 {
        config: crate::RtdetrV2Config,
    },
    RfDetr {
        config: crate::RfDetrConfig,
    },
    DfineNano {
        config: crate::DfineConfig,
    },
    MobileNetV2 {
        input_channels: usize,
        classes: usize,
        width_multiplier: f64,
    },
    EfficientNet {
        input_channels: usize,
        classes: usize,
        width_multiplier: f64,
    },
    YoloX {
        input_channels: usize,
        classes: usize,
        width_multiplier: f64,
        depth_multiplier: f64,
    },
    MaskRcnn {
        config: crate::MaskRcnnConfig,
    },
    UNet {
        input_channels: usize,
        classes: usize,
        base_channels: usize,
        depth: usize,
    },
}
impl Recipe {
    pub fn objective(&self) -> Option<Objective> {
        match self {
            Self::Mlp { objective, .. }
            | Self::Lstm { objective, .. }
            | Self::Gru { objective, .. }
            | Self::Cnn1d { objective, .. }
            | Self::Tcn { objective, .. }
            | Self::CnnLstm { objective, .. }
            | Self::ImageSensorFusion { objective, .. } => Some(*objective),
            Self::ResNet18 { .. }
            | Self::DinoV2 { .. }
            | Self::RtdetrV2 { .. }
            | Self::RfDetr { .. }
            | Self::DfineNano { .. }
            | Self::MaskRcnn { .. }
            | Self::YoloX { .. }
            | Self::MobileNetV2 { .. }
            | Self::EfficientNet { .. }
            | Self::UNet { .. } => Some(Objective::Classification),
            _ => None,
        }
    }
    pub fn outputs(&self) -> usize {
        match self {
            Self::Mlp { outputs, .. }
            | Self::Lstm { outputs, .. }
            | Self::Gru { outputs, .. }
            | Self::Cnn1d { outputs, .. }
            | Self::Tcn { outputs, .. }
            | Self::CnnLstm { outputs, .. }
            | Self::ImageSensorFusion { outputs, .. } => *outputs,
            Self::ResNet18 { classes, .. }
            | Self::YoloX { classes, .. }
            | Self::MobileNetV2 { classes, .. }
            | Self::EfficientNet { classes, .. }
            | Self::UNet { classes, .. } => *classes,
            Self::MaskRcnn { config } => config.classes,
            Self::DinoV2 { config } => config.classes,
            Self::RtdetrV2 { config } => config.classes,
            Self::RfDetr { config } => config.classes,
            Self::DfineNano { config } => config.classes,
            Self::DenseAutoencoder { input_features, .. }
            | Self::Conv1dAutoencoder { input_features, .. }
            | Self::LstmAutoencoder { input_features, .. } => *input_features,
        }
    }
    pub fn is_detection(&self) -> bool {
        matches!(
            self,
            Self::YoloX { .. }
                | Self::MaskRcnn { .. }
                | Self::RtdetrV2 { .. }
                | Self::RfDetr { .. }
                | Self::DfineNano { .. }
        )
    }
    pub fn is_instance_segmentation(&self) -> bool {
        matches!(self, Self::MaskRcnn { .. })
    }
    pub fn is_segmentation(&self) -> bool {
        matches!(self, Self::UNet { .. })
    }
    pub fn validate(&self) -> Result<()> {
        let (input, hidden, extra) = match self {
            Self::DfineNano { config } => {
                config.validate()?;
                (3, 128, 1)
            }
            Self::RfDetr { config } => {
                config.validate()?;
                (3, 384, 1)
            }
            Self::RtdetrV2 { config } => {
                config.validate()?;
                (3, 256, 1)
            }
            Self::DinoV2 { config } => {
                config.validate()?;
                (3, config.variant.embedding_dim(), 1)
            }
            Self::MaskRcnn { config } => {
                config.validate()?;
                (config.input_channels, config.base_channels, 1)
            }
            Self::Mlp {
                input_features,
                hidden,
                ..
            }
            | Self::Lstm {
                input_features,
                hidden,
                ..
            }
            | Self::Gru {
                input_features,
                hidden,
                ..
            }
            | Self::Cnn1d {
                input_features,
                hidden,
                ..
            } => (*input_features, *hidden, 1),
            Self::Tcn {
                input_features,
                hidden,
                levels,
                ..
            } => {
                if *levels == 0 || *levels > 12 {
                    return Err(Error::Invalid("TCN levels must be 1..=12".into()));
                }
                (*input_features, *hidden, *levels)
            }
            Self::DenseAutoencoder {
                input_features,
                hidden,
                latent,
            }
            | Self::Conv1dAutoencoder {
                input_features,
                hidden,
                latent,
            } => (*input_features, *hidden, *latent),
            Self::LstmAutoencoder {
                input_features,
                hidden,
                latent,
            } => (*input_features, *hidden, *latent),
            Self::CnnLstm {
                input_channels,
                cnn_channels,
                hidden,
                ..
            } => (*input_channels, *hidden, *cnn_channels),
            Self::ImageSensorFusion {
                input_channels,
                height,
                width,
                sensor_features,
                cnn_channels,
                hidden,
                ..
            } => {
                if *height < 4
                    || *width < 4
                    || *height > 4096
                    || *width > 4096
                    || *sensor_features == 0
                    || *sensor_features > 65536
                    || *cnn_channels == 0
                    || *cnn_channels > 512
                {
                    return Err(Error::Invalid(
                        "Invalid image/sensor fusion dimensions".into(),
                    ));
                }
                (*input_channels, *hidden, *cnn_channels)
            }
            Self::ResNet18 {
                input_channels,
                base_channels,
                ..
            } => (*input_channels, *base_channels, 1),
            Self::MobileNetV2 {
                input_channels,
                width_multiplier,
                ..
            }
            | Self::EfficientNet {
                input_channels,
                width_multiplier,
                ..
            } => {
                if !width_multiplier.is_finite()
                    || *width_multiplier <= 0.0
                    || *width_multiplier > 4.0
                {
                    return Err(Error::Invalid(
                        "Width multiplier must be within (0, 4]".into(),
                    ));
                }
                (*input_channels, 1280, 1)
            }
            Self::YoloX {
                input_channels,
                width_multiplier,
                depth_multiplier,
                ..
            } => {
                if !width_multiplier.is_finite()
                    || *width_multiplier <= 0.0
                    || *width_multiplier > 2.0
                    || !depth_multiplier.is_finite()
                    || *depth_multiplier <= 0.0
                    || *depth_multiplier > 2.0
                {
                    return Err(Error::Invalid(
                        "YOLOX width/depth must be within (0, 2]".into(),
                    ));
                }
                (*input_channels, 1024, 1)
            }
            Self::UNet {
                input_channels,
                base_channels,
                depth,
                ..
            } => {
                if *depth == 0 || *depth > 5 {
                    return Err(Error::Invalid("U-Net depth must be 1..=5".into()));
                }
                (*input_channels, *base_channels, *depth)
            }
        };
        if [input, hidden, extra, self.outputs()].contains(&0)
            || input > 65536
            || hidden > 4096
            || extra > 65536
            || self.outputs() > 65536
        {
            return Err(Error::Invalid(
                "Model dimensions must be positive and within limits".into(),
            ));
        }
        if !self.is_detection()
            && self.objective() == Some(Objective::Classification)
            && self.outputs() < 2
        {
            return Err(Error::Invalid(
                "Classification requires at least two classes".into(),
            ));
        }
        Ok(())
    }
    pub fn validate_input(&self, input: &TensorData) -> Result<()> {
        input.validate()?;
        self.validate_shape(&input.shape)
    }
    pub fn validate_shape(&self, shape: &[usize]) -> Result<()> {
        self.validate()?;
        let elements = shape.iter().try_fold(1usize, |n, d| n.checked_mul(*d));
        if shape.is_empty()
            || shape.len() > 5
            || shape.contains(&0)
            || elements.is_none_or(|n| n > 268_435_456)
        {
            return Err(Error::Invalid(
                "Tensor shape exceeds the supported rank or element budget".into(),
            ));
        }
        let valid = match self {
            Self::DinoV2 { config } => config.validate_input_shape(shape).is_ok(),
            Self::RtdetrV2 { config } => config.validate_input_shape(shape).is_ok(),
            Self::RfDetr { config } => config.validate_input_shape(shape).is_ok(),
            Self::DfineNano { config } => config.validate_input_shape(shape).is_ok(),
            Self::MaskRcnn { config } => {
                shape.len() == 4
                    && shape[1] == config.input_channels
                    && shape[2] >= 16
                    && shape[3] >= 16
                    && shape[2]
                        .checked_mul(shape[3])
                        .is_some_and(|n| n <= 4_194_304)
            }
            Self::Mlp { input_features, .. } | Self::DenseAutoencoder { input_features, .. } => {
                shape.len() == 2 && shape[1] == *input_features
            }
            Self::Lstm { input_features, .. }
            | Self::Gru { input_features, .. }
            | Self::Cnn1d { input_features, .. }
            | Self::Tcn { input_features, .. }
            | Self::Conv1dAutoencoder { input_features, .. }
            | Self::LstmAutoencoder { input_features, .. } => {
                shape.len() == 3 && shape[2] == *input_features
            }
            Self::CnnLstm { input_channels, .. } => {
                shape.len() == 5 && shape[2] == *input_channels && shape[3] >= 4 && shape[4] >= 4
            }
            Self::ImageSensorFusion {
                input_channels,
                height,
                width,
                sensor_features,
                ..
            } => {
                shape.len() == 2
                    && shape[1]
                        == input_channels
                            .checked_mul(*height)
                            .and_then(|n| n.checked_mul(*width))
                            .and_then(|n| n.checked_add(*sensor_features))
                            .unwrap_or(usize::MAX)
            }
            Self::ResNet18 { input_channels, .. }
            | Self::MobileNetV2 { input_channels, .. }
            | Self::EfficientNet { input_channels, .. } => {
                shape.len() == 4 && shape[1] == *input_channels && shape[2] >= 8 && shape[3] >= 8
            }
            Self::YoloX { input_channels, .. } => {
                shape.len() == 4
                    && shape[1] == *input_channels
                    && shape[2] >= 32
                    && shape[3] >= 32
                    && shape[2].is_multiple_of(32)
                    && shape[3].is_multiple_of(32)
            }
            Self::UNet {
                input_channels,
                depth,
                ..
            } => {
                let divisor = 1usize << depth;
                shape.len() == 4
                    && shape[1] == *input_channels
                    && shape[2].is_multiple_of(divisor)
                    && shape[3].is_multiple_of(divisor)
            }
        };
        if !valid {
            return Err(Error::Invalid(format!(
                "Input shape {:?} does not match recipe",
                shape
            )));
        }
        Ok(())
    }
}

/// Temporal inputs use [batch, time, channels]; images use [batch, channels, height, width].
/// Video adds time before channels: [batch, time, channels, height, width].
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
pub struct TensorData {
    pub shape: Vec<usize>,
    pub values: Vec<f32>,
}
impl TensorData {
    pub fn validate(&self) -> Result<()> {
        if self.shape.is_empty() || self.shape.len() > 5 || self.shape.contains(&0) {
            return Err(Error::Invalid(
                "Tensor rank must be 1..=5 with positive dimensions".into(),
            ));
        }
        let count = self.shape.iter().try_fold(1usize, |n, d| n.checked_mul(*d));
        if count != Some(self.values.len()) || self.values.len() > 268_435_456 {
            return Err(Error::Invalid(
                "Tensor shape/length mismatch or element limit exceeded".into(),
            ));
        }
        if self.values.iter().any(|v| !v.is_finite()) {
            return Err(Error::Invalid("Tensor contains a non-finite value".into()));
        }
        Ok(())
    }
    #[cfg(feature = "engine")]
    pub(crate) fn batch(&self, indices: &[usize]) -> Self {
        let width = self.values.len() / self.shape[0];
        let values = indices
            .iter()
            .flat_map(|i| self.values[i * width..(i + 1) * width].iter().copied())
            .collect();
        let mut shape = self.shape.clone();
        shape[0] = indices.len();
        Self { shape, values }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Targets {
    Classes {
        values: Vec<i64>,
    },
    Dense {
        tensor: TensorData,
    },
    Segmentation {
        shape: Vec<usize>,
        values: Vec<i64>,
    },
    Reconstruction,
    Boxes {
        values: Vec<Vec<BoundingBox>>,
    },
    Instances {
        values: Vec<Vec<crate::InstanceTarget>>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TensorDataset {
    pub inputs: TensorData,
    pub targets: Targets,
}
impl TensorDataset {
    pub fn validate(&self, recipe: &Recipe) -> Result<()> {
        recipe.validate_input(&self.inputs)?;
        let n = self.inputs.shape[0];
        if recipe.is_instance_segmentation() {
            if let Targets::Instances { values } = &self.targets {
                if values.len() != n {
                    return Err(Error::Invalid(
                        "Instance target batch size differs from input".into(),
                    ));
                }
                for instances in values {
                    if instances.len() > MASK_RCNN_MAX_INSTANCES {
                        return Err(Error::Invalid(format!(
                            "An image has {} instances; Mask R-CNN accepts at most {MASK_RCNN_MAX_INSTANCES} per image",
                            instances.len()
                        )));
                    }
                    for instance in instances {
                        instance.validate(
                            recipe.outputs(),
                            self.inputs.shape[2],
                            self.inputs.shape[3],
                        )?;
                    }
                    if instance_mask_elements(instances)
                        .is_none_or(|elements| elements > MASK_RCNN_ELEMENT_BUDGET)
                    {
                        return Err(Error::Invalid(format!(
                            "An image's {} full-image instance masks exceed the Mask R-CNN limit of {MASK_RCNN_ELEMENT_BUDGET} mask elements",
                            instances.len()
                        )));
                    }
                }
                return Ok(());
            }
            return Err(Error::Invalid(
                "Mask R-CNN requires instance boxes and masks".into(),
            ));
        }
        if recipe.is_detection() {
            if let Targets::Boxes { values } = &self.targets {
                if values.len() != n {
                    return Err(Error::Invalid(
                        "Detection target batch size differs from input".into(),
                    ));
                }
                for boxes in values {
                    for bbox in boxes {
                        bbox.validate(recipe.outputs())?;
                    }
                }
                return Ok(());
            }
            return Err(Error::Invalid(
                "Object detection requires bounding-box targets".into(),
            ));
        }
        match (&self.targets, recipe.objective(), recipe.is_segmentation()) {
            (Targets::Classes { values }, Some(Objective::Classification), false) => {
                if values.len() != n
                    || values
                        .iter()
                        .any(|v| *v < 0 || *v as usize >= recipe.outputs())
                {
                    return Err(Error::Invalid(
                        "Class labels must match batch size and class range".into(),
                    ));
                }
            }
            (Targets::Dense { tensor }, Some(Objective::Regression), false) => {
                tensor.validate()?;
                if tensor.shape != [n, recipe.outputs()] {
                    return Err(Error::Invalid(
                        "Regression targets must be [batch, outputs]".into(),
                    ));
                }
            }
            (Targets::Segmentation { shape, values }, _, true) => {
                let expected = vec![n, self.inputs.shape[2], self.inputs.shape[3]];
                if *shape != expected
                    || values.len() != expected.iter().product::<usize>()
                    || values
                        .iter()
                        .any(|v| *v < 0 || *v as usize >= recipe.outputs())
                {
                    return Err(Error::Invalid(
                        "Segmentation labels must be [batch, height, width] and valid class IDs"
                            .into(),
                    ));
                }
            }
            (Targets::Reconstruction, None, false) => {}
            _ => {
                return Err(Error::Invalid(
                    "Target kind does not match the recipe".into(),
                ));
            }
        }
        Ok(())
    }
    #[cfg(feature = "engine")]
    pub(crate) fn batch(&self, indices: &[usize]) -> Self {
        let targets = match &self.targets {
            Targets::Classes { values } => Targets::Classes {
                values: indices.iter().map(|i| values[*i]).collect(),
            },
            Targets::Dense { tensor } => Targets::Dense {
                tensor: tensor.batch(indices),
            },
            Targets::Reconstruction => Targets::Reconstruction,
            Targets::Boxes { values } => Targets::Boxes {
                values: indices.iter().map(|i| values[*i].clone()).collect(),
            },
            Targets::Instances { values } => Targets::Instances {
                values: indices.iter().map(|i| values[*i].clone()).collect(),
            },
            Targets::Segmentation { shape, values } => {
                let width = values.len() / shape[0];
                let mut new_shape = shape.clone();
                new_shape[0] = indices.len();
                Targets::Segmentation {
                    shape: new_shape,
                    values: indices
                        .iter()
                        .flat_map(|i| values[i * width..(i + 1) * width].iter().copied())
                        .collect(),
                }
            }
        };
        Self {
            inputs: self.inputs.batch(indices),
            targets,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
pub struct TrainingConfig {
    pub recipe: Recipe,
    #[serde(default)]
    pub backend: BackendChoice,
    pub epochs: usize,
    pub batch_size: usize,
    pub learning_rate: f64,
    pub seed: u64,
    #[serde(default = "default_clip")]
    pub gradient_clip: f32,
}
fn default_clip() -> f32 {
    5.0
}
impl TrainingConfig {
    pub fn validate(&self) -> Result<()> {
        self.validate_stored()?;
        if let Recipe::MaskRcnn { config } = &self.recipe {
            validate_mask_rcnn_roi_budget(config, self.batch_size)?;
        }
        Ok(())
    }

    /// The checks a saved model's configuration must pass to load. Training budgets stay in
    /// validate(), so a model trained before a budget existed still loads.
    pub fn validate_stored(&self) -> Result<()> {
        self.recipe.validate()?;
        if self.recipe.is_instance_segmentation() && self.batch_size > 32 {
            return Err(Error::Invalid(
                "Mask R-CNN batches are limited to 32 images".into(),
            ));
        }
        if self.epochs == 0
            || self.epochs > 1_000_000
            || self.batch_size == 0
            || self.batch_size > 4096
            || !self.learning_rate.is_finite()
            || self.learning_rate <= 0.0
            || self.learning_rate > 1.0
            || !self.gradient_clip.is_finite()
            || self.gradient_clip <= 0.0
            || self.gradient_clip > 1_000_000.0
        {
            return Err(Error::Invalid(
                "Training settings must be positive and within epoch, batch, learning-rate and gradient limits".into(),
            ));
        }
        Ok(())
    }

    /// Validates `data` against the recipe and against the largest batch a shuffled epoch of
    /// `batch_size` samples can draw, so per-batch limits fail before training starts.
    pub fn validate_dataset(&self, data: &TensorDataset) -> Result<()> {
        data.validate(&self.recipe)?;
        if let Targets::Instances { values } = &data.targets {
            validate_instance_batches(values, self.batch_size, MASK_RCNN_ELEMENT_BUDGET)?;
        }
        Ok(())
    }
}

/// Per-batch element budget for Mask R-CNN ROI mask-head activations and full-image target
/// masks, and the per-image budget for predicted full-image masks.
pub(crate) const MASK_RCNN_ELEMENT_BUDGET: usize = 64 * 1024 * 1024;
pub(crate) const MASK_RCNN_MAX_INSTANCES: usize = 1024;

/// Elements of the widest mask-head activation: `[rois, 2 * base_channels, mask, mask]`.
pub(crate) fn mask_rcnn_roi_elements(config: &crate::MaskRcnnConfig, rois: usize) -> Option<usize> {
    let channels = config.base_channels.checked_mul(2)?;
    let area = config.mask_size.checked_mul(config.mask_size)?;
    rois.checked_mul(channels)?.checked_mul(area)
}

pub(crate) fn instance_mask_elements(instances: &[crate::InstanceTarget]) -> Option<usize> {
    instances.iter().try_fold(0usize, |total, instance| {
        total.checked_add(instance.mask.values.len())
    })
}

fn validate_mask_rcnn_roi_budget(config: &crate::MaskRcnnConfig, batch_size: usize) -> Result<()> {
    let per_image = mask_rcnn_roi_elements(config, config.training_samples).unwrap_or(usize::MAX);
    let fitting = MASK_RCNN_ELEMENT_BUDGET / per_image.max(1);
    if batch_size <= fitting {
        return Ok(());
    }
    Err(Error::Invalid(if fitting == 0 {
        format!(
            "Mask R-CNN training ROIs of one image need {per_image} mask-head elements, above the {MASK_RCNN_ELEMENT_BUDGET}-element limit; reduce training_samples, base_channels or mask_size"
        )
    } else {
        format!(
            "Mask R-CNN batch size {batch_size} exceeds the {MASK_RCNN_ELEMENT_BUDGET}-element ROI mask-head budget; use at most {fitting} images per batch or reduce training_samples, base_channels or mask_size"
        )
    }))
}

fn validate_instance_batches(
    values: &[Vec<crate::InstanceTarget>],
    batch_size: usize,
    budget: usize,
) -> Result<()> {
    let mut elements = values
        .iter()
        .map(|instances| instance_mask_elements(instances).unwrap_or(usize::MAX))
        .collect::<Vec<_>>();
    elements.sort_unstable_by(|a, b| b.cmp(a));
    let fitting = elements
        .iter()
        .scan(0usize, |total, count| {
            *total = total.saturating_add(*count);
            Some(*total)
        })
        .take_while(|total| *total <= budget)
        .count();
    let batch = batch_size.min(elements.len());
    if fitting < batch {
        return Err(Error::Invalid(format!(
            "Instance masks of the {batch} largest images total more than the {budget}-element Mask R-CNN batch limit; use a batch size of at most {fitting}"
        )));
    }
    Ok(())
}

#[derive(Clone, Debug, Default)]
pub struct CancellationToken(Arc<AtomicBool>);
impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TrainingProgress {
    pub epoch: usize,
    pub batch: usize,
    pub steps: usize,
    pub training_loss: f32,
    pub validation_loss: Option<f32>,
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TrainingReport {
    /// The concrete backend used for the latest training session.
    #[serde(default)]
    pub backend: Option<BackendChoice>,
    pub completed_epochs: usize,
    pub steps: usize,
    pub initial_loss: f32,
    pub final_loss: f32,
    pub validation_loss: Option<f32>,
    pub history: Vec<TrainingProgress>,
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PredictionBatch {
    pub output: TensorData,
    pub classes: Option<Vec<i64>>,
    pub reconstruction_error: Option<Vec<f32>>,
    pub detections: Option<Vec<Vec<Detection>>>,
    pub instances: Option<Vec<Vec<crate::InstanceDetection>>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
pub struct BoundingBox {
    pub class_id: usize,
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
}
impl BoundingBox {
    fn validate(&self, classes: usize) -> Result<()> {
        if self.class_id >= classes
            || [self.x_min, self.y_min, self.x_max, self.y_max]
                .iter()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            || self.x_max <= self.x_min
            || self.y_max <= self.y_min
        {
            return Err(Error::Invalid(
                "Bounding boxes need valid classes and nonempty normalized coordinates".into(),
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
pub struct Detection {
    pub class_id: usize,
    pub confidence: f32,
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct DetectionOptions {
    pub confidence_threshold: f32,
    pub iou_threshold: f32,
    pub max_detections: usize,
}
impl Default for DetectionOptions {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.25,
            iou_threshold: 0.45,
            max_detections: 300,
        }
    }
}
impl DetectionOptions {
    pub fn validate(&self) -> Result<()> {
        if !self.confidence_threshold.is_finite()
            || !self.iou_threshold.is_finite()
            || !(0.0..=1.0).contains(&self.confidence_threshold)
            || !(0.0..=1.0).contains(&self.iou_threshold)
            || self.max_detections == 0
            || self.max_detections > 10000
        {
            return Err(Error::Invalid(
                "Invalid detection thresholds or output limit".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct BackendProbe {
    pub backend: BackendChoice,
    pub device: String,
    pub max_absolute_error: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mask_rcnn(config: crate::MaskRcnnConfig, batch_size: usize) -> TrainingConfig {
        TrainingConfig {
            recipe: Recipe::MaskRcnn { config },
            backend: BackendChoice::Cpu,
            epochs: 1,
            batch_size,
            learning_rate: 0.001,
            seed: 1,
            gradient_clip: 5.0,
        }
    }

    fn instances(count: usize, side: usize) -> Vec<crate::InstanceTarget> {
        (0..count)
            .map(|_| crate::InstanceTarget {
                bbox: BoundingBox {
                    class_id: 0,
                    x_min: 0.25,
                    y_min: 0.25,
                    x_max: 0.75,
                    y_max: 0.75,
                },
                mask: TensorData {
                    shape: vec![side, side],
                    values: vec![0.0; side * side],
                },
            })
            .collect()
    }

    #[test]
    fn mask_rcnn_training_config_rejects_batches_over_the_roi_mask_budget() {
        // 64 training ROIs x 64 channels x 28 x 28 = 3,211,264 mask-head elements per image.
        let config = crate::MaskRcnnConfig::default();
        assert!(mask_rcnn(config.clone(), 20).validate().is_ok());
        let error = mask_rcnn(config, 21).validate().unwrap_err();
        assert!(error.to_string().contains("at most 20 images"), "{error}");
        let oversized = crate::MaskRcnnConfig {
            base_channels: 256,
            mask_size: 56,
            training_samples: 512,
            ..Default::default()
        };
        assert!(mask_rcnn(oversized, 1).validate().is_err());
    }

    #[test]
    fn stored_configs_skip_the_training_roi_budget_but_keep_their_limits() {
        let config = crate::MaskRcnnConfig::default();
        let over_budget = mask_rcnn(config.clone(), 21);
        assert!(over_budget.validate().is_err());
        assert!(over_budget.validate_stored().is_ok());
        assert!(mask_rcnn(config.clone(), 33).validate_stored().is_err());
        let mut no_epochs = mask_rcnn(config, 1);
        no_epochs.epochs = 0;
        assert!(no_epochs.validate_stored().is_err());
    }

    #[test]
    fn instance_datasets_reject_crowded_images_and_oversized_worst_case_batches() {
        let config = crate::MaskRcnnConfig {
            input_channels: 1,
            classes: 1,
            ..Default::default()
        };
        let dataset = |targets: Vec<Vec<crate::InstanceTarget>>| TensorDataset {
            inputs: TensorData {
                shape: vec![targets.len(), 1, 16, 16],
                values: vec![0.0; targets.len() * 256],
            },
            targets: Targets::Instances { values: targets },
        };
        let crowded = dataset(vec![instances(MASK_RCNN_MAX_INSTANCES + 1, 16)]);
        let error = crowded
            .validate(&Recipe::MaskRcnn {
                config: config.clone(),
            })
            .unwrap_err();
        assert!(error.to_string().contains("at most 1024"), "{error}");
        assert!(
            mask_rcnn(config, 2)
                .validate_dataset(&dataset(vec![instances(3, 16), instances(1, 16)]))
                .is_ok()
        );
        // Sorted per-image mask elements are 32, 16 and 16; any two images can share a batch.
        let values = vec![instances(1, 4), instances(2, 4), instances(1, 4)];
        assert!(validate_instance_batches(&values, 1, 40).is_ok());
        let error = validate_instance_batches(&values, 2, 40).unwrap_err();
        assert!(error.to_string().contains("at most 1"), "{error}");
        assert!(validate_instance_batches(&values, 2, 48).is_ok());
        assert!(validate_instance_batches(&values, 8, 64).is_ok());
        assert!(validate_instance_batches(&values, 8, 63).is_err());
    }
}
