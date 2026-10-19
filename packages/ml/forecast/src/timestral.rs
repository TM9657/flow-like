//! Native inference for the released Timestral checkpoint format.
// Architecture and decoding follow mistralai/timestral (Apache-2.0).

use crate::{Error, ForecastRequest, ForecastResult, PointStatistic, Result, SeriesForecast};
use burn::tensor::{
    Bool, Device, Tensor, TensorData,
    activation::{sigmoid, silu, softmax},
};
use serde::Deserialize;
use std::path::Path;

fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}

fn checked_elements(dimensions: &[usize], name: &str) -> Result<usize> {
    let count = dimensions
        .iter()
        .try_fold(1_usize, |count, dimension| count.checked_mul(*dimension));
    match count {
        Some(count) if count <= isize::MAX as usize / std::mem::size_of::<f32>() => Ok(count),
        _ => Err(invalid(format!(
            "Timestral {name} dimensions overflow a tensor allocation"
        ))),
    }
}

fn load_tensor<const D: usize>(
    weights: &crate::weights::Weights,
    name: &str,
    expected: [usize; D],
    device: &Device,
) -> Result<Tensor<D>> {
    checked_elements(&expected, name)?;
    if weights.shape(name)? != expected {
        return Err(invalid(format!(
            "Timestral {name} must have shape {expected:?}"
        )));
    }
    weights.tensor::<D>(name, device)
}
fn yes() -> bool {
    true
}
fn rope_theta() -> f64 {
    10_000.0
}
fn half() -> f64 {
    0.5
}
fn epsilon() -> f64 {
    1e-6
}
fn gate() -> String {
    "sdpa_sigmoid".into()
}
fn width_factor() -> f32 {
    2.0
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    patch_size: usize,
    d_model: usize,
    num_heads: usize,
    mlp_hidden_dim: usize,
    block_schedule: Vec<String>,
    quantile_knots: Vec<f32>,
    block_size: usize,
    context_length: usize,
    #[serde(default = "rope_theta")]
    rope_theta: f64,
    #[serde(default = "half")]
    rope_partial_factor: f64,
    #[serde(default = "epsilon")]
    norm_eps: f64,
    #[serde(default)]
    attn_bias: bool,
    #[serde(default)]
    mlp_bias: bool,
    #[serde(default = "gate")]
    time_attn_gate: String,
    #[serde(default = "gate")]
    variate_attn_gate: String,
    #[serde(default = "epsilon")]
    minimum_scale: f64,
    #[serde(default = "yes")]
    use_asinh: bool,
    #[serde(default)]
    stabilize_with_global: bool,
    #[serde(default = "yes")]
    covariate_enabled: bool,
    #[serde(default = "yes")]
    split_context_horizon_attn: bool,
    #[serde(default = "yes")]
    attend_missing_patches: bool,
    #[serde(default = "width_factor")]
    quantile_width_factor: f32,
}

impl Config {
    fn validate(&self) -> Result<()> {
        if self.patch_size == 0
            || self.d_model == 0
            || self.num_heads == 0
            || !self.d_model.is_multiple_of(self.num_heads)
            || self.mlp_hidden_dim == 0
            || self.block_size == 0
            || self.context_length == 0
            || self.block_schedule.is_empty()
        {
            return Err(invalid(
                "Timestral configuration has invalid architecture dimensions",
            ));
        }
        let rotary = self.rope_dim();
        if self.head_dim() % 2 != 0
            || rotary == 0
            || rotary % 2 != 0
            || rotary > self.head_dim()
            || !self.rope_partial_factor.is_finite()
            || self.rope_partial_factor <= 0.0
            || self.rope_partial_factor > 1.0
            || !self.rope_theta.is_finite()
            || self.rope_theta <= 0.0
        {
            return Err(invalid(
                "Timestral configuration has invalid rotary dimensions or base",
            ));
        }
        if self.block_schedule.iter().any(|kind| axes(kind).is_none())
            || [&self.time_attn_gate, &self.variate_attn_gate]
                .iter()
                .any(|v| !matches!(v.as_str(), "none" | "sdpa_sigmoid"))
        {
            return Err(invalid("Unsupported Timestral attention schedule or gate"));
        }
        let knots = &self.quantile_knots;
        if !knots.contains(&0.5)
            || knots
                .iter()
                .any(|q| !q.is_finite() || *q <= 0.0 || *q >= 1.0)
            || knots.windows(2).any(|w| w[0] >= w[1])
            || knots
                .iter()
                .zip(knots.iter().rev())
                .any(|(lo, hi)| (lo + hi - 1.0).abs() > 1e-6)
        {
            return Err(invalid(
                "Timestral quantile knots must be increasing, include 0.5, and be symmetric",
            ));
        }
        if !self.norm_eps.is_finite()
            || self.norm_eps <= 0.0
            || !(self.norm_eps as f32).is_finite()
            || self.norm_eps as f32 == 0.0
            || !self.minimum_scale.is_finite()
            || self.minimum_scale <= 0.0
            || !(self.minimum_scale as f32).is_finite()
            || self.minimum_scale as f32 == 0.0
            || !self.quantile_width_factor.is_finite()
            || self.quantile_width_factor <= 0.0
        {
            return Err(invalid(
                "Timestral normalization and quantile scales must be positive and finite",
            ));
        }
        if self.stabilize_with_global {
            return Err(invalid(
                "Timestral advancing refinement requires stabilize_with_global=false",
            ));
        }
        // Validate derived dimensions before opening weights or multiplying shape fields.
        let patch_input = checked_elements(&[2, self.patch_size], "patch input")?;
        let residual_hidden = checked_elements(&[4, self.d_model], "residual hidden")?;
        let qkv = checked_elements(&[3, self.d_model], "attention projection")?;
        let mlp = checked_elements(&[2, self.mlp_hidden_dim], "MLP projection")?;
        let output = checked_elements(&[self.patch_size, knots.len()], "quantile output")?;
        checked_elements(&[self.block_size, self.patch_size], "native horizon")?;
        for (input, output) in [
            (patch_input, residual_hidden),
            (residual_hidden, self.d_model),
            (patch_input, self.d_model),
            (self.d_model, qkv),
            (self.d_model, mlp),
            (self.mlp_hidden_dim, self.d_model),
            (residual_hidden, output),
            (self.d_model, output),
        ] {
            checked_elements(&[input, output], "weight matrix")?;
        }
        let context_patches = self.context_length.div_ceil(self.patch_size);
        checked_elements(&[context_patches, self.patch_size], "padded context")?;
        let window = context_patches
            .checked_add(self.block_size.min(32))
            .ok_or_else(|| invalid("Timestral context plus decode block overflows"))?;
        let sequence = checked_elements(&[2, window], "covariate sequence")?;
        checked_elements(&[self.num_heads, sequence, sequence], "attention scores")?;
        Ok(())
    }
    fn head_dim(&self) -> usize {
        self.d_model / self.num_heads
    }
    fn rope_dim(&self) -> usize {
        (self.head_dim() as f64 * self.rope_partial_factor) as usize
    }
    fn median_index(&self) -> usize {
        self.quantile_knots.iter().position(|q| *q == 0.5).unwrap()
    }
    fn known_covariates(&self) -> bool {
        if !self.covariate_enabled {
            return false;
        }
        let ops: Vec<bool> = self
            .block_schedule
            .iter()
            .flat_map(|s| axes(s).unwrap())
            .copied()
            .collect();
        ops.iter()
            .enumerate()
            .any(|(i, time)| *time && ops[i + 1..].contains(&true) && ops[i + 1..].contains(&false))
    }

    fn validate_workspace(
        &self,
        rows: usize,
        context_patches: usize,
        horizon_patches: usize,
        covariates: bool,
    ) -> Result<()> {
        use crate::types::validate_workspace;
        let encoder = context_patches
            .checked_add(if covariates { horizon_patches } else { 0 })
            .ok_or_else(|| invalid("Timestral encoder length overflows"))?;
        let decoder = if covariates { encoder } else { horizon_patches };
        let sequence = encoder
            .checked_add(decoder)
            .ok_or_else(|| invalid("Timestral sequence length overflows"))?;
        // Masks cover the full sequence even when time attention is split.
        validate_workspace(&[rows, sequence, sequence], "Timestral time mask")?;
        if self.split_context_horizon_attn {
            validate_workspace(
                &[rows, self.num_heads, encoder, encoder],
                "Timestral context attention",
            )?;
            validate_workspace(
                &[rows, self.num_heads, decoder, sequence],
                "Timestral forecast attention",
            )?;
        } else {
            validate_workspace(
                &[rows, self.num_heads, sequence, sequence],
                "Timestral time attention",
            )?;
        }
        validate_workspace(
            &[sequence, self.num_heads, rows, rows],
            "Timestral variate attention",
        )?;
        for (dimensions, name) in [
            ([rows, encoder, 2, self.patch_size], "patch input"),
            ([rows, sequence, 3, self.d_model], "attention projection"),
            ([rows, sequence, 2, self.mlp_hidden_dim], "MLP projection"),
            ([rows, sequence, 4, self.d_model], "residual hidden"),
            (
                [
                    rows,
                    horizon_patches,
                    self.patch_size,
                    self.quantile_knots.len(),
                ],
                "quantile output",
            ),
        ] {
            validate_workspace(&dimensions, &format!("Timestral {name}"))?;
        }
        Ok(())
    }
}

fn axes(kind: &str) -> Option<&'static [bool]> {
    match kind {
        "time" => Some(&[true]),
        "variate" => Some(&[false]),
        "time_variate" => Some(&[true, false]),
        "variate_time" => Some(&[false, true]),
        _ => None,
    }
}

struct Linear {
    weight: Tensor<2>,
    bias: Option<Tensor<1>>,
}
impl Linear {
    fn load(
        w: &crate::weights::Weights,
        name: &str,
        input: usize,
        output: usize,
        bias: bool,
        device: &Device,
    ) -> Result<Self> {
        let weight = load_tensor(w, &format!("{name}.weight"), [output, input], device)?;
        let bias = if bias {
            Some(load_tensor(w, &format!("{name}.bias"), [output], device)?)
        } else {
            None
        };
        Ok(Self {
            weight: weight.transpose(),
            bias,
        })
    }
    fn forward(&self, x: Tensor<3>) -> Tensor<3> {
        let mut out = x.matmul(self.weight.clone().unsqueeze());
        if let Some(bias) = &self.bias {
            out = out + bias.clone().unsqueeze::<3>();
        }
        out
    }
}

struct Residual {
    hidden: Linear,
    output: Linear,
    residual: Linear,
}
impl Residual {
    fn load(
        w: &crate::weights::Weights,
        name: &str,
        input: usize,
        hidden: usize,
        output: usize,
        bias: bool,
        device: &Device,
    ) -> Result<Self> {
        Ok(Self {
            hidden: Linear::load(
                w,
                &format!("{name}.hidden_layer"),
                input,
                hidden,
                bias,
                device,
            )?,
            output: Linear::load(
                w,
                &format!("{name}.output_layer"),
                hidden,
                output,
                bias,
                device,
            )?,
            residual: Linear::load(
                w,
                &format!("{name}.residual_layer"),
                input,
                output,
                bias,
                device,
            )?,
        })
    }
    fn forward(&self, x: Tensor<3>) -> Tensor<3> {
        self.output.forward(silu(self.hidden.forward(x.clone()))) + self.residual.forward(x)
    }
}

struct Norm {
    scale: Tensor<1>,
    eps: f64,
}
impl Norm {
    fn load(w: &crate::weights::Weights, name: &str, c: &Config, device: &Device) -> Result<Self> {
        let scale = load_tensor(w, &format!("{name}.scale"), [c.d_model], device)?;
        Ok(Self {
            scale,
            eps: c.norm_eps,
        })
    }
    fn forward(&self, x: Tensor<3>) -> Tensor<3> {
        let inverse = (x.clone().powf_scalar(2.0).mean_dim(2) + self.eps)
            .sqrt()
            .recip();
        x * inverse * self.scale.clone().unsqueeze::<3>()
    }
}

struct Attention {
    qkv: Linear,
    out: Linear,
    gate: Option<Linear>,
    time: bool,
    heads: usize,
}
impl Attention {
    fn load(
        w: &crate::weights::Weights,
        name: &str,
        time: bool,
        c: &Config,
        device: &Device,
    ) -> Result<Self> {
        let gate = if (if time {
            &c.time_attn_gate
        } else {
            &c.variate_attn_gate
        }) == "sdpa_sigmoid"
        {
            Some(Linear::load(
                w,
                &format!("{name}._gate"),
                c.d_model,
                c.d_model,
                c.attn_bias,
                device,
            )?)
        } else {
            None
        };
        Ok(Self {
            qkv: Linear::load(
                w,
                &format!("{name}.wQKV"),
                c.d_model,
                3 * c.d_model,
                c.attn_bias,
                device,
            )?,
            out: Linear::load(
                w,
                &format!("{name}.wO"),
                c.d_model,
                c.d_model,
                c.attn_bias,
                device,
            )?,
            gate,
            time,
            heads: c.num_heads,
        })
    }
    fn forward(
        &self,
        x: Tensor<3>,
        mask: &Tensor<4, Bool>,
        cos: &Tensor<4>,
        sin: &Tensor<4>,
        enc_len: Option<usize>,
    ) -> Tensor<3> {
        let inputs = if self.time { x } else { x.swap_dims(0, 1) };
        let [batch, seq, width] = inputs.dims();
        let head = width / self.heads;
        // Checkpoints interleave head indices within each head dimension.
        let projected = self
            .qkv
            .forward(inputs.clone())
            .reshape([batch, seq, 3, head, self.heads]);
        let split = |i| {
            projected
                .clone()
                .narrow(2, i, 1)
                .reshape([batch, seq, head, self.heads])
                .swap_dims(2, 3)
        };
        let (mut q, mut k, v) = (split(0), split(1), split(2));
        if self.time {
            q = rotary(q, cos, sin);
            k = rotary(k, cos, sin);
        }
        let attend = |q: Tensor<4>, k: Tensor<4>, v: Tensor<4>, mask: Tensor<4, Bool>| {
            let qlen = q.dims()[1];
            let scores = q.swap_dims(1, 2).matmul(k.permute([0, 2, 3, 1])) / (head as f64).sqrt();
            softmax(scores.mask_fill(mask, f32::NEG_INFINITY), 3)
                .matmul(v.swap_dims(1, 2))
                .swap_dims(1, 2)
                .reshape([batch, qlen, width])
        };
        let mut out = if self.time && enc_len.is_some() {
            let n = enc_len.unwrap();
            Tensor::cat(
                vec![
                    attend(
                        q.clone().narrow(1, 0, n),
                        k.clone().narrow(1, 0, n),
                        v.clone().narrow(1, 0, n),
                        mask.clone().narrow(2, 0, n).narrow(3, 0, n),
                    ),
                    attend(
                        q.narrow(1, n, seq - n),
                        k,
                        v,
                        mask.clone().narrow(2, n, seq - n),
                    ),
                ],
                1,
            )
        } else {
            attend(q, k, v, mask.clone())
        };
        if let Some(gate) = &self.gate {
            out = out * sigmoid(gate.forward(inputs));
        }
        self.out
            .forward(if self.time { out } else { out.swap_dims(0, 1) })
    }
}

fn rotary(x: Tensor<4>, cos: &Tensor<4>, sin: &Tensor<4>) -> Tensor<4> {
    let half = x.dims()[3] / 2;
    let rotated = Tensor::cat(
        vec![
            -x.clone().narrow(3, half, half),
            x.clone().narrow(3, 0, half),
        ],
        3,
    );
    x * cos.clone() + rotated * sin.clone()
}

struct Block {
    attention: Vec<(Norm, Attention)>,
    mlp_norm: Norm,
    w12: Linear,
    w3: Linear,
    hidden: usize,
}
impl Block {
    fn load(w: &crate::weights::Weights, i: usize, c: &Config, device: &Device) -> Result<Self> {
        let name = format!("transformer.layers.{i}");
        let attention = axes(&c.block_schedule[i])
            .unwrap()
            .iter()
            .enumerate()
            .map(|(j, time)| {
                Ok((
                    Norm::load(w, &format!("{name}.attn_norms.{j}"), c, device)?,
                    Attention::load(w, &format!("{name}.attns.{j}"), *time, c, device)?,
                ))
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            attention,
            mlp_norm: Norm::load(w, &format!("{name}.mlp_norm"), c, device)?,
            w12: Linear::load(
                w,
                &format!("{name}.mlp.w12"),
                c.d_model,
                2 * c.mlp_hidden_dim,
                c.mlp_bias,
                device,
            )?,
            w3: Linear::load(
                w,
                &format!("{name}.mlp.w3"),
                c.mlp_hidden_dim,
                c.d_model,
                c.mlp_bias,
                device,
            )?,
            hidden: c.mlp_hidden_dim,
        })
    }
    fn forward(&self, mut x: Tensor<3>, masks: &Masks) -> Tensor<3> {
        for (norm, attn) in &self.attention {
            x = x.clone()
                + attn.forward(
                    norm.forward(x),
                    if attn.time {
                        &masks.time
                    } else {
                        &masks.variate
                    },
                    &masks.cos,
                    &masks.sin,
                    masks.split,
                );
        }
        let fused = self.w12.forward(self.mlp_norm.forward(x.clone()));
        x + self.w3.forward(
            fused.clone().narrow(2, 0, self.hidden)
                * silu(fused.narrow(2, self.hidden, self.hidden)),
        )
    }
}

struct Masks {
    time: Tensor<4, Bool>,
    variate: Tensor<4, Bool>,
    cos: Tensor<4>,
    sin: Tensor<4>,
    split: Option<usize>,
}

/// A loaded Timestral network. The device selects the Burn execution backend.
pub struct Timestral {
    config: Config,
    patch: Residual,
    query: Tensor<1>,
    blocks: Vec<Block>,
    out_norm: Norm,
    head: Residual,
    device: Device,
}

impl Timestral {
    pub fn load(directory: &Path, device: &Device) -> Result<Self> {
        let config: Config =
            serde_json::from_slice(&std::fs::read(directory.join("config.json"))?)?;
        config.validate()?;
        let w = crate::weights::Weights::open(directory)?;
        let c = &config;
        let patch = Residual::load(
            &w,
            "representation.patch_proj",
            2 * c.patch_size,
            4 * c.d_model,
            c.d_model,
            true,
            device,
        )?;
        let query = load_tensor(&w, "representation.forecast_query", [c.d_model], device)?;
        let blocks = (0..c.block_schedule.len())
            .map(|i| Block::load(&w, i, c, device))
            .collect::<Result<_>>()?;
        let out_norm = Norm::load(&w, "transformer.out_norm", c, device)?;
        let head = Residual::load(
            &w,
            "head.projection",
            c.d_model,
            4 * c.d_model,
            c.patch_size * c.quantile_knots.len(),
            false,
            device,
        )?;
        Ok(Self {
            config,
            patch,
            query,
            blocks,
            out_norm,
            head,
            device: device.clone(),
        })
    }

    pub fn quantile_levels(&self) -> &[f32] {
        &self.config.quantile_knots
    }
    pub fn context_length(&self) -> usize {
        self.config.context_length
    }
    pub fn supports_covariates(&self) -> bool {
        self.config.covariate_enabled
    }
    pub fn supports_known_covariates(&self) -> bool {
        self.config.known_covariates()
    }

    pub fn capabilities(&self) -> crate::ForecastCapabilities {
        crate::ForecastCapabilities {
            family: crate::ForecastFamily::Timestral,
            max_context: self.config.context_length,
            max_horizon: Some(crate::MAX_FORECAST_HORIZON),
            native_horizon: self.config.block_size * self.config.patch_size,
            quantile_levels: self.config.quantile_knots.clone(),
            joint_multivariate: true,
            past_covariates: self.config.covariate_enabled,
            known_covariates: self.config.known_covariates(),
        }
    }

    /// Forecasts each series jointly across its target variates using upstream defaults.
    pub fn forecast(&self, request: &ForecastRequest) -> Result<ForecastResult> {
        request.validate_for_quantiles(self.config.quantile_knots.len())?;
        if request.horizon == 0 || request.series.is_empty() {
            return Err(invalid("A forecast needs series and a positive horizon"));
        }
        let levels = if request.quantile_levels.is_empty() {
            self.config.quantile_knots.clone()
        } else {
            request.quantile_levels.clone()
        };
        let indices: Vec<usize> = levels
            .iter()
            .map(|q| {
                self.config
                    .quantile_knots
                    .iter()
                    .position(|k| q == k)
                    .ok_or_else(|| {
                        invalid(format!("Timestral does not provide native quantile {q}"))
                    })
            })
            .collect::<Result<_>>()?;
        if levels.windows(2).any(|w| w[0] >= w[1]) {
            return Err(invalid(
                "Requested quantile levels must be strictly increasing",
            ));
        }
        let mut outputs = Vec::with_capacity(request.series.len());
        for series in &request.series {
            let input = Prepared::new(series, request.horizon, &self.config)?;
            let positive = self.decode(&input, request.horizon, 1.0)?;
            let negative = self.decode(&input, request.horizon, -1.0)?;
            let q = self.config.quantile_knots.len();
            let median = self.config.median_index();
            let mut point = vec![vec![0.0; request.horizon]; series.targets.len()];
            let mut quantiles =
                vec![vec![vec![0.0; levels.len()]; request.horizon]; series.targets.len()];
            for v in 0..series.targets.len() {
                for t in 0..request.horizon {
                    let base = (v * request.horizon + t) * q;
                    let combine = |i: usize| {
                        let value = 0.5 * (positive[base + i] - negative[base + q - 1 - i]);
                        if input.nonnegative[v] {
                            value.max(0.0)
                        } else {
                            value
                        }
                    };
                    point[v][t] = combine(median);
                    for (j, i) in indices.iter().enumerate() {
                        quantiles[v][t][j] = combine(*i);
                    }
                    if !point[v][t].is_finite() || quantiles[v][t].iter().any(|x| !x.is_finite()) {
                        return Err(invalid("Timestral produced a non-finite forecast"));
                    }
                }
            }
            outputs.push(SeriesForecast {
                id: series.id.clone(),
                point,
                quantiles,
            });
        }
        Ok(ForecastResult {
            series: outputs,
            quantile_levels: levels,
            point_statistic: PointStatistic::Median,
        })
    }

    fn decode(&self, input: &Prepared, horizon: usize, sign: f32) -> Result<Vec<f32>> {
        let c = &self.config;
        let mut context: Vec<Vec<f32>> = input
            .rows
            .iter()
            .map(|row| row.iter().map(|x| *x * sign).collect())
            .collect();
        let mut observed = input.observed.clone();
        let q = c.quantile_knots.len();
        let mut output = vec![0.0; input.rows.len() * horizon * q];
        let patches = horizon.div_ceil(c.patch_size);
        let block_limit = c.block_size.min(32);
        let mut offset = 0;
        while offset < patches * c.patch_size {
            let block = block_limit.min((patches * c.patch_size - offset) / c.patch_size);
            let blen = block * c.patch_size;
            let future: Vec<Vec<f32>> = input
                .future
                .iter()
                .map(|r| r[offset..offset + blen].iter().map(|x| x * sign).collect())
                .collect();
            let future_mask: Vec<Vec<bool>> = input
                .future_mask
                .iter()
                .map(|r| r[offset..offset + blen].to_vec())
                .collect();
            let mut values = self.predict_block(
                &context,
                &observed,
                &input.roles,
                &future,
                &future_mask,
                block,
            )?;
            refine(&mut values, &context, &observed, &input.roles, c, blen);
            for r in 0..context.len() {
                let take = blen.min(horizon.saturating_sub(offset));
                output[(r * horizon + offset) * q..(r * horizon + offset + take) * q]
                    .copy_from_slice(&values[r * blen * q..(r * blen + take) * q]);
            }
            offset += blen;
            if offset >= patches * c.patch_size {
                break;
            }
            for r in 0..context.len() {
                for t in 0..blen {
                    let (value, mask) = match input.roles[r] {
                        1 => (0.0, false),
                        2 => (future[r][t], future_mask[r][t]),
                        _ => {
                            let value = values[(r * blen + t) * q + c.median_index()];
                            (
                                if value.is_finite() && value.abs() < 1e30 {
                                    value
                                } else {
                                    0.0
                                },
                                true,
                            )
                        }
                    };
                    context[r].push(value);
                    observed[r].push(mask);
                }
                if context[r].len() > c.context_length {
                    let keep = c
                        .patch_size
                        .max(c.context_length / c.patch_size * c.patch_size);
                    let drop = context[r].len() - keep;
                    context[r].drain(..drop);
                    observed[r].drain(..drop);
                }
            }
        }
        // The cap uses the original, truncated context, before autoregressive feedback.
        for r in 0..input.rows.len() {
            let row: Vec<f32> = input.rows[r].iter().map(|x| *x * sign).collect();
            let (_, scales) = causal_stats(&row, &input.observed[r], c.patch_size, c.minimum_scale);
            let mut min = f32::INFINITY;
            let mut max = f32::NEG_INFINITY;
            for (x, mask) in row.iter().zip(&input.observed[r]) {
                if *mask {
                    min = min.min(*x);
                    max = max.max(*x);
                }
            }
            if !min.is_finite() {
                min = 0.0;
                max = 0.0;
            }
            let margin = scales.last().copied().unwrap_or(c.minimum_scale as f32) * 1e4;
            for value in &mut output[r * horizon * q..(r + 1) * horizon * q] {
                *value = value.clamp(min - margin, max + margin);
                if value.is_nan() {
                    return Err(invalid("Timestral forecast is unstable for this input"));
                }
            }
        }
        Ok(output)
    }

    fn predict_block(
        &self,
        context: &[Vec<f32>],
        observed: &[Vec<bool>],
        roles: &[u8],
        future: &[Vec<f32>],
        future_mask: &[Vec<bool>],
        horizon_patches: usize,
    ) -> Result<Vec<f32>> {
        let c = &self.config;
        let covariates = roles.iter().any(|r| *r != 0);
        let rows = context.len();
        let context_patches = context[0].len() / c.patch_size;
        c.validate_workspace(rows, context_patches, horizon_patches, covariates)?;
        let n_enc = context_patches + if covariates { horizon_patches } else { 0 };
        let n_dec = if covariates { n_enc } else { horizon_patches };
        let mut features = Vec::with_capacity(rows * n_enc * 2 * c.patch_size);
        let mut patch_observed = vec![vec![false; n_enc]; rows];
        let mut anchors = Vec::with_capacity(rows);
        for r in 0..rows {
            let mut values = context[r].clone();
            let mut mask = observed[r].clone();
            if covariates {
                values.extend(&future[r]);
                mask.extend(&future_mask[r]);
            }
            let (mut locs, mut scales) =
                causal_stats(&values, &mask, c.patch_size, c.minimum_scale);
            let mut binary = false;
            if roles[r] == 2 {
                let (loc, scale, is_binary) = instance_stats(&values, &mask, c.minimum_scale);
                locs.fill(loc);
                scales.fill(scale);
                binary = is_binary;
            }
            anchors.push((locs[context_patches - 1], scales[context_patches - 1]));
            for p in 0..n_enc {
                for i in 0..c.patch_size {
                    let t = p * c.patch_size + i;
                    let mut normalized = if mask[t] {
                        (values[t] - locs[p]) / scales[p].max(f32::EPSILON)
                    } else {
                        0.0
                    };
                    if c.use_asinh && !binary && mask[t] {
                        normalized = normalized.asinh();
                    }
                    features.push(normalized);
                    patch_observed[r][p] |= mask[t];
                }
                for i in 0..c.patch_size {
                    features.push(if mask[p * c.patch_size + i] { 0.0 } else { 1.0 });
                }
            }
        }
        let x_enc = self.patch.forward(Tensor::from_data(
            TensorData::new(features, [rows, n_enc, 2 * c.patch_size]),
            &self.device,
        ));
        let x_dec = self
            .query
            .clone()
            .reshape([1, 1, c.d_model])
            .expand([rows, n_dec, c.d_model]);
        let masks = build_masks(
            &patch_observed,
            roles,
            context_patches,
            horizon_patches,
            covariates,
            c,
            &self.device,
        );
        let mut hidden = Tensor::cat(vec![x_enc, x_dec], 1);
        for block in &self.blocks {
            hidden = block.forward(hidden, &masks);
        }
        let start = n_enc + if covariates { context_patches } else { 0 };
        let raw = self.head.forward(self.out_norm.forward(hidden).narrow(
            1,
            start,
            horizon_patches,
        ));
        let mut data = raw
            .into_data()
            .try_to_vec::<f32>()
            .map_err(|e| Error::Tensor(e.to_string()))?;
        let nq = c.quantile_knots.len();
        let blen = horizon_patches * c.patch_size;
        for r in 0..rows {
            for t in 0..blen {
                let quantiles = &mut data[(r * blen + t) * nq..(r * blen + t + 1) * nq];
                monotone(quantiles, &c.quantile_knots, c.quantile_width_factor);
                for value in quantiles.iter_mut() {
                    if c.use_asinh {
                        *value = value.sinh();
                    }
                    *value = *value * anchors[r].1 + anchors[r].0;
                }
                clamp_nonfinite(quantiles);
            }
        }
        Ok(data)
    }
}

struct Prepared {
    rows: Vec<Vec<f32>>,
    observed: Vec<Vec<bool>>,
    roles: Vec<u8>,
    future: Vec<Vec<f32>>,
    future_mask: Vec<Vec<bool>>,
    nonnegative: Vec<bool>,
}
impl Prepared {
    fn new(series: &crate::ForecastSeries, horizon: usize, c: &Config) -> Result<Self> {
        let time = series.targets.first().map(Vec::len).unwrap_or(0);
        if time == 0 || series.targets.iter().any(|r| r.len() != time) {
            return Err(invalid(
                "Timestral targets must have a nonempty, shared time dimension",
            ));
        }
        if series
            .observed_mask
            .as_ref()
            .is_some_and(|m| m.len() != series.targets.len() || m.iter().any(|r| r.len() != time))
        {
            return Err(invalid(
                "Timestral observation mask must match target dimensions",
            ));
        }
        if series.past_covariates.iter().any(|r| r.len() != time)
            || series
                .known_covariates
                .iter()
                .any(|r| r.len() != time + horizon)
        {
            return Err(invalid(
                "Timestral covariates must cover context, or context plus horizon for known covariates",
            ));
        }
        if (!series.past_covariates.is_empty() || !series.known_covariates.is_empty())
            && !c.covariate_enabled
        {
            return Err(invalid(
                "This Timestral checkpoint does not support covariates",
            ));
        }
        if !series.known_covariates.is_empty() && !c.known_covariates() {
            return Err(invalid(
                "This Timestral attention schedule cannot condition on known future covariates",
            ));
        }
        let future_length = checked_elements(
            &[horizon.div_ceil(c.patch_size), c.patch_size],
            "padded horizon",
        )?;
        let row_count = series
            .targets
            .len()
            .checked_add(series.past_covariates.len())
            .and_then(|count| count.checked_add(series.known_covariates.len()))
            .ok_or_else(|| invalid("Timestral row count overflows"))?;
        let context_patches = time.min(c.context_length).div_ceil(c.patch_size);
        let block = horizon.div_ceil(c.patch_size).min(c.block_size).min(32);
        let covariates = row_count > series.targets.len();
        c.validate_workspace(row_count, context_patches, block, covariates)?;
        crate::types::validate_workspace(
            &[2, row_count, future_length, c.quantile_knots.len()],
            "Timestral mirrored forecasts",
        )?;
        let sequence = context_patches
            .checked_add(block)
            .and_then(|window| window.checked_mul(if covariates { 2 } else { 1 }))
            .ok_or_else(|| invalid("Timestral sequence length overflows"))?;
        checked_elements(&[row_count, horizon, c.quantile_knots.len()], "forecast")?;
        checked_elements(&[row_count, future_length], "future covariates")?;
        checked_elements(
            &[row_count, c.num_heads, sequence, sequence],
            "time attention",
        )?;
        checked_elements(
            &[sequence, c.num_heads, row_count, row_count],
            "variate attention",
        )?;
        checked_elements(&[row_count, sequence, 3, c.d_model], "projected inputs")?;
        let mut rows = Vec::new();
        let mut observed = Vec::new();
        let mut roles = Vec::new();
        let mut future = Vec::new();
        let mut future_mask = Vec::new();
        let mut nonnegative = Vec::new();
        for (v, target) in series.targets.iter().enumerate() {
            let mask: Vec<bool> = target
                .iter()
                .enumerate()
                .map(|(t, x)| {
                    x.is_finite() && series.observed_mask.as_ref().is_none_or(|m| m[v][t])
                })
                .collect();
            nonnegative.push(
                mask.iter().any(|m| *m) && target.iter().zip(&mask).all(|(x, m)| !m || *x >= 0.0),
            );
            let mut last = 0.0;
            let filled = target
                .iter()
                .zip(&mask)
                .map(|(x, m)| {
                    if *m {
                        last = *x;
                    }
                    last
                })
                .collect::<Vec<_>>();
            rows.push(filled);
            observed.push(mask);
            roles.push(0);
            future.push(vec![0.0; future_length]);
            future_mask.push(vec![false; future_length]);
        }
        for (role, covariates) in [(1, &series.past_covariates), (2, &series.known_covariates)] {
            for row in covariates {
                if row.iter().any(|x| x.is_infinite()) {
                    return Err(invalid("Timestral covariates must be finite or missing"));
                }
                rows.push(
                    row[..time]
                        .iter()
                        .map(|x| if x.is_finite() { *x } else { 0.0 })
                        .collect(),
                );
                observed.push(row[..time].iter().map(|x| x.is_finite()).collect());
                roles.push(role);
                let mut values = vec![0.0; future_length];
                let mut mask = vec![false; future_length];
                if role == 2 {
                    for (t, x) in row[time..].iter().enumerate() {
                        if x.is_finite() {
                            values[t] = *x;
                            mask[t] = true;
                        }
                    }
                }
                future.push(values);
                future_mask.push(mask);
            }
        }
        let start = time.saturating_sub(c.context_length);
        let retained = time - start;
        let pad = retained.next_multiple_of(c.patch_size) - retained;
        for r in 0..rows.len() {
            rows[r] = std::iter::repeat_n(0.0, pad)
                .chain(rows[r][start..].iter().copied())
                .collect();
            observed[r] = std::iter::repeat_n(false, pad)
                .chain(observed[r][start..].iter().copied())
                .collect();
        }
        Ok(Self {
            rows,
            observed,
            roles,
            future,
            future_mask,
            nonnegative,
        })
    }
}

fn causal_stats(
    values: &[f32],
    observed: &[bool],
    patch: usize,
    minimum: f64,
) -> (Vec<f32>, Vec<f32>) {
    let mut count = 0.0_f64;
    let mut sum = 0.0_f64;
    let mut mean = 0.0_f64;
    let mut m2 = 0.0_f64;
    let mut locs = Vec::with_capacity(values.len() / patch);
    let mut scales = Vec::with_capacity(locs.capacity());
    for (t, (value, mask)) in values.iter().zip(observed).enumerate() {
        if *mask {
            count += 1.0;
            sum += *value as f64;
            let previous = mean;
            mean = sum / count;
            m2 += (*value as f64 - previous) * (*value as f64 - mean);
        }
        if (t + 1) % patch == 0 {
            locs.push(mean as f32);
            scales.push((m2.max(0.0) / (count - 1.0).max(1.0)).sqrt().max(minimum) as f32);
        }
    }
    (locs, scales)
}

fn instance_stats(values: &[f32], observed: &[bool], minimum: f64) -> (f32, f32, bool) {
    let count = observed.iter().filter(|m| **m).count();
    if count > 0
        && values
            .iter()
            .zip(observed)
            .all(|(x, m)| !m || *x == 0.0 || *x == 1.0)
    {
        return (0.0, 1.0, true);
    }
    let n = count.max(1) as f32;
    let mean = values
        .iter()
        .zip(observed)
        .filter_map(|(x, m)| m.then_some(*x))
        .sum::<f32>()
        / n;
    let variance = values
        .iter()
        .zip(observed)
        .filter_map(|(x, m)| m.then_some((*x - mean).powi(2)))
        .sum::<f32>()
        / n;
    (mean, variance.sqrt().max(minimum as f32), false)
}

fn monotone(values: &mut [f32], levels: &[f32], width: f32) {
    let m = levels.iter().position(|q| *q == 0.5).unwrap();
    let median = values[m];
    let mut offset = 0.0;
    let softplus = |x: f32| if x > 20.0 { x } else { x.exp().ln_1p() };
    for i in (0..m).rev() {
        offset += width * (levels[i + 1] - levels[i]) * softplus(values[i]);
        values[i] = (median - offset).max(-f32::MAX);
    }
    offset = 0.0;
    for i in m + 1..values.len() {
        offset += width * (levels[i] - levels[i - 1]) * softplus(values[i]);
        values[i] = (median + offset).min(f32::MAX);
    }
}

fn clamp_nonfinite(values: &mut [f32]) {
    let min = values
        .iter()
        .copied()
        .filter(|x| x.is_finite())
        .reduce(f32::min)
        .unwrap_or(0.0);
    let max = values
        .iter()
        .copied()
        .filter(|x| x.is_finite())
        .reduce(f32::max)
        .unwrap_or(0.0);
    for x in values {
        if *x == f32::INFINITY {
            *x = max;
        } else if *x == f32::NEG_INFINITY {
            *x = min;
        }
    }
}

fn refine(
    values: &mut [f32],
    context: &[Vec<f32>],
    observed: &[Vec<bool>],
    roles: &[u8],
    c: &Config,
    blen: usize,
) {
    let np = blen / c.patch_size;
    if np <= 1 {
        return;
    }
    let q = c.quantile_knots.len();
    let clip = (f32::MAX as f64).sqrt() / 1e3;
    for r in 0..context.len() {
        if roles[r] != 0 {
            continue;
        }
        let (locs, scales) = causal_stats(&context[r], &observed[r], c.patch_size, c.minimum_scale);
        let loc0 = *locs.last().unwrap();
        let scale0 = *scales.last().unwrap();
        let mut count = observed[r].iter().filter(|v| **v).count() as f64;
        let mut mean = context[r]
            .iter()
            .zip(&observed[r])
            .filter_map(|(v, m)| m.then_some(*v as f64))
            .sum::<f64>()
            / count.max(1.0);
        let mut m2 = context[r]
            .iter()
            .zip(&observed[r])
            .filter_map(|(v, m)| m.then_some((*v as f64 - mean).powi(2)))
            .sum::<f64>();
        let offset = mean;
        for p in 0..np {
            let scale = (m2.max(0.0) / (count - 1.0).max(1.0))
                .sqrt()
                .max(c.minimum_scale);
            let mut folded = Vec::with_capacity(c.patch_size);
            for t in p * c.patch_size..(p + 1) * c.patch_size {
                let base = (r * blen + t) * q;
                let sinh_median =
                    (values[base + c.median_index()] as f64 - loc0 as f64) / scale0 as f64;
                folded.push((sinh_median * scale + mean - offset).clamp(-clip, clip) + offset);
                let (frame_loc, frame_scale) = if p == 0 {
                    (loc0, scale0)
                } else {
                    (mean as f32, scale as f32)
                };
                for x in &mut values[base..base + q] {
                    *x = ((*x - loc0) / scale0) * frame_scale + frame_loc;
                }
                clamp_nonfinite(&mut values[base..base + q]);
            }
            let n = c.patch_size as f64;
            let next_mean = folded.iter().sum::<f64>() / n;
            let next_m2 = folded.iter().map(|v| (v - next_mean).powi(2)).sum::<f64>();
            let delta = next_mean - mean;
            m2 += next_m2 + delta * delta * count * n / (count + n);
            mean += delta * n / (count + n);
            count += n;
        }
    }
}

fn build_masks(
    observed: &[Vec<bool>],
    roles: &[u8],
    context: usize,
    horizon: usize,
    covariates: bool,
    c: &Config,
    device: &Device,
) -> Masks {
    let rows = observed.len();
    let n_enc = observed[0].len();
    let n_dec = if covariates { n_enc } else { horizon };
    let seq = n_enc + n_dec;
    let key_valid: Vec<Vec<bool>> = observed
        .iter()
        .map(|row| {
            let mut started = false;
            row.iter()
                .map(|v| {
                    started |= *v;
                    *v || (c.attend_missing_patches && started)
                })
                .collect()
        })
        .collect();
    let first = (0..n_enc)
        .find(|p| observed.iter().any(|r| r[*p]))
        .unwrap_or(0);
    let mut time = Vec::with_capacity(rows * seq * seq);
    for r in 0..rows {
        for query in 0..seq {
            for key in 0..seq {
                let allowed = if query < n_enc {
                    key < n_enc
                        && (query == key
                            || (key_valid[r][key]
                                && (key <= query || (covariates && roles[r] == 2))))
                } else if key < n_enc {
                    if covariates {
                        let p = query - n_enc;
                        let active = p >= context;
                        let anchor =
                            context + p.saturating_sub(context) / c.block_size * c.block_size;
                        active && key < anchor && key_valid[r][key]
                    } else {
                        key_valid[r][key]
                    }
                } else if covariates {
                    let qp = query - n_enc;
                    let kp = key - n_enc;
                    qp == kp
                        || (qp >= context
                            && kp >= context
                            && kp <= qp
                            && (qp - context) / c.block_size == (kp - context) / c.block_size)
                } else {
                    key <= query
                };
                time.push(!allowed);
            }
        }
    }
    let mut variate = Vec::with_capacity(seq * rows * rows);
    for slot in 0..seq {
        for query in 0..rows {
            for key in 0..rows {
                let forbidden = covariates
                    && ((roles[query] != 0 && roles[key] == 0)
                        || (roles[query] == 2 && roles[key] != 2));
                let allowed =
                    query == key || (!forbidden && (slot >= n_enc || key_valid[key][slot]));
                variate.push(!allowed);
            }
        }
    }
    let head = c.head_dim();
    let half = head / 2;
    let rotary = c.rope_dim();
    let mut cos = Vec::with_capacity(seq * head);
    let mut sin = Vec::with_capacity(seq * head);
    for position in 0..seq {
        let p = if covariates && position >= n_enc {
            position - n_enc
        } else {
            position
        };
        for dim in 0..head {
            let i = dim % half;
            let inverse = if i < rotary / 2 {
                (1.0 / c.rope_theta.powf((2 * i) as f64 / rotary as f64)) as f32
            } else {
                0.0
            };
            let angle = (p as f32 - first as f32) * inverse;
            cos.push(angle.cos());
            sin.push(angle.sin());
        }
    }
    Masks {
        time: Tensor::from_data(TensorData::new(time, [rows, 1, seq, seq]), device),
        variate: Tensor::from_data(TensorData::new(variate, [seq, 1, rows, rows]), device),
        cos: Tensor::from_data(TensorData::new(cos, [1, seq, 1, head]), device),
        sin: Tensor::from_data(TensorData::new(sin, [1, seq, 1, head]), device),
        split: c.split_context_horizon_attn.then_some(n_enc),
    }
}

#[cfg(all(test, feature = "cpu"))]
mod tests;
