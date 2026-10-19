//! Native point forecasting for IBM Granite TinyTimeMixer r2 checkpoints.
// Portions adapted from ibm-granite/granite-tsfm, revision
// 3d92535ca4138648b9d1e2f3f1421b5dec4dc3aa.
// Copyright contributors to the TSFM project.
// SPDX-License-Identifier: Apache-2.0

use crate::types::validate_workspace;
use crate::weights::Weights;
use crate::{
    Error, ForecastCapabilities, ForecastFamily, ForecastRequest, ForecastResult,
    MAX_FORECAST_HORIZON, PointStatistic, Result, SeriesForecast,
};
use burn::tensor::{Device, Tensor, TensorData, activation};
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};

#[derive(Debug, Deserialize)]
struct Config {
    architectures: Vec<String>,
    model_type: String,
    context_length: usize,
    prediction_length: usize,
    patch_length: usize,
    patch_stride: usize,
    num_patches: usize,
    d_model: usize,
    num_layers: usize,
    expansion_factor: usize,
    adaptive_patching_levels: usize,
    gated_attn: bool,
    norm_eps: f32,
    norm_mlp: String,
    mode: String,
    scaling: String,
    loss: String,
    use_decoder: bool,
    decoder_d_model: usize,
    decoder_num_layers: usize,
    decoder_adaptive_patching_levels: usize,
    decoder_mode: String,
    decoder_raw_residual: bool,
    #[serde(default = "minimum_scale")]
    minimum_scale: f32,
    #[serde(flatten)]
    other: BTreeMap<String, serde_json::Value>,
}

fn minimum_scale() -> f32 {
    1e-5
}

impl Config {
    fn validate(&self) -> Result<()> {
        if self.architectures != ["TinyTimeMixerForPrediction"]
            || self.model_type != "tinytimemixer"
            || self.mode != "common_channel"
            || self.decoder_mode != "common_channel"
            || self.scaling != "std"
            || self.loss != "mse"
            || self.norm_mlp != "LayerNorm"
        {
            return Err(Error::Invalid("TinyTimeMixer supports MSE point checkpoints with common-channel mixing, standard scaling and LayerNorm".into()));
        }
        // Reject newer architectures instead of loading their overlapping weights as r2.
        for (name, value) in &self.other {
            let supported = match name.as_str() {
                "self_attn"
                | "use_positional_encoding"
                | "resolution_prefix_tuning"
                | "enable_forecast_channel_mixing"
                | "multi_scale"
                | "multi_quantile_head"
                | "use_register_context_gating"
                | "enable_base_norm_always" => value == false,
                "register_tokens" | "fft_length" => value == 0,
                "categorical_vocab_size_list"
                | "prediction_channel_indices"
                | "exogenous_channel_indices"
                | "prediction_filter_length"
                | "masked_context_length" => value.is_null(),
                "gate_mode" => value == "softmax",
                "scaling_dim" => value == 1,
                "keepdim" | "patch_last" => value == true,
                "num_input_channels" => value.as_u64().is_some_and(|n| n > 0),
                "d_model_scale"
                | "decoder_d_model_scale"
                | "distribution_output"
                | "dropout"
                | "head_dropout"
                | "fcm_context_length"
                | "fcm_gated_attn"
                | "fcm_mix_layers"
                | "fcm_prepend_past"
                | "fcm_use_mixer"
                | "frequency_token_vocab_size"
                | "init_embed"
                | "init_linear"
                | "init_processing"
                | "init_std"
                | "num_parallel_samples"
                | "positional_encoding_type"
                | "post_init"
                | "self_attn_heads"
                | "stride_ratio"
                | "torch_dtype"
                | "transformers_version"
                | "return_dict"
                | "output_hidden_states"
                | "_name_or_path" => true,
                _ => false,
            };
            if !supported {
                return Err(Error::Invalid(format!(
                    "Unsupported TinyTimeMixer configuration {name}={value}"
                )));
            }
        }
        if self.context_length == 0
            || self.patch_length == 0
            || self.patch_stride == 0
            || self.patch_stride != self.patch_length
            || self.context_length < self.patch_length
            || self.prediction_length == 0
            || self.d_model == 0
            || self.num_layers == 0
            || self.expansion_factor == 0
            || self.num_patches == 0
            || !self.norm_eps.is_finite()
            || self.norm_eps <= 0.0
            || !self.minimum_scale.is_finite()
            || self.minimum_scale <= 0.0
            || (self.use_decoder && (self.decoder_d_model == 0 || self.decoder_num_layers == 0))
            || self.num_layers > 1024
            || self.decoder_num_layers > 1024
            || self.adaptive_patching_levels > 32
            || self.decoder_adaptive_patching_levels > 32
        {
            return Err(Error::Invalid(
                "Invalid TinyTimeMixer dimensions or normalization configuration".into(),
            ));
        }
        let patches = (self.context_length - self.patch_length) / self.patch_stride + 1;
        if patches != self.num_patches {
            return Err(Error::Invalid(
                "TinyTimeMixer num_patches disagrees with context and patch stride".into(),
            ));
        }
        validate_workspace(&[self.context_length], "TinyTimeMixer context")?;
        validate_workspace(
            &[self.num_patches, self.patch_length],
            "TinyTimeMixer patch input",
        )?;
        validate_workspace(
            &[self.patch_length, self.d_model],
            "TinyTimeMixer patch projection",
        )?;
        self.validate_block(self.d_model, self.adaptive_patching_levels, self.num_layers)?;
        let head_dim = if self.use_decoder {
            self.validate_block(
                self.decoder_d_model,
                self.decoder_adaptive_patching_levels,
                self.decoder_num_layers,
            )?;
            validate_workspace(
                &[self.d_model, self.decoder_d_model],
                "TinyTimeMixer decoder adapter",
            )?;
            validate_workspace(
                &[self.patch_length, self.decoder_d_model],
                "TinyTimeMixer decoder residual",
            )?;
            self.decoder_d_model
        } else {
            self.d_model
        };
        validate_workspace(
            &[self.num_patches, head_dim, self.prediction_length],
            "TinyTimeMixer forecast head",
        )
    }

    fn factor(&self, width: usize, level: usize) -> Result<usize> {
        let factor = 1usize.checked_shl(level as u32).ok_or_else(|| {
            Error::Invalid("TinyTimeMixer adaptive patch dimensions overflow".into())
        })?;
        let factor = if width / factor <= 4 { 1 } else { factor };
        if width % factor != 0 {
            return Err(Error::Invalid(
                "TinyTimeMixer hidden dimension must be divisible by each adaptive patch factor"
                    .into(),
            ));
        }
        Ok(factor)
    }

    fn validate_block(&self, width: usize, levels: usize, layers: usize) -> Result<()> {
        for level in 0..levels.max(1) {
            let factor = self.factor(width, level)?;
            validate_workspace(
                &[self.num_patches, factor],
                "TinyTimeMixer adaptive patches",
            )?;
            let patches = self.num_patches * factor;
            let features = width / factor;
            validate_workspace(
                &[patches, features, self.expansion_factor],
                "TinyTimeMixer hidden workspace",
            )?;
            validate_workspace(
                &[
                    patches,
                    patches,
                    self.expansion_factor,
                    layers,
                    levels.max(1),
                ],
                "TinyTimeMixer patch mixers",
            )?;
            validate_workspace(
                &[
                    features,
                    features,
                    self.expansion_factor,
                    layers,
                    levels.max(1),
                ],
                "TinyTimeMixer feature mixers",
            )?;
        }
        Ok(())
    }

    fn shapes(&self) -> BTreeMap<String, Vec<usize>> {
        let mut shapes = BTreeMap::new();
        linear_shapes(
            &mut shapes,
            "backbone.encoder.patcher",
            self.patch_length,
            self.d_model,
        );
        self.block_shapes(
            &mut shapes,
            "backbone.encoder.mlp_mixer_encoder",
            self.d_model,
            self.adaptive_patching_levels,
            self.num_layers,
        );
        let head_dim = if self.use_decoder {
            if self.d_model != self.decoder_d_model {
                linear_shapes(
                    &mut shapes,
                    "decoder.adapter",
                    self.d_model,
                    self.decoder_d_model,
                );
            }
            if self.decoder_raw_residual {
                linear_shapes(
                    &mut shapes,
                    "decoder.decoder_raw_embedding",
                    self.patch_length,
                    self.decoder_d_model,
                );
            }
            self.block_shapes(
                &mut shapes,
                "decoder.decoder_block",
                self.decoder_d_model,
                self.decoder_adaptive_patching_levels,
                self.decoder_num_layers,
            );
            self.decoder_d_model
        } else {
            self.d_model
        };
        linear_shapes(
            &mut shapes,
            "head.base_forecast_block",
            self.num_patches * head_dim,
            self.prediction_length,
        );
        shapes
    }

    fn block_shapes(
        &self,
        shapes: &mut BTreeMap<String, Vec<usize>>,
        prefix: &str,
        width: usize,
        levels: usize,
        layers: usize,
    ) {
        for group in 0..levels.max(1) {
            let factor = self
                .factor(width, if levels == 0 { 0 } else { levels - group - 1 })
                .unwrap();
            let patches = self.num_patches * factor;
            let features = width / factor;
            for layer in 0..layers {
                let prefix = if levels == 0 {
                    format!("{prefix}.mixers.{layer}")
                } else {
                    format!("{prefix}.mixers.{group}.mixer_layers.{layer}")
                };
                for (name, size) in [("patch_mixer", patches), ("feature_mixer", features)] {
                    if name == "patch_mixer" && patches == 1 {
                        continue;
                    }
                    let name = format!("{prefix}.{name}");
                    shapes.insert(format!("{name}.norm.norm.weight"), vec![features]);
                    shapes.insert(format!("{name}.norm.norm.bias"), vec![features]);
                    linear_shapes(
                        shapes,
                        &format!("{name}.mlp.fc1"),
                        size,
                        size * self.expansion_factor,
                    );
                    linear_shapes(
                        shapes,
                        &format!("{name}.mlp.fc2"),
                        size * self.expansion_factor,
                        size,
                    );
                    if self.gated_attn {
                        linear_shapes(
                            shapes,
                            &format!("{name}.gating_block.attn_layer"),
                            size,
                            size,
                        );
                    }
                }
            }
        }
    }
}

fn linear_shapes(
    shapes: &mut BTreeMap<String, Vec<usize>>,
    prefix: &str,
    input: usize,
    output: usize,
) {
    shapes.insert(format!("{prefix}.weight"), vec![output, input]);
    shapes.insert(format!("{prefix}.bias"), vec![output]);
}

struct Linear {
    weight: Tensor<2>,
    bias: Tensor<1>,
}
impl Linear {
    fn load(weights: &Weights, prefix: &str, device: &Device) -> Result<Self> {
        Ok(Self {
            weight: weights
                .tensor::<2>(&format!("{prefix}.weight"), device)?
                .transpose(),
            bias: weights.tensor(&format!("{prefix}.bias"), device)?,
        })
    }
    fn forward(&self, input: Tensor<3>) -> Tensor<3> {
        input.matmul(self.weight.clone().unsqueeze::<3>()) + self.bias.clone().unsqueeze::<3>()
    }
}

struct Mixer {
    norm_weight: Tensor<1>,
    norm_bias: Tensor<1>,
    fc1: Linear,
    fc2: Linear,
    gate: Option<Linear>,
    epsilon: f32,
    patch_axis: bool,
}
impl Mixer {
    fn load(
        w: &Weights,
        name: &str,
        config: &Config,
        patch_axis: bool,
        device: &Device,
    ) -> Result<Self> {
        Ok(Self {
            norm_weight: w.tensor(&format!("{name}.norm.norm.weight"), device)?,
            norm_bias: w.tensor(&format!("{name}.norm.norm.bias"), device)?,
            fc1: Linear::load(w, &format!("{name}.mlp.fc1"), device)?,
            fc2: Linear::load(w, &format!("{name}.mlp.fc2"), device)?,
            gate: config
                .gated_attn
                .then(|| Linear::load(w, &format!("{name}.gating_block.attn_layer"), device))
                .transpose()?,
            epsilon: config.norm_eps,
            patch_axis,
        })
    }
    fn forward(&self, input: Tensor<3>) -> Tensor<3> {
        let centered = input.clone() - input.clone().mean_dim(2);
        let variance = centered.clone().square().mean_dim(2);
        let mut hidden = centered / (variance + self.epsilon).sqrt()
            * self.norm_weight.clone().unsqueeze::<3>()
            + self.norm_bias.clone().unsqueeze::<3>();
        if self.patch_axis {
            hidden = hidden.swap_dims(1, 2);
        }
        hidden = self.fc2.forward(activation::gelu(self.fc1.forward(hidden)));
        if let Some(gate) = &self.gate {
            hidden = hidden.clone() * activation::softmax(gate.forward(hidden), 2);
        }
        if self.patch_axis {
            hidden = hidden.swap_dims(1, 2);
        }
        input + hidden
    }
}

struct Layer {
    patch: Option<Mixer>,
    feature: Mixer,
}
struct Group {
    factor: usize,
    layers: Vec<Layer>,
}
struct Block {
    groups: Vec<Group>,
}
impl Block {
    fn load(
        w: &Weights,
        prefix: &str,
        config: &Config,
        width: usize,
        levels: usize,
        layers: usize,
        device: &Device,
    ) -> Result<Self> {
        let mut groups = Vec::new();
        for group in 0..levels.max(1) {
            let factor = config.factor(width, if levels == 0 { 0 } else { levels - group - 1 })?;
            let mut entries = Vec::new();
            for layer in 0..layers {
                let name = if levels == 0 {
                    format!("{prefix}.mixers.{layer}")
                } else {
                    format!("{prefix}.mixers.{group}.mixer_layers.{layer}")
                };
                entries.push(Layer {
                    patch: (config.num_patches * factor > 1)
                        .then(|| {
                            Mixer::load(w, &format!("{name}.patch_mixer"), config, true, device)
                        })
                        .transpose()?,
                    feature: Mixer::load(
                        w,
                        &format!("{name}.feature_mixer"),
                        config,
                        false,
                        device,
                    )?,
                });
            }
            groups.push(Group {
                factor,
                layers: entries,
            });
        }
        Ok(Self { groups })
    }
    fn forward(&self, mut hidden: Tensor<3>) -> Tensor<3> {
        let [batch, patches, width] = hidden.dims();
        for group in &self.groups {
            hidden = hidden.reshape([batch, patches * group.factor, width / group.factor]);
            for layer in &group.layers {
                if let Some(patch) = &layer.patch {
                    hidden = patch.forward(hidden);
                }
                hidden = layer.feature.forward(hidden);
            }
            hidden = hidden.reshape([batch, patches, width]);
        }
        hidden
    }
}

pub struct TinyTimeMixer {
    config: Config,
    device: Device,
    patcher: Linear,
    encoder: Block,
    adapter: Option<Linear>,
    raw_embedding: Option<Linear>,
    decoder: Option<Block>,
    head: Linear,
}

impl TinyTimeMixer {
    pub fn load(directory: &Path, device: &Device) -> Result<Self> {
        let config: Config =
            serde_json::from_slice(&std::fs::read(directory.join("config.json"))?)?;
        config.validate()?;
        let weights = Weights::open(directory)?;
        let shapes = config.shapes();
        // Validate the entire checkpoint before transferring any weight to the backend.
        for (name, shape) in &shapes {
            if weights.shape(name)? != *shape {
                return Err(Error::Invalid(format!(
                    "TinyTimeMixer tensor {name} must have shape {shape:?}"
                )));
            }
        }
        for name in weights.names() {
            if !shapes.contains_key(name) {
                return Err(Error::Invalid(format!(
                    "Unexpected TinyTimeMixer tensor {name}"
                )));
            }
        }
        let patcher = Linear::load(&weights, "backbone.encoder.patcher", device)?;
        let encoder = Block::load(
            &weights,
            "backbone.encoder.mlp_mixer_encoder",
            &config,
            config.d_model,
            config.adaptive_patching_levels,
            config.num_layers,
            device,
        )?;
        let adapter = (config.use_decoder && config.d_model != config.decoder_d_model)
            .then(|| Linear::load(&weights, "decoder.adapter", device))
            .transpose()?;
        let raw_embedding = (config.use_decoder && config.decoder_raw_residual)
            .then(|| Linear::load(&weights, "decoder.decoder_raw_embedding", device))
            .transpose()?;
        let decoder = config
            .use_decoder
            .then(|| {
                Block::load(
                    &weights,
                    "decoder.decoder_block",
                    &config,
                    config.decoder_d_model,
                    config.decoder_adaptive_patching_levels,
                    config.decoder_num_layers,
                    device,
                )
            })
            .transpose()?;
        let head = Linear::load(&weights, "head.base_forecast_block", device)?;
        Ok(Self {
            config,
            device: device.clone(),
            patcher,
            encoder,
            adapter,
            raw_embedding,
            decoder,
            head,
        })
    }

    pub fn capabilities(&self) -> ForecastCapabilities {
        ForecastCapabilities {
            family: ForecastFamily::TinyTimeMixer,
            max_context: self.config.context_length,
            max_horizon: Some(self.config.prediction_length.min(MAX_FORECAST_HORIZON)),
            native_horizon: self.config.prediction_length,
            quantile_levels: vec![],
            joint_multivariate: false,
            past_covariates: false,
            known_covariates: false,
        }
    }

    pub fn forecast(&self, request: &ForecastRequest) -> Result<ForecastResult> {
        request.validate_for_quantiles(0)?;
        if !request.quantile_levels.is_empty() {
            return Err(Error::Invalid("This TinyTimeMixer checkpoint produces point forecasts only; quantile_levels must be empty".into()));
        }
        if request.horizon > self.config.prediction_length {
            return Err(Error::Invalid(format!(
                "TinyTimeMixer horizon exceeds its {}-step prediction head",
                self.config.prediction_length
            )));
        }
        let mut forecasts = Vec::with_capacity(request.series.len());
        for series in &request.series {
            if series.targets.len() != 1 {
                return Err(Error::Invalid("TinyTimeMixer common-channel checkpoints forecast one target per independent series; split jointly modeled targets into separate series".into()));
            }
            if !series.past_covariates.is_empty() || !series.known_covariates.is_empty() {
                return Err(Error::Invalid(
                    "This TinyTimeMixer checkpoint does not support covariates".into(),
                ));
            }
            let (values, observed) = padded_history(
                &series.targets[0],
                series.observed_mask.as_ref().map(|m| m[0].as_slice()),
                self.config.context_length,
            );
            let input = Tensor::<3>::from_data(
                TensorData::new(values, [1, self.config.context_length, 1]),
                &self.device,
            );
            let mask = Tensor::<3>::from_data(
                TensorData::new(observed, [1, self.config.context_length, 1]),
                &self.device,
            );
            let count = mask.clone().sum_dim(1).clamp_min(1.0);
            let loc = (input.clone() * mask.clone()).sum_dim(1) / count.clone();
            let variance = ((input.clone() - loc.clone()) * mask).square().sum_dim(1) / count;
            let scale = (variance + self.config.minimum_scale).sqrt();
            let normalized = (input - loc.clone()) / scale.clone();
            let length = self.config.patch_length;
            let stride = self.config.patch_stride;
            let patches = self.config.num_patches;
            let start = self.config.context_length - (length + stride * (patches - 1));
            let patch_input = Tensor::cat(
                (0..patches)
                    .map(|i| {
                        normalized
                            .clone()
                            .slice([0..1, start + i * stride..start + i * stride + length, 0..1])
                            .reshape([1, 1, length])
                    })
                    .collect(),
                1,
            );
            let mut hidden = self
                .encoder
                .forward(self.patcher.forward(patch_input.clone()));
            if let Some(adapter) = &self.adapter {
                hidden = adapter.forward(hidden);
            }
            if let Some(embedding) = &self.raw_embedding {
                hidden = hidden + embedding.forward(patch_input);
            }
            if let Some(decoder) = &self.decoder {
                hidden = decoder.forward(hidden);
            }
            let [_, p, d] = hidden.dims();
            let output = self.head.forward(hidden.reshape([1, 1, p * d])) * scale + loc;
            let point = output
                .slice([0..1, 0..1, 0..request.horizon])
                .into_data()
                .try_to_vec::<f32>()
                .map_err(|e| Error::Tensor(format!("TinyTimeMixer output: {e}")))?;
            if point.iter().any(|v| !v.is_finite()) {
                return Err(Error::Tensor(
                    "TinyTimeMixer produced non-finite forecasts".into(),
                ));
            }
            forecasts.push(SeriesForecast {
                id: series.id.clone(),
                point: vec![point],
                quantiles: vec![vec![vec![]; request.horizon]],
            });
        }
        Ok(ForecastResult {
            series: forecasts,
            quantile_levels: vec![],
            point_statistic: PointStatistic::Mean,
        })
    }
}

fn padded_history(values: &[f32], mask: Option<&[bool]>, context: usize) -> (Vec<f32>, Vec<f32>) {
    let mut padded = vec![0.0; context];
    let mut observed = vec![0.0; context];
    let kept = values.len().min(context);
    for i in 0..kept {
        let source = values.len() - kept + i;
        if mask.is_none_or(|mask| mask[source]) {
            padded[context - kept + i] = values[source];
            observed[context - kept + i] = 1.0;
        }
    }
    (padded, observed)
}

#[cfg(all(test, feature = "cpu"))]
#[path = "tiny_time_mixer_tests.rs"]
mod tests;
