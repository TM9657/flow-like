//! Native inference for Chronos-2 checkpoints from `chronos-forecasting`.
//!
//! The architecture and long-horizon quantile propagation follow Amazon's
//! Apache-2.0 implementation in `src/chronos/chronos2`.
// Portions adapted from chronos-forecasting.
// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::types::validate_workspace;
use crate::weights::Weights;
use crate::{Error, ForecastRequest, ForecastResult, PointStatistic, Result, SeriesForecast};
use burn::tensor::{Device, Tensor, TensorData, activation};
use serde::Deserialize;
use std::path::Path;

const UNROLLED_QUANTILES: [f32; 9] = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9];

#[derive(Debug, Deserialize)]
struct Config {
    d_model: usize,
    d_kv: usize,
    d_ff: usize,
    num_layers: usize,
    num_heads: usize,
    #[serde(default = "epsilon")]
    layer_norm_epsilon: f64,
    #[serde(default = "rope_theta")]
    rope_theta: f32,
    #[serde(default = "relu")]
    feed_forward_proj: String,
    #[serde(default)]
    is_gated_act: bool,
    #[serde(default = "reg_id")]
    reg_token_id: usize,
    chronos_config: ForecastConfig,
}

#[derive(Debug, Deserialize)]
struct ForecastConfig {
    context_length: usize,
    input_patch_size: usize,
    input_patch_stride: usize,
    output_patch_size: usize,
    quantiles: Vec<f32>,
    #[serde(default)]
    use_reg_token: bool,
    #[serde(default)]
    use_arcsinh: bool,
    #[serde(default = "one")]
    max_output_patches: usize,
    time_encoding_scale: Option<usize>,
}

fn epsilon() -> f64 {
    1e-6
}
fn rope_theta() -> f32 {
    10_000.0
}
fn relu() -> String {
    "relu".into()
}
fn reg_id() -> usize {
    1
}
fn one() -> usize {
    1
}

impl Config {
    fn validate(&self) -> Result<()> {
        let c = &self.chronos_config;
        if self.d_model == 0
            || self.d_kv == 0
            || self.d_kv % 2 != 0
            || self.d_ff == 0
            || self.num_heads == 0
            || self.num_layers == 0
            || c.context_length == 0
            || c.input_patch_size == 0
            || c.input_patch_stride != c.input_patch_size
            || c.output_patch_size != c.input_patch_size
            || c.max_output_patches == 0
            || c.time_encoding_scale == Some(0)
            || !self.layer_norm_epsilon.is_finite()
            || self.layer_norm_epsilon <= 0.0
            || !self.rope_theta.is_finite()
            || self.rope_theta <= 0.0
        {
            return Err(Error::Invalid(
                "Invalid Chronos-2 dimensions, patch stride, or normalization configuration".into(),
            ));
        }
        if c.use_reg_token && self.reg_token_id != 1 {
            return Err(Error::Invalid(
                "Chronos-2 requires registration token ID 1".into(),
            ));
        }
        let products = [
            [c.input_patch_size, 3, self.d_ff],
            [c.input_patch_size, 3, self.d_model],
            [c.output_patch_size, c.quantiles.len(), self.d_ff],
            [c.output_patch_size, c.quantiles.len(), self.d_model],
            [c.output_patch_size, c.max_output_patches, 1],
            [self.num_heads, self.d_kv, self.d_model],
            [self.d_model, self.d_ff, 1],
        ];
        if products.iter().any(|dimensions| {
            dimensions
                .iter()
                .try_fold(1_usize, |size, dim| size.checked_mul(*dim))
                .is_none()
        }) || c.context_length.checked_add(c.input_patch_size).is_none()
        {
            return Err(Error::Invalid(
                "Chronos-2 configuration dimensions overflow".into(),
            ));
        }
        if self.is_gated_act
            || !matches!(
                self.feed_forward_proj.as_str(),
                "relu" | "gelu" | "gelu_new" | "silu"
            )
        {
            return Err(Error::Invalid(format!(
                "Unsupported Chronos-2 activation: {}",
                self.feed_forward_proj
            )));
        }
        if c.quantiles.is_empty()
            || c.quantiles
                .iter()
                .any(|q| !q.is_finite() || *q <= 0.0 || *q >= 1.0)
            || c.quantiles.windows(2).any(|w| w[0] >= w[1])
            || !c.quantiles.contains(&0.5)
        {
            return Err(Error::Invalid(
                "Chronos-2 quantiles must be sorted, unique, inside (0, 1), and contain 0.5".into(),
            ));
        }
        Ok(())
    }

    fn validate_forecast_workspace(
        &self,
        channels: usize,
        history: usize,
        horizon: usize,
    ) -> Result<()> {
        let c = &self.chronos_config;
        let patches = horizon.div_ceil(c.output_patch_size);
        validate_workspace(
            &[channels, c.quantiles.len(), patches, c.output_patch_size],
            "Chronos-2 native quantile output including covariates",
        )?;
        let padded_horizon = patches * c.output_patch_size;
        validate_workspace(&[channels, horizon], "Chronos-2 future values")?;
        let native_horizon = c.max_output_patches * c.output_patch_size;
        if horizon > native_horizon {
            if UNROLLED_QUANTILES.iter().any(|q| !c.quantiles.contains(q)) {
                return Err(Error::Invalid(
                    "Chronos-2 long-horizon inference requires native decile quantiles".into(),
                ));
            }
            let retained_history = history
                .min(c.context_length)
                .checked_add(padded_horizon)
                .ok_or_else(|| Error::Invalid("Chronos-2 history dimensions overflow".into()))?
                .min(c.context_length);
            validate_workspace(
                &[UNROLLED_QUANTILES.len(), channels, retained_history],
                "Chronos-2 propagated quantile histories",
            )?;
            validate_workspace(
                &[
                    UNROLLED_QUANTILES.len(),
                    channels,
                    c.quantiles.len(),
                    (padded_horizon - native_horizon).min(native_horizon),
                ],
                "Chronos-2 propagated quantile forecasts",
            )?;
        }
        Ok(())
    }

    fn validate_step_workspace(
        &self,
        channels: usize,
        history: usize,
        output_patches: usize,
    ) -> Result<(usize, usize)> {
        let c = &self.chronos_config;
        let context_patches = history.min(c.context_length).div_ceil(c.input_patch_size);
        let tokens = context_patches
            .checked_add(usize::from(c.use_reg_token))
            .and_then(|n| n.checked_add(output_patches))
            .ok_or_else(|| Error::Invalid("Chronos-2 token dimensions overflow".into()))?;
        validate_workspace(
            &[channels, self.num_heads, tokens, tokens],
            "Chronos-2 temporal attention",
        )?;
        validate_workspace(
            &[tokens, self.num_heads, channels, channels],
            "Chronos-2 group attention",
        )?;
        validate_workspace(&[channels, tokens, self.d_model], "Chronos-2 hidden states")?;
        validate_workspace(
            &[channels, tokens, self.num_heads, self.d_kv],
            "Chronos-2 attention projections",
        )?;
        validate_workspace(
            &[channels, tokens, self.d_ff],
            "Chronos-2 feedforward states",
        )?;
        validate_workspace(
            &[channels, tokens, 3, c.input_patch_size],
            "Chronos-2 patch features",
        )?;
        validate_workspace(
            &[
                channels,
                c.quantiles.len(),
                output_patches,
                c.output_patch_size,
            ],
            "Chronos-2 native prediction block",
        )?;
        Ok((context_patches, tokens))
    }
}

struct Linear {
    weight: Tensor<2>,
    bias: Option<Tensor<1>>,
}

impl Linear {
    fn load(
        w: &Weights,
        prefix: &str,
        input: usize,
        output: usize,
        bias: bool,
        device: &Device,
    ) -> Result<Self> {
        let weight =
            tensor::<2>(w, &format!("{prefix}.weight"), [output, input], device)?.transpose();
        let bias = if bias {
            Some(tensor::<1>(w, &format!("{prefix}.bias"), [output], device)?)
        } else {
            None
        };
        Ok(Self { weight, bias })
    }

    fn forward(&self, x: Tensor<3>) -> Tensor<3> {
        let [b, t, d] = x.dims();
        let output = self.weight.dims()[1];
        let mut y = x.reshape([b * t, d]).matmul(self.weight.clone());
        if let Some(bias) = &self.bias {
            y = y + bias.clone().reshape([1, output]);
        }
        y.reshape([b, t, output])
    }
}

fn tensor<const D: usize>(
    w: &Weights,
    name: &str,
    shape: [usize; D],
    device: &Device,
) -> Result<Tensor<D>> {
    let actual = w.shape(name)?;
    if actual != shape {
        return Err(Error::Invalid(format!(
            "Chronos-2 tensor {name}: expected {shape:?}, got {:?}",
            actual
        )));
    }
    w.tensor::<D>(name, device)
}

struct RmsNorm {
    weight: Tensor<1>,
    epsilon: f64,
}

impl RmsNorm {
    fn load(w: &Weights, prefix: &str, config: &Config, device: &Device) -> Result<Self> {
        Ok(Self {
            weight: tensor::<1>(w, &format!("{prefix}.weight"), [config.d_model], device)?,
            epsilon: config.layer_norm_epsilon,
        })
    }

    fn forward(&self, x: Tensor<3>) -> Tensor<3> {
        let variance = x.clone().powf_scalar(2.0).mean_dim(2);
        let d = self.weight.dims()[0];
        x * (variance + self.epsilon).sqrt().recip() * self.weight.clone().reshape([1, 1, d])
    }
}

fn activate(x: Tensor<3>, name: &str) -> Tensor<3> {
    match name {
        "relu" => activation::relu(x),
        "gelu" => activation::gelu(x),
        "gelu_new" => {
            let inner = (x.clone() + x.clone().powf_scalar(3.0) * 0.044715)
                * (2.0_f64 / std::f64::consts::PI).sqrt();
            x * (inner.tanh() + 1.0) * 0.5
        }
        "silu" => x.clone() * activation::sigmoid(x),
        _ => unreachable!("activation validated during loading"),
    }
}

struct ResidualEmbedding {
    hidden: Linear,
    output: Linear,
    residual: Linear,
}

impl ResidualEmbedding {
    fn load(
        w: &Weights,
        prefix: &str,
        input: usize,
        hidden: usize,
        output: usize,
        device: &Device,
    ) -> Result<Self> {
        Ok(Self {
            hidden: Linear::load(
                w,
                &format!("{prefix}.hidden_layer"),
                input,
                hidden,
                true,
                device,
            )?,
            output: Linear::load(
                w,
                &format!("{prefix}.output_layer"),
                hidden,
                output,
                true,
                device,
            )?,
            residual: Linear::load(
                w,
                &format!("{prefix}.residual_layer"),
                input,
                output,
                true,
                device,
            )?,
        })
    }

    fn forward(&self, x: Tensor<3>, act: &str) -> Tensor<3> {
        self.output
            .forward(activate(self.hidden.forward(x.clone()), act))
            + self.residual.forward(x)
    }
}

struct Attention {
    norm: RmsNorm,
    q: Linear,
    k: Linear,
    v: Linear,
    o: Linear,
    heads: usize,
    head_dim: usize,
}

impl Attention {
    fn load(w: &Weights, prefix: &str, c: &Config, device: &Device) -> Result<Self> {
        let inner = c.num_heads * c.d_kv;
        let attn = format!("{prefix}.self_attention");
        Ok(Self {
            norm: RmsNorm::load(w, &format!("{prefix}.layer_norm"), c, device)?,
            q: Linear::load(w, &format!("{attn}.q"), c.d_model, inner, false, device)?,
            k: Linear::load(w, &format!("{attn}.k"), c.d_model, inner, false, device)?,
            v: Linear::load(w, &format!("{attn}.v"), c.d_model, inner, false, device)?,
            o: Linear::load(w, &format!("{attn}.o"), inner, c.d_model, false, device)?,
            heads: c.num_heads,
            head_dim: c.d_kv,
        })
    }

    fn forward(&self, x: Tensor<3>, mask: Tensor<4>, rope: Option<&Rotary>) -> Tensor<3> {
        let [b, t, _] = x.dims();
        let normalized = self.norm.forward(x.clone());
        let shape = |v: Tensor<3>| v.reshape([b, t, self.heads, self.head_dim]).swap_dims(1, 2);
        let mut q = shape(self.q.forward(normalized.clone()));
        let mut k = shape(self.k.forward(normalized.clone()));
        let v = shape(self.v.forward(normalized));
        if let Some(rope) = rope {
            q = rope.apply(q);
            k = rope.apply(k);
        }
        // Chronos-2 deliberately does not divide attention scores by sqrt(head_dim).
        let probabilities = activation::softmax(q.matmul(k.swap_dims(2, 3)) + mask, 3);
        let output =
            probabilities
                .matmul(v)
                .swap_dims(1, 2)
                .reshape([b, t, self.heads * self.head_dim]);
        x + self.o.forward(output)
    }
}

struct Rotary {
    cos: Tensor<4>,
    sin: Tensor<4>,
}

impl Rotary {
    fn new(length: usize, dim: usize, theta: f32, device: &Device) -> Self {
        let mut cos = Vec::with_capacity(length * dim);
        let mut sin = Vec::with_capacity(length * dim);
        for p in 0..length {
            for i in 0..dim {
                let angle = p as f32 / theta.powf((2 * (i % (dim / 2))) as f32 / dim as f32);
                cos.push(angle.cos());
                sin.push(angle.sin());
            }
        }
        Self {
            cos: Tensor::from_data(TensorData::new(cos, [1, 1, length, dim]), device),
            sin: Tensor::from_data(TensorData::new(sin, [1, 1, length, dim]), device),
        }
    }

    fn apply(&self, x: Tensor<4>) -> Tensor<4> {
        let [b, h, t, d] = x.dims();
        let first = x.clone().slice([0..b, 0..h, 0..t, 0..d / 2]);
        let last = x.clone().slice([0..b, 0..h, 0..t, d / 2..d]);
        x * self.cos.clone() + Tensor::cat(vec![-last, first], 3) * self.sin.clone()
    }
}

struct Block {
    time: Attention,
    group: Attention,
    norm: RmsNorm,
    wi: Linear,
    wo: Linear,
}

impl Block {
    fn load(w: &Weights, index: usize, c: &Config, device: &Device) -> Result<Self> {
        let p = format!("encoder.block.{index}.layer");
        Ok(Self {
            time: Attention::load(w, &format!("{p}.0"), c, device)?,
            group: Attention::load(w, &format!("{p}.1"), c, device)?,
            norm: RmsNorm::load(w, &format!("{p}.2.layer_norm"), c, device)?,
            wi: Linear::load(
                w,
                &format!("{p}.2.mlp.wi"),
                c.d_model,
                c.d_ff,
                false,
                device,
            )?,
            wo: Linear::load(
                w,
                &format!("{p}.2.mlp.wo"),
                c.d_ff,
                c.d_model,
                false,
                device,
            )?,
        })
    }

    fn forward(
        &self,
        x: Tensor<3>,
        time_mask: Tensor<4>,
        group_mask: Tensor<4>,
        rope: &Rotary,
        act: &str,
    ) -> Tensor<3> {
        let x = self.time.forward(x, time_mask, Some(rope));
        let x = self
            .group
            .forward(x.swap_dims(0, 1), group_mask, None)
            .swap_dims(0, 1);
        x.clone()
            + self
                .wo
                .forward(activate(self.wi.forward(self.norm.forward(x)), act))
    }
}

/// Chronos-2's native numeric forecasting implementation.
///
/// Targets and covariates in each series share group attention. Different series
/// remain independent, matching upstream's default `cross_learning=false`.
pub struct Chronos2 {
    config: Config,
    input: ResidualEmbedding,
    output: ResidualEmbedding,
    reg: Option<Tensor<3>>,
    blocks: Vec<Block>,
    norm: RmsNorm,
    device: Device,
}

// Internal predictions retain covariates for autoregressive propagation: [channel, quantile, time].
type Predictions = Vec<Vec<Vec<f32>>>;

impl Chronos2 {
    pub fn capabilities(&self) -> crate::ForecastCapabilities {
        let c = &self.config.chronos_config;
        let native_horizon = c.max_output_patches * c.output_patch_size;
        crate::ForecastCapabilities {
            family: crate::ForecastFamily::Chronos2,
            max_context: c.context_length,
            max_horizon: Some(
                if UNROLLED_QUANTILES.iter().any(|q| !c.quantiles.contains(q)) {
                    native_horizon.min(crate::types::MAX_FORECAST_HORIZON)
                } else {
                    crate::types::MAX_FORECAST_HORIZON
                },
            ),
            native_horizon,
            quantile_levels: c.quantiles.clone(),
            joint_multivariate: true,
            past_covariates: true,
            known_covariates: true,
        }
    }

    pub fn load(directory: &Path, device: &Device) -> Result<Self> {
        let config: Config =
            serde_json::from_slice(&std::fs::read(directory.join("config.json"))?)?;
        config.validate()?;
        let weights = Weights::open(directory)?;
        Self::from_weights(config, &weights, device)
    }

    fn from_weights(config: Config, w: &Weights, device: &Device) -> Result<Self> {
        let c = &config.chronos_config;
        let input = ResidualEmbedding::load(
            w,
            "input_patch_embedding",
            c.input_patch_size * 3,
            config.d_ff,
            config.d_model,
            device,
        )?;
        let output = ResidualEmbedding::load(
            w,
            "output_patch_embedding",
            config.d_model,
            config.d_ff,
            c.quantiles.len() * c.output_patch_size,
            device,
        )?;
        let reg = if c.use_reg_token {
            let shared = tensor::<2>(w, "shared.weight", [2, config.d_model], device)?;
            let width = config.d_model;
            Some(
                shared
                    .slice([config.reg_token_id..config.reg_token_id + 1, 0..width])
                    .reshape([1, 1, width]),
            )
        } else {
            None
        };
        let blocks = (0..config.num_layers)
            .map(|i| Block::load(w, i, &config, device))
            .collect::<Result<Vec<_>>>()?;
        let norm = RmsNorm::load(w, "encoder.final_layer_norm", &config, device)?;
        Ok(Self {
            config,
            input,
            output,
            reg,
            blocks,
            norm,
            device: device.clone(),
        })
    }

    pub fn forecast(&self, request: &ForecastRequest) -> Result<ForecastResult> {
        request.validate_for_quantiles(self.config.chronos_config.quantiles.len())?;
        let native = &self.config.chronos_config.quantiles;
        let levels = if request.quantile_levels.is_empty() {
            native.clone()
        } else {
            request.quantile_levels.clone()
        };
        if levels
            .iter()
            .any(|q| *q < native[0] || *q > native[native.len() - 1])
        {
            return Err(Error::Invalid(format!(
                "Chronos-2 supports quantiles between {} and {}",
                native[0],
                native[native.len() - 1]
            )));
        }
        let median = native
            .iter()
            .position(|q| *q == 0.5)
            .expect("validated median");
        let mut results = Vec::with_capacity(request.series.len());
        for series in &request.series {
            let history_length = series.targets[0].len();
            let channels = series
                .targets
                .len()
                .checked_add(series.past_covariates.len())
                .and_then(|n| n.checked_add(series.known_covariates.len()))
                .ok_or_else(|| Error::Invalid("Chronos-2 channel dimensions overflow".into()))?;
            self.config
                .validate_forecast_workspace(channels, history_length, request.horizon)?;
            let mut context = series.targets.clone();
            if let Some(mask) = &series.observed_mask {
                for (values, observed) in context.iter_mut().zip(mask) {
                    for (value, is_observed) in values.iter_mut().zip(observed) {
                        if !is_observed {
                            *value = f32::NAN;
                        }
                    }
                }
            }
            context.extend(series.past_covariates.iter().cloned());
            let mut future = vec![vec![f32::NAN; request.horizon]; context.len()];
            for known in &series.known_covariates {
                context.push(known[..history_length].to_vec());
                future.push(known[history_length..].to_vec());
            }
            let prediction = self.predict(context, &future, request.horizon)?;
            let mut point = Vec::with_capacity(series.targets.len());
            let mut quantiles = Vec::with_capacity(series.targets.len());
            for channel in prediction.iter().take(series.targets.len()) {
                point.push(channel[median][..request.horizon].to_vec());
                quantiles.push(
                    (0..request.horizon)
                        .map(|t| {
                            levels
                                .iter()
                                .map(|q| interpolate(native, *q, |i| channel[i][t]))
                                .collect()
                        })
                        .collect(),
                );
            }
            results.push(SeriesForecast {
                id: series.id.clone(),
                point,
                quantiles,
            });
        }
        Ok(ForecastResult {
            series: results,
            quantile_levels: levels,
            point_statistic: PointStatistic::Median,
        })
    }

    fn predict(
        &self,
        mut context: Vec<Vec<f32>>,
        future: &[Vec<f32>],
        horizon: usize,
    ) -> Result<Predictions> {
        let c = &self.config.chronos_config;
        for row in &mut context {
            if row.len() > c.context_length {
                row.drain(..row.len() - c.context_length);
            }
        }
        let native_horizon = c.max_output_patches * c.output_patch_size;
        let patches = |remaining: usize| {
            remaining
                .div_ceil(c.output_patch_size)
                .min(c.max_output_patches)
        };
        let mut prediction = self.step(&context, future, 0, patches(horizon))?;
        let mut output = prediction.clone();
        let mut offset = prediction[0][0].len();
        if horizon <= native_horizon {
            return Ok(output);
        }

        // Upstream propagates nine quantile paths and recombines their predictive
        // distributions. Rolling only the median would collapse uncertainty.
        let paths = UNROLLED_QUANTILES;
        let path_mass = quantile_mass(&paths);
        let native_mass = quantile_mass(&c.quantiles);
        let mut histories = vec![context; paths.len()];
        let mut weights = Vec::with_capacity(paths.len() * c.quantiles.len());
        for p in &path_mass {
            for q in &native_mass {
                weights.push(p * q);
            }
        }
        while offset < horizon {
            let previous_length = prediction[0][0].len();
            let previous_offset = offset - previous_length;
            let mut forecasts = Vec::with_capacity(paths.len());
            for (path, history) in paths.iter().zip(&mut histories) {
                for (channel, values) in history.iter_mut().enumerate() {
                    let retained_previous = previous_length.min(c.context_length);
                    let retained_history = c.context_length - retained_previous;
                    if values.len() > retained_history {
                        values.drain(..values.len() - retained_history);
                    }
                    for t in previous_length - retained_previous..previous_length {
                        let known = future[channel]
                            .get(previous_offset + t)
                            .copied()
                            .unwrap_or(f32::NAN);
                        values.push(if known.is_nan() {
                            interpolate(&c.quantiles, *path, |q| prediction[channel][q][t])
                        } else {
                            known
                        });
                    }
                }
                forecasts.push(self.step(history, future, offset, patches(horizon - offset))?);
            }
            let length = forecasts[0][0][0].len();
            prediction = vec![vec![vec![0.0; length]; c.quantiles.len()]; histories[0].len()];
            for (channel, pred) in prediction.iter_mut().enumerate() {
                for t in 0..length {
                    let samples: Vec<f32> = forecasts
                        .iter()
                        .flat_map(|f| f[channel].iter().map(|q| q[t]))
                        .collect();
                    let mixture = weighted_quantiles(&samples, &weights, &c.quantiles);
                    for (q, value) in mixture.into_iter().enumerate() {
                        pred[q][t] = value;
                    }
                }
                for (q, values) in pred.iter().enumerate() {
                    output[channel][q].extend(values);
                }
            }
            offset += length;
        }
        Ok(output)
    }

    fn step(
        &self,
        context: &[Vec<f32>],
        future: &[Vec<f32>],
        offset: usize,
        output_patches: usize,
    ) -> Result<Predictions> {
        let c = &self.config.chronos_config;
        let channels = context.len();
        let (context_patches, total) =
            self.config
                .validate_step_workspace(channels, context[0].len(), output_patches)?;
        let prepared = Prepared::new(context, future, offset, output_patches, c);
        let patch_size = c.input_patch_size;
        debug_assert_eq!(context_patches, prepared.context_patches);
        let x = Tensor::from_data(
            TensorData::new(
                prepared.context,
                [channels, context_patches, patch_size * 3],
            ),
            &self.device,
        );
        let mut x = self.input.forward(x, &self.config.feed_forward_proj);
        if let Some(reg) = &self.reg {
            x = Tensor::cat(
                vec![x, reg.clone().expand([channels, 1, self.config.d_model])],
                1,
            );
        }
        let f = Tensor::from_data(
            TensorData::new(prepared.future, [channels, output_patches, patch_size * 3]),
            &self.device,
        );
        x = Tensor::cat(
            vec![x, self.input.forward(f, &self.config.feed_forward_proj)],
            1,
        );
        let mask = Tensor::<2>::from_data(
            TensorData::new(prepared.mask, [channels, total]),
            &self.device,
        );
        let time_mask = mask.clone().reshape([channels, 1, 1, total]);
        let group_mask = mask.transpose().reshape([total, 1, 1, channels]);
        let rope = Rotary::new(
            total,
            self.config.d_kv,
            self.config.rope_theta,
            &self.device,
        );
        for block in &self.blocks {
            x = block.forward(
                x,
                time_mask.clone(),
                group_mask.clone(),
                &rope,
                &self.config.feed_forward_proj,
            );
        }
        let x = self.norm.forward(x).slice([
            0..channels,
            total - output_patches..total,
            0..self.config.d_model,
        ]);
        let raw = self
            .output
            .forward(x, &self.config.feed_forward_proj)
            .into_data()
            .try_to_vec::<f32>()
            .map_err(|e| Error::Tensor(format!("Cannot read Chronos-2 prediction: {e:?}")))?;
        let nq = c.quantiles.len();
        let length = output_patches * c.output_patch_size;
        let mut output = vec![vec![vec![0.0; length]; nq]; channels];
        for (channel, quantiles) in output.iter_mut().enumerate() {
            let (loc, scale) = prepared.loc_scale[channel];
            for (q, values) in quantiles.iter_mut().enumerate() {
                for (t, value) in values.iter_mut().enumerate() {
                    let index = (channel * output_patches + t / patch_size) * nq * patch_size
                        + q * patch_size
                        + t % patch_size;
                    let normalized = if c.use_arcsinh {
                        raw[index].sinh()
                    } else {
                        raw[index]
                    };
                    *value = normalized * scale + loc;
                    if !value.is_finite() {
                        return Err(Error::Tensor(
                            "Chronos-2 produced a non-finite forecast".into(),
                        ));
                    }
                }
            }
        }
        Ok(output)
    }
}

struct Prepared {
    context: Vec<f32>,
    future: Vec<f32>,
    mask: Vec<f32>,
    context_patches: usize,
    loc_scale: Vec<(f32, f32)>,
}

impl Prepared {
    fn new(
        context: &[Vec<f32>],
        future: &[Vec<f32>],
        offset: usize,
        output_patches: usize,
        c: &ForecastConfig,
    ) -> Self {
        let p = c.input_patch_size;
        let length = context[0].len().min(c.context_length);
        let context_patches = length.div_ceil(p);
        let padded = context_patches * p;
        let left_padding = padded - length;
        let time_scale = c.time_encoding_scale.unwrap_or(c.context_length) as f32;
        let mut result = Self {
            context: Vec::new(),
            future: Vec::new(),
            mask: Vec::new(),
            context_patches,
            loc_scale: Vec::new(),
        };
        for (channel, row) in context.iter().enumerate() {
            let row = &row[row.len() - length..];
            let count = row.iter().filter(|x| !x.is_nan()).count();
            let loc = if count == 0 {
                0.0
            } else {
                row.iter().filter(|x| !x.is_nan()).sum::<f32>() / count as f32
            };
            let scale = if count == 0 {
                1.0
            } else {
                let variance = row
                    .iter()
                    .filter(|x| !x.is_nan())
                    .map(|x| (x - loc).powi(2))
                    .sum::<f32>()
                    / count as f32;
                let scale = variance.sqrt();
                if scale == 0.0 { 1e-5 } else { scale }
            };
            result.loc_scale.push((loc, scale));
            let normalize = |value: f32| {
                let x = (value - loc) / scale;
                if c.use_arcsinh { x.asinh() } else { x }
            };
            for patch in 0..context_patches {
                for i in 0..p {
                    result
                        .context
                        .push(((patch * p + i) as f32 - padded as f32) / time_scale);
                }
                let values: Vec<f32> = (0..p)
                    .map(|i| {
                        (patch * p + i)
                            .checked_sub(left_padding)
                            .map(|index| row[index])
                            .unwrap_or(f32::NAN)
                    })
                    .collect();
                result.context.extend(
                    values
                        .iter()
                        .map(|v| if v.is_nan() { 0.0 } else { normalize(*v) }),
                );
                result
                    .context
                    .extend(values.iter().map(|v| if v.is_nan() { 0.0 } else { 1.0 }));
                result.mask.push(if values.iter().all(|v| v.is_nan()) {
                    f32::MIN
                } else {
                    0.0
                });
            }
            result.mask.extend(std::iter::repeat_n(
                0.0,
                usize::from(c.use_reg_token) + output_patches,
            ));
            for patch in 0..output_patches {
                for i in 0..p {
                    result.future.push((patch * p + i) as f32 / time_scale);
                }
                let values: Vec<f32> = (0..p)
                    .map(|i| {
                        future[channel]
                            .get(offset + patch * p + i)
                            .copied()
                            .unwrap_or(f32::NAN)
                    })
                    .collect();
                result.future.extend(
                    values
                        .iter()
                        .map(|v| if v.is_nan() { 0.0 } else { normalize(*v) }),
                );
                result
                    .future
                    .extend(values.iter().map(|v| if v.is_nan() { 0.0 } else { 1.0 }));
            }
        }
        result
    }
}

fn interpolate(levels: &[f32], query: f32, value: impl Fn(usize) -> f32) -> f32 {
    if query <= levels[0] {
        return value(0);
    }
    if query >= levels[levels.len() - 1] {
        return value(levels.len() - 1);
    }
    let hi = levels.partition_point(|q| *q < query);
    let lo = hi - 1;
    let alpha = (query - levels[lo]) / (levels[hi] - levels[lo]);
    value(lo) * (1.0 - alpha) + value(hi) * alpha
}

fn quantile_mass(levels: &[f32]) -> Vec<f32> {
    let mut mass: Vec<f32> = (0..levels.len())
        .map(|i| {
            let low = if i == 0 { 0.0 } else { levels[i - 1] };
            let high = if i + 1 == levels.len() {
                1.0
            } else {
                levels[i + 1]
            };
            (high - low) / 2.0
        })
        .collect();
    let total: f32 = mass.iter().sum();
    for value in &mut mass {
        *value /= total;
    }
    mass
}

fn weighted_quantiles(samples: &[f32], weights: &[f32], levels: &[f32]) -> Vec<f32> {
    let mut order: Vec<usize> = (0..samples.len()).collect();
    order.sort_by(|a, b| samples[*a].total_cmp(&samples[*b]));
    let total: f32 = weights.iter().sum();
    let mut sum = 0.0;
    let cumulative: Vec<f32> = order
        .iter()
        .map(|i| {
            sum += weights[*i] / total;
            sum.clamp(0.0, 1.0)
        })
        .collect();
    levels
        .iter()
        .map(|q| interpolate(&cumulative, *q, |i| samples[order[i]]))
        .collect()
}

#[cfg(all(test, feature = "cpu"))]
mod tests {
    use super::*;
    use crate::ForecastSeries;
    use safetensors::{Dtype, tensor::TensorView};
    use std::collections::BTreeMap;

    fn close(actual: &[f32], expected: &[f32], tolerance: f32) {
        assert_eq!(actual.len(), expected.len());
        for (i, (a, e)) in actual.iter().zip(expected).enumerate() {
            assert!((a - e).abs() <= tolerance, "element {i}: {a} != {e}");
        }
    }

    fn tiny_config() -> serde_json::Value {
        serde_json::json!({
            "architectures": ["Chronos2Model"], "d_model": 2, "d_kv": 2,
            "d_ff": 4, "num_layers": 1, "num_heads": 1,
            "chronos_config": { "context_length": 4, "input_patch_size": 2,
                "input_patch_stride": 2, "output_patch_size": 2,
                "quantiles": [0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9],
                "use_reg_token": true, "use_arcsinh": true, "max_output_patches": 1 }
        })
    }

    fn checkpoint(signal: bool) -> tempfile::TempDir {
        checkpoint_with_vocab(signal, 2)
    }

    fn checkpoint_with_vocab(signal: bool, vocab: usize) -> tempfile::TempDir {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("config.json"),
            tiny_config().to_string(),
        )
        .unwrap();
        let mut weights = BTreeMap::<String, (Vec<usize>, Vec<f32>)>::new();
        let mut linear = |name: &str, input: usize, output: usize, bias: bool| {
            weights.insert(
                format!("{name}.weight"),
                (vec![output, input], vec![0.0; input * output]),
            );
            if bias {
                weights.insert(format!("{name}.bias"), (vec![output], vec![0.0; output]));
            }
        };
        for (name, input, output) in [
            ("input_patch_embedding", 6, 2),
            ("output_patch_embedding", 2, 18),
        ] {
            linear(&format!("{name}.hidden_layer"), input, 4, true);
            linear(&format!("{name}.output_layer"), 4, output, true);
            linear(&format!("{name}.residual_layer"), input, output, true);
        }
        for axis in 0..2 {
            for projection in ["q", "k", "v", "o"] {
                linear(
                    &format!("encoder.block.0.layer.{axis}.self_attention.{projection}"),
                    2,
                    2,
                    false,
                );
            }
        }
        linear("encoder.block.0.layer.2.mlp.wi", 2, 4, false);
        linear("encoder.block.0.layer.2.mlp.wo", 4, 2, false);
        weights.insert(
            "shared.weight".into(),
            (vec![vocab, 2], vec![0.0; vocab * 2]),
        );
        for prefix in [
            "encoder.block.0.layer.0.layer_norm",
            "encoder.block.0.layer.1.layer_norm",
            "encoder.block.0.layer.2.layer_norm",
            "encoder.final_layer_norm",
        ] {
            weights.insert(format!("{prefix}.weight"), (vec![2], vec![1.0; 2]));
        }
        let output_bias = &mut weights
            .get_mut("output_patch_embedding.output_layer.bias")
            .unwrap()
            .1;
        for q in 0..9 {
            for p in 0..2 {
                output_bias[q * 2 + p] = (q as f32 - 4.0) * 0.1 + p as f32 * 0.025;
            }
        }
        if signal {
            weights
                .get_mut("input_patch_embedding.residual_layer.weight")
                .unwrap()
                .1 = vec![1.0, 0.0, 0.5, 0.0, 0.3, 0.0, 0.0, 1.0, 0.0, 0.5, 0.0, 0.3];
            for axis in 0..2 {
                for projection in ["q", "k", "v", "o"] {
                    weights
                        .get_mut(&format!(
                            "encoder.block.0.layer.{axis}.self_attention.{projection}.weight"
                        ))
                        .unwrap()
                        .1 = vec![1.0, 0.0, 0.0, 1.0];
                }
            }
            let residual = &mut weights
                .get_mut("output_patch_embedding.residual_layer.weight")
                .unwrap()
                .1;
            for q in 0..9 {
                for p in 0..2 {
                    residual[(q * 2 + p) * 2 + p] = 0.3;
                }
            }
        }
        let bytes: Vec<(String, Vec<usize>, Vec<u8>)> = weights
            .into_iter()
            .map(|(n, (s, v))| (n, s, v.into_iter().flat_map(f32::to_le_bytes).collect()))
            .collect();
        let views: Vec<_> = bytes
            .iter()
            .map(|(name, shape, bytes)| {
                (
                    name.as_str(),
                    TensorView::new(Dtype::F32, shape.clone(), bytes).unwrap(),
                )
            })
            .collect();
        safetensors::tensor::serialize_to_file(
            views,
            None,
            &directory.path().join("model.safetensors"),
        )
        .unwrap();
        directory
    }

    fn request(horizon: usize) -> ForecastRequest {
        ForecastRequest {
            series: vec![ForecastSeries {
                id: "target".into(),
                targets: vec![vec![1.0, 2.0, 3.0, 4.0]],
                observed_mask: None,
                past_covariates: vec![],
                known_covariates: vec![],
            }],
            horizon,
            quantile_levels: vec![],
        }
    }

    #[test]
    fn preparation_matches_nan_normalization_and_patched_covariates() {
        let config: Config = serde_json::from_value(tiny_config()).unwrap();
        // Truncation precedes normalization; a completely missing patch is masked.
        let prepared = Prepared::new(
            &[vec![100.0, f32::NAN, f32::NAN, 2.0, 4.0]],
            &[vec![6.0]],
            0,
            1,
            &config.chronos_config,
        );
        assert_eq!(prepared.loc_scale, [(3.0, 1.0)]);
        close(
            &prepared.context,
            &[
                -1.0,
                -0.75,
                0.0,
                0.0,
                0.0,
                0.0,
                -0.5,
                -0.25,
                (-1.0_f32).asinh(),
                1.0_f32.asinh(),
                1.0,
                1.0,
            ],
            1e-6,
        );
        close(
            &prepared.future,
            &[0.0, 0.25, 3.0_f32.asinh(), 0.0, 1.0, 0.0],
            1e-6,
        );
        assert_eq!(prepared.mask, [f32::MIN, 0.0, 0.0, 0.0]);
        let padded = Prepared::new(
            &[vec![2.0, 3.0, 4.0]],
            &[vec![]],
            0,
            1,
            &config.chronos_config,
        );
        assert_eq!(padded.context[4..6], [0.0, 1.0]);
    }

    #[test]
    fn temporal_attention_matches_scalar_rope_reference_without_score_scaling() {
        let device = Device::flex();
        let identity = || Linear {
            weight: Tensor::from_data([[1.0, 0.0], [0.0, 1.0]], &device),
            bias: None,
        };
        let attention = Attention {
            norm: RmsNorm {
                weight: Tensor::ones([2], &device),
                epsilon: 1e-6,
            },
            q: identity(),
            k: identity(),
            v: identity(),
            o: identity(),
            heads: 1,
            head_dim: 2,
        };
        let x = Tensor::from_data([[[1.0, 0.0], [0.0, 2.0]]], &device);
        let result = attention
            .forward(
                x.clone(),
                Tensor::zeros([1, 1, 1, 2], &device),
                Some(&Rotary::new(2, 2, 10_000.0, &device)),
            )
            .into_data()
            .try_to_vec::<f32>()
            .unwrap();
        let a = 1.0_f32 / (0.5_f32 + 1e-6).sqrt();
        let b = 2.0_f32 / (2.0_f32 + 1e-6).sqrt();
        let cross = -a * b * 1.0_f32.sin();
        let p0 = 1.0 / (1.0 + (cross - a * a).exp());
        let p1 = 1.0 / (1.0 + (cross - b * b).exp());
        close(
            &result,
            &[1.0 + p0 * a, (1.0 - p0) * b, (1.0 - p1) * a, 2.0 + p1 * b],
            2e-6,
        );
        let masked = attention
            .forward(x, Tensor::from_data([[[[0.0, f32::MIN]]]], &device), None)
            .into_data()
            .try_to_vec::<f32>()
            .unwrap();
        close(&masked, &[1.0 + a, 0.0, a, 2.0], 2e-6);
    }

    #[test]
    fn checkpoint_forecast_matches_analytic_quantiles_and_output_layout() {
        let directory = checkpoint(false);
        let model = Chronos2::load(directory.path(), &Device::flex()).unwrap();
        let mut request = request(2);
        request.quantile_levels = vec![0.15, 0.5, 0.85];
        let result = model.forecast(&request).unwrap();
        result.validate(&request).unwrap();
        let scale = 1.25_f32.sqrt();
        for t in 0..2 {
            let native =
                |q: usize| 2.5 + scale * ((q as f32 - 4.0) * 0.1 + t as f32 * 0.025).sinh();
            close(
                &result.series[0].quantiles[0][t],
                &[
                    (native(0) + native(1)) * 0.5,
                    native(4),
                    (native(7) + native(8)) * 0.5,
                ],
                2e-6,
            );
            close(&[result.series[0].point[0][t]], &[native(4)], 2e-6);
        }
        assert_eq!(result.point_statistic, PointStatistic::Median);
        request.quantile_levels = vec![0.01];
        assert!(model.forecast(&request).is_err());
    }

    #[test]
    fn masked_values_cannot_leak_and_known_future_covariates_affect_targets() {
        let directory = checkpoint(true);
        let model = Chronos2::load(directory.path(), &Device::flex()).unwrap();
        let mut request = request(5);
        request.series[0].observed_mask = Some(vec![vec![true, false, true, true]]);
        request.series[0].past_covariates = vec![vec![4.0, 3.0, 2.0, 1.0]];
        request.series[0].known_covariates =
            vec![vec![1.0, 2.0, 3.0, 4.0, 2.0, 2.0, 2.0, 2.0, 2.0]];
        let original = model.forecast(&request).unwrap();
        original.validate(&request).unwrap();
        request.series[0].targets[0][1] = 1e20;
        let changed_missing = model.forecast(&request).unwrap();
        close(
            &original.series[0].point[0],
            &changed_missing.series[0].point[0],
            0.0,
        );
        request.series[0].known_covariates[0][4..].fill(20.0);
        let changed_known = model.forecast(&request).unwrap();
        assert!(
            (original.series[0].point[0][0] - changed_known.series[0].point[0][0]).abs() > 1e-3
        );
        for step in &original.series[0].quantiles[0] {
            assert!(step[8] - step[0] > 0.01);
        }
        let mut second = request.series[0].clone();
        second.id = "second".into();
        second.targets[0].iter_mut().for_each(|v| *v += 100.0);
        request.series.push(second);
        let batched = model.forecast(&request).unwrap();
        close(
            &changed_known.series[0].point[0],
            &batched.series[0].point[0],
            0.0,
        );
    }

    #[test]
    fn weighted_long_horizon_distribution_preserves_probability_masses() {
        close(
            &quantile_mass(&[0.1, 0.5, 0.9]),
            &[0.5 / 1.8, 0.8 / 1.8, 0.5 / 1.8],
            1e-6,
        );
        close(
            &weighted_quantiles(&[30.0, 10.0, 20.0], &[0.5, 0.25, 0.25], &[0.25, 0.5, 0.75]),
            &[10.0, 20.0, 25.0],
            1e-6,
        );
    }

    #[test]
    fn workspace_limits_cover_both_attention_axes_and_padded_covariate_outputs() {
        let mut config: Config = serde_json::from_value(tiny_config()).unwrap();
        config.validate_step_workspace(3, 4, 1).unwrap();
        config.validate_forecast_workspace(3, 4, 5).unwrap();
        config.chronos_config.context_length = 32_768;
        let error = config.validate_step_workspace(1, 32_768, 1).unwrap_err();
        assert!(error.to_string().contains("temporal attention"));
        config.chronos_config.context_length = 4;
        let error = config.validate_step_workspace(8192, 4, 1).unwrap_err();
        assert!(error.to_string().contains("group attention"));
        assert!(config.validate_step_workspace(usize::MAX, 4, 1).is_err());

        // One requested step still allocates a complete two-step output patch.
        let error = config
            .validate_forecast_workspace(4_000_000, 4, 1)
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("native quantile output including covariates")
        );
        let error = config
            .validate_forecast_workspace(500_000, 4, 3)
            .unwrap_err();
        assert!(error.to_string().contains("propagated quantile forecasts"));
        config.chronos_config.context_length = 8_000_000;
        let error = config
            .validate_forecast_workspace(1, 8_000_000, 3)
            .unwrap_err();
        assert!(error.to_string().contains("propagated quantile histories"));
    }

    #[test]
    fn rolling_history_truncation_matches_an_already_truncated_request() {
        let directory = checkpoint(true);
        let model = Chronos2::load(directory.path(), &Device::flex()).unwrap();
        let short = request(7);
        let mut long = short.clone();
        long.series[0].targets[0].splice(0..0, [1000.0, -1000.0, 2000.0]);
        let expected = model.forecast(&short).unwrap();
        let actual = model.forecast(&long).unwrap();
        for (actual, expected) in actual.series[0].quantiles[0]
            .iter()
            .zip(&expected.series[0].quantiles[0])
        {
            close(actual, expected, 0.0);
        }
        assert_eq!(
            model.capabilities().max_horizon,
            Some(crate::types::MAX_FORECAST_HORIZON)
        );
    }

    #[test]
    fn rejects_checkpoint_shapes_and_unsupported_patch_stride() {
        let directory = checkpoint(false);
        let mut config = tiny_config();
        config["d_model"] = 3.into();
        std::fs::write(directory.path().join("config.json"), config.to_string()).unwrap();
        assert!(Chronos2::load(directory.path(), &Device::flex()).is_err());
        config = tiny_config();
        config["chronos_config"]["input_patch_stride"] = 1.into();
        std::fs::write(directory.path().join("config.json"), config.to_string()).unwrap();
        assert!(Chronos2::load(directory.path(), &Device::flex()).is_err());
    }

    #[test]
    fn rejects_dimension_overflow_and_inconsistent_registration_tokens() {
        for field in ["num_heads", "d_ff", "d_model"] {
            let mut config = tiny_config();
            config[field] = serde_json::json!(usize::MAX);
            let config: Config = serde_json::from_value(config).unwrap();
            assert!(
                config
                    .validate()
                    .unwrap_err()
                    .to_string()
                    .contains("overflow")
            );
        }
        let mut config = tiny_config();
        config["chronos_config"]["max_output_patches"] = serde_json::json!(usize::MAX);
        let config: Config = serde_json::from_value(config).unwrap();
        assert!(
            config
                .validate()
                .unwrap_err()
                .to_string()
                .contains("overflow")
        );
        let mut config = tiny_config();
        for field in [
            "input_patch_size",
            "input_patch_stride",
            "output_patch_size",
        ] {
            config["chronos_config"][field] = serde_json::json!(usize::MAX);
        }
        let config: Config = serde_json::from_value(config).unwrap();
        assert!(
            config
                .validate()
                .unwrap_err()
                .to_string()
                .contains("overflow")
        );

        let directory = checkpoint(false);
        let mut config = tiny_config();
        config["reg_token_id"] = serde_json::json!(0);
        std::fs::write(directory.path().join("config.json"), config.to_string()).unwrap();
        let error = Chronos2::load(directory.path(), &Device::flex())
            .err()
            .unwrap();
        assert!(error.to_string().contains("token ID 1"));
        let directory = checkpoint_with_vocab(false, 3);
        let error = Chronos2::load(directory.path(), &Device::flex())
            .err()
            .unwrap();
        assert!(
            error
                .to_string()
                .contains("shared.weight: expected [2, 2], got [3, 2]")
        );
    }

    #[test]
    fn checks_tensor_metadata_before_reading_values_or_transferring_to_device() {
        let directory = tempfile::tempdir().unwrap();
        let bytes = f32::NAN.to_le_bytes();
        let view = TensorView::new(Dtype::F32, vec![1, 1], &bytes).unwrap();
        safetensors::tensor::serialize_to_file(
            [("weight", view)],
            None,
            &directory.path().join("model.safetensors"),
        )
        .unwrap();
        let weights = Weights::open(directory.path()).unwrap();
        let error = tensor::<2>(&weights, "weight", [2, 2], &Device::flex()).unwrap_err();
        assert!(error.to_string().contains("expected [2, 2], got [1, 1]"));
    }

    #[test]
    #[ignore = "requires downloaded amazon/chronos-2 checkpoint; set FLOWLIKE_CHRONOS2_CHECKPOINT"]
    fn official_checkpoint_forecasts_multivariate_covariate_and_long_horizon_inputs() {
        let directory =
            std::env::var("FLOWLIKE_CHRONOS2_CHECKPOINT").expect("checkpoint directory");
        let model = Chronos2::load(Path::new(&directory), &Device::flex()).unwrap();
        let mut request = request(3);
        request.series[0].targets = vec![
            (0..32).map(|i| (i as f32 * 0.1).sin()).collect(),
            (0..32).map(|i| 2.0 + (i as f32 * 0.1).cos()).collect(),
        ];
        request.series[0].observed_mask =
            Some(vec![(0..32).map(|i| i != 12).collect(), vec![true; 32]]);
        request.series[0].past_covariates = vec![(0..32).map(|i| i as f32).collect()];
        request.series[0].known_covariates = vec![(0..35).map(|i| (i % 7) as f32).collect()];
        request.quantile_levels = vec![0.1, 0.5, 0.9];
        let result = model.forecast(&request).unwrap();
        result.validate(&request).unwrap();
        assert!(
            result.series[0]
                .quantiles
                .iter()
                .flatten()
                .all(|q| q[0] < q[2])
        );
        // Exercise the same propagation algorithm with a short native horizon,
        // keeping this smoke test practical on a CPU.
        let mut model = model;
        model.config.chronos_config.max_output_patches = 1;
        request.horizon = 17;
        request.series[0].known_covariates[0] = (0..49).map(|i| (i % 7) as f32).collect();
        model
            .forecast(&request)
            .unwrap()
            .validate(&request)
            .unwrap();
    }
}
