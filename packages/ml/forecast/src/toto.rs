// Native inference port of Datadog Toto 2.0, Copyright 2026 Datadog, Inc.
// Upstream: https://github.com/DataDog/toto, Apache-2.0.
use crate::{
    Error, ForecastRequest, ForecastResult, PointStatistic, Result, SeriesForecast,
    weights::Weights,
};
use burn::tensor::{
    Device, Int, Tensor, TensorData,
    activation::{silu, softmax},
};
use serde::Deserialize;
use std::path::Path;

const QUANTILES: [f32; 9] = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9];
const MAX_PATCHES: usize = 8192;

#[derive(Debug, Deserialize)]
struct Config {
    patch_size: usize,
    d_model: usize,
    num_heads: usize,
    num_layers: usize,
    layer_group_size: usize,
    num_variate_layers_per_group: usize,
    variate_layer_first: bool,
    #[serde(default)]
    dropout_p: f64,
    #[serde(default = "default_eps")]
    norm_eps: f64,
    #[serde(default)]
    attn_bias: bool,
    #[serde(default)]
    mlp_bias: bool,
    #[serde(default = "one")]
    num_output_patches: usize,
    #[serde(default)]
    d_ff: Option<usize>,
    #[serde(default)]
    qk_dim: Option<usize>,
    #[serde(default)]
    v_dim: Option<usize>,
    #[serde(default)]
    num_groups: Option<usize>,
    #[serde(default = "yes")]
    qk_norm: bool,
    #[serde(default)]
    norm_include_weight: bool,
    #[serde(default)]
    qk_norm_include_weight: Option<bool>,
    #[serde(default)]
    per_dim_scale: bool,
    #[serde(default)]
    use_xpos: bool,
}
fn default_eps() -> f64 {
    5e-5
}
fn one() -> usize {
    1
}
fn yes() -> bool {
    true
}

impl Config {
    fn validate(&self) -> Result<()> {
        let projection_width = self
            .num_heads
            .checked_add(self.groups())
            .and_then(|heads| heads.checked_mul(self.qdim()))
            .and_then(|qk| {
                self.vdim()
                    .checked_mul(self.groups())
                    .and_then(|v| qk.checked_add(v))
            });
        if self.d_model.checked_mul(8).is_none()
            || self.patch_size.checked_mul(MAX_PATCHES).is_none()
            || self
                .d_ff
                .is_some_and(|width| width.checked_mul(2).is_none())
            || projection_width.is_none()
            || self.vdim().checked_mul(self.num_heads).is_none()
            || self.patch_size == 0
            || self.d_model == 0
            || self.num_heads == 0
            || self.num_layers == 0
            || self.layer_group_size == 0
            || self.num_layers % self.layer_group_size != 0
            || self.num_variate_layers_per_group > self.layer_group_size
            || self.d_model % self.num_heads != 0
            || self.groups() == 0
            || self.num_heads % self.groups() != 0
            || self.qdim() < 4
            || self.qdim() % 4 != 0
            || self.vdim() == 0
            || self.ffdim() == 0
            || !self.norm_eps.is_finite()
            || self.norm_eps <= 0.0
            || self.dropout_p != 0.0
            || self.num_output_patches != 1
        {
            return Err(Error::Invalid("Unsupported or invalid Toto 2 configuration; inference requires positive dimensions, compatible attention groups and one output patch".into()));
        }
        Ok(())
    }
    fn qdim(&self) -> usize {
        self.qk_dim.unwrap_or(self.d_model / self.num_heads.max(1))
    }
    fn vdim(&self) -> usize {
        self.v_dim.unwrap_or(self.qdim())
    }
    fn groups(&self) -> usize {
        self.num_groups.unwrap_or(self.num_heads)
    }
    fn ffdim(&self) -> usize {
        self.d_ff
            .unwrap_or(((4 * self.d_model * 2 / 3) + 7) / 8 * 8)
    }
    fn variate_layer(&self, index: usize) -> bool {
        let position = index % self.layer_group_size;
        if self.variate_layer_first {
            position < self.num_variate_layers_per_group
        } else {
            position >= self.layer_group_size - self.num_variate_layers_per_group
        }
    }
}

#[derive(Debug)]
struct Linear {
    weight: Tensor<2>,
    bias: Option<Tensor<1>>,
    scale: f64,
}
impl Linear {
    fn load(
        weights: &Weights,
        name: &str,
        input: usize,
        output: usize,
        bias: bool,
        readout: bool,
        device: &Device,
    ) -> Result<Self> {
        let weight =
            tensor::<2>(weights, &format!("{name}.weight"), [output, input], device)?.transpose();
        let bias = if bias {
            Some(tensor::<1>(
                weights,
                &format!("{name}.bias"),
                [output],
                device,
            )?)
        } else {
            None
        };
        Ok(Self {
            weight,
            bias,
            scale: if readout {
                1.0 / input as f64
            } else {
                1.0 / (input as f64).sqrt()
            },
        })
    }
    fn forward(&self, x: Tensor<3>) -> Tensor<3> {
        let mut output = x.matmul(self.weight.clone().unsqueeze::<3>());
        if let Some(bias) = &self.bias {
            output = output + bias.clone().unsqueeze::<3>();
        }
        output * self.scale
    }
}

#[derive(Debug)]
struct Norm {
    weight: Option<Tensor<1>>,
    eps: f64,
}
impl Norm {
    fn load(
        weights: &Weights,
        name: &str,
        width: usize,
        weighted: bool,
        eps: f64,
        device: &Device,
    ) -> Result<Self> {
        Ok(Self {
            weight: if weighted {
                Some(tensor::<1>(
                    weights,
                    &format!("{name}.weight"),
                    [width],
                    device,
                )?)
            } else {
                None
            },
            eps,
        })
    }
    fn forward<const D: usize>(&self, x: Tensor<D>) -> Tensor<D> {
        let mut out = x.clone() / (x.square().mean_dim(D - 1) + self.eps).sqrt();
        if let Some(weight) = &self.weight {
            out = out * weight.clone().unsqueeze::<D>();
        }
        out
    }
}

#[derive(Debug)]
struct ResidualMlp {
    first: Linear,
    second: Linear,
    skip: Linear,
}
impl ResidualMlp {
    fn load(
        weights: &Weights,
        prefix: &str,
        input: usize,
        hidden: usize,
        output: usize,
        readout: bool,
        device: &Device,
    ) -> Result<Self> {
        Ok(Self {
            first: Linear::load(
                weights,
                &format!("{prefix}.linear1"),
                input,
                hidden,
                true,
                false,
                device,
            )?,
            second: Linear::load(
                weights,
                &format!("{prefix}.linear2"),
                hidden,
                output,
                true,
                readout,
                device,
            )?,
            skip: Linear::load(
                weights,
                &format!("{prefix}.skip_proj"),
                input,
                output,
                true,
                readout,
                device,
            )?,
        })
    }
    fn forward(&self, input: Tensor<3>) -> Tensor<3> {
        // Only the patch projections use unit-scaled SiLU. Transformer SwiGLU uses ordinary SiLU.
        let scale = (0.8 * (2.0 / (1.0 - 1.0 / std::f64::consts::PI)).sqrt().ln()
            + 0.2 * 2.0_f64.ln())
        .exp();
        let main = self
            .second
            .forward(silu(self.first.forward(input.clone())) * scale);
        (main + self.skip.forward(input)) / 2.0_f64.sqrt()
    }
}

#[derive(Debug)]
struct Attention {
    input: Linear,
    output: Linear,
    q_norm: Option<Norm>,
    k_norm: Option<Norm>,
    per_dim_scale: Option<Tensor<1>>,
    heads: usize,
    groups: usize,
    qdim: usize,
    vdim: usize,
    use_xpos: bool,
}
impl Attention {
    fn load(weights: &Weights, prefix: &str, c: &Config, device: &Device) -> Result<Self> {
        let norm_weight = c.qk_norm_include_weight.unwrap_or(c.norm_include_weight);
        let per_dim_scale = if c.per_dim_scale {
            let raw = tensor::<1>(
                weights,
                &format!("{prefix}._pds.per_dim_scale"),
                [c.qdim()],
                device,
            )?;
            let raw = raw
                .to_data()
                .try_to_vec::<f32>()
                .map_err(|e| Error::Tensor(e.to_string()))?;
            let scale = raw
                .into_iter()
                .map(|x| {
                    if x > 20.0 {
                        x / std::f32::consts::LN_2
                    } else {
                        x.exp().ln_1p() / std::f32::consts::LN_2
                    }
                })
                .collect::<Vec<_>>();
            Some(Tensor::from_data(
                TensorData::new(scale, [c.qdim()]),
                device,
            ))
        } else {
            None
        };
        Ok(Self {
            input: Linear::load(
                weights,
                &format!("{prefix}.in_proj"),
                c.d_model,
                c.qdim() * (c.num_heads + c.groups()) + c.vdim() * c.groups(),
                c.attn_bias,
                false,
                device,
            )?,
            output: Linear::load(
                weights,
                &format!("{prefix}.out_proj"),
                c.vdim() * c.num_heads,
                c.d_model,
                c.attn_bias,
                false,
                device,
            )?,
            q_norm: if c.qk_norm {
                Some(Norm::load(
                    weights,
                    &format!("{prefix}.q_norm"),
                    c.qdim(),
                    norm_weight,
                    c.norm_eps,
                    device,
                )?)
            } else {
                None
            },
            k_norm: if c.qk_norm {
                Some(Norm::load(
                    weights,
                    &format!("{prefix}.k_norm"),
                    c.qdim(),
                    norm_weight,
                    c.norm_eps,
                    device,
                )?)
            } else {
                None
            },
            per_dim_scale,
            heads: c.num_heads,
            groups: c.groups(),
            qdim: c.qdim(),
            vdim: c.vdim(),
            use_xpos: c.use_xpos,
        })
    }
    fn forward(&self, x: Tensor<3>, mask: Tensor<4>, temporal: bool) -> Tensor<3> {
        let [batch, seq, _] = x.dims();
        let projected = self.input.forward(x);
        let qend = self.heads * self.qdim;
        let kend = qend + self.groups * self.qdim;
        let vend = kend + self.groups * self.vdim;
        let mut q = projected
            .clone()
            .slice([0..batch, 0..seq, 0..qend])
            .reshape([batch, seq, self.heads, self.qdim])
            .swap_dims(1, 2);
        let mut k = projected
            .clone()
            .slice([0..batch, 0..seq, qend..kend])
            .reshape([batch, seq, self.groups, self.qdim])
            .swap_dims(1, 2);
        let mut v = projected
            .slice([0..batch, 0..seq, kend..vend])
            .reshape([batch, seq, self.groups, self.vdim])
            .swap_dims(1, 2);
        if let Some(norm) = &self.q_norm {
            q = norm.forward(q);
        }
        if let Some(norm) = &self.k_norm {
            k = norm.forward(k);
        }
        if let Some(scale) = &self.per_dim_scale {
            q = q * scale.clone().unsqueeze::<4>();
        }
        if temporal {
            q = rotary(q, self.use_xpos, false);
            k = rotary(k, self.use_xpos, true);
        }
        if self.groups != self.heads {
            let indices = Tensor::<1, Int>::from_data(
                TensorData::new(
                    (0..self.heads)
                        .map(|h| (h / (self.heads / self.groups)) as i32)
                        .collect::<Vec<_>>(),
                    [self.heads],
                ),
                &k.device(),
            );
            k = k.select(1, indices.clone());
            v = v.select(1, indices);
        }
        let probabilities = softmax(q.matmul(k.swap_dims(2, 3)) / self.qdim as f64 + mask, 3);
        self.output
            .forward(probabilities.matmul(v).swap_dims(1, 2).reshape([
                batch,
                seq,
                self.heads * self.vdim,
            ]))
    }
}

fn rotary(input: Tensor<4>, xpos: bool, inverse: bool) -> Tensor<4> {
    let [batch, heads, seq, width] = input.dims();
    let rotary_width = width / 2;
    let device = input.device();
    let mut cosine = Vec::with_capacity(seq * rotary_width);
    let mut sine = Vec::with_capacity(seq * rotary_width);
    for position in 0..seq {
        for dim in 0..rotary_width {
            let pair = dim / 2 * 2;
            let angle = position as f32 / 10000_f32.powf(pair as f32 / rotary_width as f32);
            let mut scale = 1.0;
            if xpos {
                let base = (pair as f32 + 0.4 * rotary_width as f32) / (1.4 * rotary_width as f32);
                scale = base.powf((position as f32 - (seq / 2) as f32) / 256.0);
                if inverse {
                    scale = 1.0 / scale;
                }
            }
            cosine.push(angle.cos() * scale);
            sine.push(angle.sin() * scale * if dim % 2 == 0 { -1.0 } else { 1.0 });
        }
    }
    let cos = Tensor::from_data(TensorData::new(cosine, [1, 1, seq, rotary_width]), &device);
    let sin = Tensor::from_data(TensorData::new(sine, [1, 1, seq, rotary_width]), &device);
    let front = input
        .clone()
        .slice([0..batch, 0..heads, 0..seq, 0..rotary_width]);
    let pairs = Tensor::<1, Int>::from_data(
        TensorData::new(
            (0..rotary_width)
                .map(|i| (i ^ 1) as i32)
                .collect::<Vec<_>>(),
            [rotary_width],
        ),
        &device,
    );
    let rotated = front.clone() * cos + front.select(3, pairs) * sin;
    Tensor::cat(
        vec![
            rotated,
            input.slice([0..batch, 0..heads, 0..seq, rotary_width..width]),
        ],
        3,
    )
}

#[derive(Debug)]
struct Layer {
    attention: Attention,
    norm1: Norm,
    norm2: Norm,
    fc1: Linear,
    fc2: Linear,
    attn_tau: f64,
    mlp_tau: f64,
    variate: bool,
}
impl Layer {
    fn load(weights: &Weights, index: usize, c: &Config, device: &Device) -> Result<Self> {
        let prefix = format!("transformer.layers.{index}");
        let attn_tau = weights.scalar(&format!("{prefix}.attn_tau"))? as f64;
        let mlp_tau = weights.scalar(&format!("{prefix}.mlp_tau"))? as f64;
        if !attn_tau.is_finite() || !mlp_tau.is_finite() {
            return Err(Error::Invalid("Non-finite Toto residual scale".into()));
        }
        Ok(Self {
            attention: Attention::load(weights, &format!("{prefix}.attn"), c, device)?,
            norm1: Norm::load(
                weights,
                &format!("{prefix}.norm1"),
                c.d_model,
                c.norm_include_weight,
                c.norm_eps,
                device,
            )?,
            norm2: Norm::load(
                weights,
                &format!("{prefix}.norm2"),
                c.d_model,
                c.norm_include_weight,
                c.norm_eps,
                device,
            )?,
            fc1: Linear::load(
                weights,
                &format!("{prefix}.ffn.fc1"),
                c.d_model,
                2 * c.ffdim(),
                c.mlp_bias,
                false,
                device,
            )?,
            fc2: Linear::load(
                weights,
                &format!("{prefix}.ffn.fc2"),
                c.ffdim(),
                c.d_model,
                c.mlp_bias,
                false,
                device,
            )?,
            attn_tau,
            mlp_tau,
            variate: c.variate_layer(index),
        })
    }
    fn forward(&self, mut x: Tensor<3>, mask: Tensor<4>) -> Tensor<3> {
        if self.variate {
            x = x.swap_dims(0, 1);
        }
        let attn = self
            .attention
            .forward(self.norm1.forward(x.clone()), mask, !self.variate);
        x = residual(attn, x, self.attn_tau);
        let z = self.fc1.forward(self.norm2.forward(x.clone()));
        let [batch, seq, width] = z.dims();
        let gate = z.clone().slice([0..batch, 0..seq, 0..width / 2]);
        let value = z.slice([0..batch, 0..seq, width / 2..width]);
        x = residual(self.fc2.forward(gate * silu(value)), x, self.mlp_tau);
        if self.variate { x.swap_dims(0, 1) } else { x }
    }
}
fn residual(main: Tensor<3>, skip: Tensor<3>, tau: f64) -> Tensor<3> {
    (main * tau + skip) / (1.0 + tau * tau).sqrt()
}
fn tensor<const D: usize>(
    weights: &Weights,
    name: &str,
    shape: [usize; D],
    device: &Device,
) -> Result<Tensor<D>> {
    if weights.shape(name)? != shape {
        return Err(Error::Invalid(format!(
            "Toto tensor {name} has an incompatible shape; expected {shape:?}"
        )));
    }
    weights.tensor::<D>(name, device)
}

/// Toto 2.0 with single-pass decoding, matching the upstream default forecast path.
#[derive(Debug)]
pub struct Toto2 {
    config: Config,
    patch_projection: ResidualMlp,
    layers: Vec<Layer>,
    output_norm: Norm,
    output_projection: ResidualMlp,
    device: Device,
}
impl Toto2 {
    pub fn capabilities(&self) -> crate::ForecastCapabilities {
        let joint = self.config.num_variate_layers_per_group > 0;
        crate::ForecastCapabilities {
            family: crate::ForecastFamily::Toto2,
            max_context: (MAX_PATCHES - 1) * self.config.patch_size,
            max_horizon: Some(
                ((MAX_PATCHES - 1) * self.config.patch_size).min(crate::MAX_FORECAST_HORIZON),
            ),
            native_horizon: self.config.patch_size,
            quantile_levels: QUANTILES.to_vec(),
            joint_multivariate: joint,
            past_covariates: joint,
            known_covariates: joint,
        }
    }

    pub fn load(directory: &Path, device: &Device) -> Result<Self> {
        let config: Config =
            serde_json::from_slice(&std::fs::read(directory.join("config.json"))?)?;
        config.validate()?;
        let weights = Weights::open(directory)?;
        let patch_projection = ResidualMlp::load(
            &weights,
            "patch_proj",
            2 * config.patch_size,
            4 * config.d_model,
            config.d_model,
            false,
            device,
        )?;
        let layers = (0..config.num_layers)
            .map(|i| Layer::load(&weights, i, &config, device))
            .collect::<Result<Vec<_>>>()?;
        let output_norm = Norm::load(
            &weights,
            "transformer.out_norm",
            config.d_model,
            config.norm_include_weight,
            config.norm_eps,
            device,
        )?;
        let output_projection = ResidualMlp::load(
            &weights,
            "output_head.param_projection.proj",
            config.d_model,
            4 * config.d_model,
            config.patch_size * QUANTILES.len(),
            true,
            device,
        )?;
        Ok(Self {
            config,
            patch_projection,
            layers,
            output_norm,
            output_projection,
            device: device.clone(),
        })
    }

    pub fn forecast(&self, request: &ForecastRequest) -> Result<ForecastResult> {
        request.validate_for_quantiles(QUANTILES.len())?;
        if self.config.num_variate_layers_per_group == 0
            && request.series.iter().any(|series| {
                !series.past_covariates.is_empty() || !series.known_covariates.is_empty()
            })
        {
            return Err(Error::Invalid(
                "This Toto configuration has no variate attention and cannot use covariates".into(),
            ));
        }
        let levels = if request.quantile_levels.is_empty() {
            QUANTILES.to_vec()
        } else {
            request.quantile_levels.clone()
        };
        if levels
            .iter()
            .any(|q| !q.is_finite() || *q < QUANTILES[0] || *q > QUANTILES[8])
        {
            return Err(Error::Invalid(
                "Toto quantiles must lie between 0.1 and 0.9".into(),
            ));
        }
        let mut forecasts = Vec::with_capacity(request.series.len());
        for series in &request.series {
            let channels = series
                .targets
                .len()
                .checked_add(series.past_covariates.len())
                .and_then(|count| count.checked_add(series.known_covariates.len()))
                .ok_or_else(|| Error::Invalid("Toto channel count overflow".into()))?;
            validate_workload(
                &self.config,
                channels,
                series.targets[0].len(),
                request.horizon,
            )?;
            let has_time_attention =
                self.config.num_variate_layers_per_group < self.config.layer_group_size;
            let has_variate_attention = self.config.num_variate_layers_per_group > 0;
            let prepared = Prepared::new(
                series,
                request.horizon,
                self.config.patch_size,
                has_time_attention,
                has_variate_attention,
            )?;
            let mut x = self.patch_projection.forward(Tensor::from_data(
                TensorData::new(
                    prepared.embeddings,
                    [
                        prepared.variables,
                        prepared.patches,
                        2 * self.config.patch_size,
                    ],
                ),
                &self.device,
            ));
            let time_mask = has_time_attention.then(|| {
                Tensor::from_data(
                    TensorData::new(
                        prepared.time_mask,
                        [prepared.variables, 1, prepared.patches, prepared.patches],
                    ),
                    &self.device,
                )
            });
            let var_mask = has_variate_attention.then(|| {
                Tensor::from_data(
                    TensorData::new(
                        prepared.var_mask,
                        [prepared.patches, 1, prepared.variables, prepared.variables],
                    ),
                    &self.device,
                )
            });
            for layer in &self.layers {
                x = layer.forward(
                    x,
                    if layer.variate {
                        var_mask
                            .as_ref()
                            .expect("variate mask required by config")
                            .clone()
                    } else {
                        time_mask
                            .as_ref()
                            .expect("time mask required by config")
                            .clone()
                    },
                );
            }
            // Each token predicts the next patch; the final placeholder has no requested output.
            let anchors = self.output_norm.forward(x).slice([
                0..prepared.variables,
                prepared.context_patches - 1..prepared.patches - 1,
                0..self.config.d_model,
            ]);
            let raw = self
                .output_projection
                .forward(anchors)
                .to_data()
                .try_to_vec::<f32>()
                .map_err(|e| Error::Tensor(e.to_string()))?;
            let mut point = vec![vec![0.0; request.horizon]; series.targets.len()];
            let mut quantiles =
                vec![vec![vec![0.0; levels.len()]; request.horizon]; series.targets.len()];
            for variable in 0..series.targets.len() {
                for patch in 0..prepared.forecast_patches {
                    let loc = prepared.locations
                        [variable * prepared.patches + prepared.context_patches + patch];
                    let scale = prepared.scales
                        [variable * prepared.patches + prepared.context_patches + patch];
                    // Upstream replaces infinities using finite extrema within each quantile patch.
                    let mut real = vec![[0.0_f32; 9]; self.config.patch_size];
                    for (step, values) in real.iter_mut().enumerate() {
                        for (q, value) in values.iter_mut().enumerate() {
                            let index = ((variable * prepared.forecast_patches + patch)
                                * self.config.patch_size
                                + step)
                                * 9
                                + q;
                            *value = raw[index].sinh() * scale + loc;
                        }
                    }
                    replace_infinities(&mut real)?;
                    for (step, values) in real.iter_mut().enumerate() {
                        let time = patch * self.config.patch_size + step;
                        if time >= request.horizon {
                            break;
                        }
                        values.sort_by(f32::total_cmp);
                        point[variable][time] = values[4];
                        for (index, &level) in levels.iter().enumerate() {
                            quantiles[variable][time][index] = interpolate(values, level);
                        }
                    }
                }
            }
            forecasts.push(SeriesForecast {
                id: series.id.clone(),
                point,
                quantiles,
            });
        }
        Ok(ForecastResult {
            series: forecasts,
            quantile_levels: levels,
            point_statistic: PointStatistic::Median,
        })
    }
}

fn validate_workload(
    config: &Config,
    channels: usize,
    history: usize,
    horizon: usize,
) -> Result<()> {
    use crate::types::validate_workspace;
    let forecast_patches = horizon.div_ceil(config.patch_size);
    let patches = history
        .div_ceil(config.patch_size)
        .checked_add(forecast_patches)
        .ok_or_else(|| Error::Invalid("Toto sequence length overflow".into()))?;
    if patches > MAX_PATCHES {
        return Err(Error::Invalid(format!(
            "Toto context plus horizon exceeds {MAX_PATCHES} patches"
        )));
    }
    if config.num_variate_layers_per_group < config.layer_group_size {
        validate_workspace(
            &[channels, config.num_heads, patches, patches],
            "Toto time attention",
        )?;
    }
    if config.num_variate_layers_per_group > 0 {
        validate_workspace(
            &[patches, config.num_heads, channels, channels],
            "Toto variate attention",
        )?;
    }
    validate_workspace(
        &[channels, patches, config.patch_size, 2],
        "Toto patch embedding",
    )?;
    validate_workspace(
        &[channels, patches, config.d_model, 4],
        "Toto patch projection",
    )?;
    validate_workspace(
        &[channels, patches, config.ffdim(), 2],
        "Toto feedforward projection",
    )?;
    let qkv_width =
        (config.num_heads + config.groups()) * config.qdim() + config.groups() * config.vdim();
    validate_workspace(&[channels, patches, qkv_width], "Toto attention projection")?;
    validate_workspace(
        &[channels, patches, config.num_heads, config.vdim()],
        "Toto attention values",
    )?;
    validate_workspace(
        &[
            channels,
            forecast_patches,
            config.patch_size,
            QUANTILES.len(),
        ],
        "Toto native quantile output",
    )?;
    Ok(())
}

fn interpolate(values: &[f32; 9], level: f32) -> f32 {
    match QUANTILES.iter().position(|q| *q >= level) {
        Some(0) => values[0],
        Some(i) => {
            let fraction = (level - QUANTILES[i - 1]) / (QUANTILES[i] - QUANTILES[i - 1]);
            values[i - 1] + fraction * (values[i] - values[i - 1])
        }
        None => values[8],
    }
}

fn replace_infinities(patch: &mut [[f32; 9]]) -> Result<()> {
    for q in 0..9 {
        let finite: Vec<f32> = patch
            .iter()
            .map(|x| x[q])
            .filter(|x| x.is_finite())
            .collect();
        if finite.len() != patch.len() {
            if finite.is_empty() || patch.iter().any(|x| x[q].is_nan()) {
                return Err(Error::Invalid(
                    "Toto produced non-finite forecast values".into(),
                ));
            }
            let min = finite.iter().copied().fold(f32::INFINITY, f32::min);
            let max = finite.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            for values in patch.iter_mut() {
                if values[q] == f32::INFINITY {
                    values[q] = max;
                } else if values[q] == f32::NEG_INFINITY {
                    values[q] = min;
                }
            }
        }
    }
    Ok(())
}

struct Prepared {
    variables: usize,
    patches: usize,
    context_patches: usize,
    forecast_patches: usize,
    embeddings: Vec<f32>,
    locations: Vec<f32>,
    scales: Vec<f32>,
    time_mask: Vec<f32>,
    var_mask: Vec<f32>,
}
impl Prepared {
    fn new(
        series: &crate::ForecastSeries,
        horizon: usize,
        patch_size: usize,
        time_attention: bool,
        variate_attention: bool,
    ) -> Result<Self> {
        let time = series.targets[0].len();
        let context_patches = time.div_ceil(patch_size);
        let forecast_patches = horizon.div_ceil(patch_size);
        let patches = context_patches
            .checked_add(forecast_patches)
            .ok_or_else(|| Error::Invalid("Toto sequence length overflow".into()))?;
        if patches > MAX_PATCHES {
            return Err(Error::Invalid(format!(
                "Toto context plus horizon exceeds {MAX_PATCHES} patches"
            )));
        }
        let context_len = context_patches * patch_size;
        let left_pad = context_len - time;
        let total_len = patches * patch_size;
        let targets = series.targets.len();
        let past = series.past_covariates.len();
        let variables = targets + past + series.known_covariates.len();
        let mut embeddings = Vec::with_capacity(variables * patches * 2 * patch_size);
        let mut locations = Vec::with_capacity(variables * patches);
        let mut scales = Vec::with_capacity(variables * patches);
        let mut groups = vec![0_i32; variables * patches];
        for variable in 0..variables {
            let mut data = vec![0.0_f32; total_len];
            let mut observed = vec![false; total_len];
            if variable < targets {
                for index in 0..time {
                    let valid = series
                        .observed_mask
                        .as_ref()
                        .map_or(true, |mask| mask[variable][index]);
                    data[left_pad + index] = if valid {
                        series.targets[variable][index]
                    } else {
                        0.0
                    };
                    observed[left_pad + index] = valid;
                }
            } else {
                let values = if variable < targets + past {
                    &series.past_covariates[variable - targets]
                } else {
                    &series.known_covariates[variable - targets - past]
                };
                data[left_pad..left_pad + values.len()].copy_from_slice(values);
                observed[left_pad..left_pad + values.len()].fill(true);
            }
            if variable < targets + past {
                // The official forecast path always exposes the final context patch.
                observed[context_len - patch_size..context_len].fill(true);
            }
            let (loc, scale) = causal_scale(&data, &observed, patch_size);
            for patch in 0..patches {
                let start = patch * patch_size;
                let mask = &observed[start..start + patch_size];
                if patch < context_patches && !mask.iter().any(|&value| value) {
                    groups[variable * patches + patch] = -1;
                }
                for offset in 0..patch_size {
                    embeddings.push(if mask[offset] {
                        ((data[start + offset] - loc[patch]) / scale[patch]).asinh()
                    } else {
                        0.0
                    });
                }
                embeddings.extend(mask.iter().map(|&present| if present { 0.0 } else { 1.0 }));
            }
            locations.extend(loc);
            scales.extend(scale);
        }
        let mut time_mask = Vec::new();
        if time_attention {
            time_mask.reserve(variables * patches * patches);
            for variable in 0..variables {
                for query in 0..patches {
                    for key in 0..patches {
                        time_mask.push(
                            if key <= query
                                && groups[variable * patches + query]
                                    == groups[variable * patches + key]
                            {
                                0.0
                            } else {
                                f32::NEG_INFINITY
                            },
                        );
                    }
                }
            }
        }
        let mut var_mask = Vec::new();
        if variate_attention {
            var_mask.reserve(patches * variables * variables);
            for patch in 0..patches {
                for query in 0..variables {
                    for key in 0..variables {
                        var_mask.push(
                            if groups[query * patches + patch] == groups[key * patches + patch] {
                                0.0
                            } else {
                                f32::NEG_INFINITY
                            },
                        );
                    }
                }
            }
        }
        Ok(Self {
            variables,
            patches,
            context_patches,
            forecast_patches,
            embeddings,
            locations,
            scales,
            time_mask,
            var_mask,
        })
    }
}

fn causal_scale(data: &[f32], observed: &[bool], patch_size: usize) -> (Vec<f32>, Vec<f32>) {
    let mut locations = Vec::with_capacity(data.len() / patch_size);
    let mut scales = Vec::with_capacity(data.len() / patch_size);
    let (mut count, mut mean, mut moment) = (0_u64, 0.0_f64, 0.0_f64);
    for (index, (&value, &present)) in data.iter().zip(observed).enumerate() {
        if present {
            count += 1;
            let delta = value as f64 - mean;
            mean += delta / count as f64;
            moment += delta * (value as f64 - mean);
        }
        if (index + 1) % patch_size == 0 {
            locations.push(mean as f32);
            scales.push(
                (moment / count.saturating_sub(1).max(1) as f64)
                    .max(0.0)
                    .sqrt()
                    .max(1e-6) as f32,
            );
        }
    }
    (locations, scales)
}

#[cfg(all(test, feature = "cpu"))]
mod tests {
    use super::*;
    use safetensors::{
        Dtype,
        tensor::{TensorView, serialize_to_file},
    };
    use std::collections::BTreeMap;

    fn values<const D: usize>(tensor: Tensor<D>) -> Vec<f32> {
        tensor.to_data().try_to_vec::<f32>().unwrap()
    }
    fn close(actual: &[f32], expected: &[f32], tolerance: f32) {
        assert_eq!(actual.len(), expected.len());
        for (index, (&a, &e)) in actual.iter().zip(expected).enumerate() {
            assert!((a - e).abs() <= tolerance, "index {index}: {a} != {e}");
        }
    }
    fn series() -> crate::ForecastSeries {
        crate::ForecastSeries {
            id: "sensor".into(),
            targets: vec![vec![1., 3., 5., 7.]],
            observed_mask: None,
            past_covariates: vec![],
            known_covariates: vec![],
        }
    }

    #[test]
    fn causal_scaling_uses_patch_ends_missing_mask_and_sample_variance() {
        let (loc, scale) = causal_scale(
            &[1., 1000., 3., 5., 0., 0.],
            &[true, false, true, true, false, false],
            2,
        );
        close(&loc, &[1., 3., 3.], 1e-6);
        close(&scale, &[1e-6, 2., 2.], 1e-6);
        let (loc, scale) = causal_scale(&[10., 10.], &[false, false], 2);
        assert_eq!(loc, [0.]);
        assert_eq!(scale, [1e-6]);
    }

    #[test]
    fn unit_linear_scales_bias_and_readout_by_the_correct_fan_in() {
        let device = Device::flex();
        let linear = |scale| Linear {
            weight: Tensor::from_data([[1., 2.], [3., 4.], [5., 6.], [7., 8.]], &device),
            bias: Some(Tensor::from_data([2., 4.], &device)),
            scale,
        };
        let input = Tensor::from_data([[[1., 2., 3., 4.]]], &device);
        close(
            &values(linear(0.5).forward(input.clone())),
            &[26., 32.],
            1e-6,
        );
        close(&values(linear(0.25).forward(input)), &[13., 16.], 1e-6);
    }

    #[test]
    fn attention_respects_causality_and_variate_groups() {
        let device = Device::flex();
        let mut projection = vec![0.0; 4 * 12];
        for i in 0..4 {
            projection[i * 12 + 8 + i] = 2.0;
        }
        let mut identity = vec![0.0; 16];
        for i in 0..4 {
            identity[i * 4 + i] = 2.0;
        }
        let attention = Attention {
            input: Linear {
                weight: Tensor::from_data(TensorData::new(projection, [4, 12]), &device),
                bias: None,
                scale: 0.5,
            },
            output: Linear {
                weight: Tensor::from_data(TensorData::new(identity, [4, 4]), &device),
                bias: None,
                scale: 0.5,
            },
            q_norm: None,
            k_norm: None,
            per_dim_scale: None,
            heads: 1,
            groups: 1,
            qdim: 4,
            vdim: 4,
            use_xpos: true,
        };
        let input = Tensor::from_data(
            [[[1., 2., 3., 4.], [3., 4., 5., 6.], [8., 9., 10., 11.]]],
            &device,
        );
        let mask = Tensor::from_data(
            [[[
                [0., f32::NEG_INFINITY, f32::NEG_INFINITY],
                [0., 0., f32::NEG_INFINITY],
                [0., 0., 0.],
            ]]],
            &device,
        );
        close(
            &values(attention.forward(input.clone(), mask, true)),
            &[1., 2., 3., 4., 2., 3., 4., 5., 4., 5., 6., 7.],
            1e-5,
        );
        let mask = Tensor::from_data(
            [[[
                [0., f32::NEG_INFINITY, 0.],
                [f32::NEG_INFINITY, 0., f32::NEG_INFINITY],
                [0., f32::NEG_INFINITY, 0.],
            ]]],
            &device,
        );
        close(
            &values(attention.forward(input, mask, false)),
            &[4.5, 5.5, 6.5, 7.5, 3., 4., 5., 6., 4.5, 5.5, 6.5, 7.5],
            1e-5,
        );
    }

    #[test]
    fn partial_rotary_and_xpos_match_scalar_reference() {
        let device = Device::flex();
        let data = vec![
            1., 2., 3., 4., 5., 6., 7., 8., 2., 3., 4., 5., 6., 7., 8., 9., 3., 4., 5., 6., 7., 8.,
            9., 10.,
        ];
        for (xpos, inverse) in [(false, false), (true, false), (true, true)] {
            let actual = values(rotary(
                Tensor::from_data(TensorData::new(data.clone(), [1, 1, 3, 8]), &device),
                xpos,
                inverse,
            ));
            let mut expected = data.clone();
            for t in 0..3 {
                for pair in 0..2 {
                    let angle = t as f64 / 10000_f64.powf(pair as f64 / 2.0);
                    let base = (2. * pair as f64 + 1.6) / 5.6;
                    let scale = if xpos {
                        base.powf((t as f64 - 1.) / 256. * if inverse { -1. } else { 1. })
                    } else {
                        1.
                    };
                    let offset = t * 8 + pair * 2;
                    expected[offset] = ((data[offset] as f64 * angle.cos()
                        - data[offset + 1] as f64 * angle.sin())
                        * scale) as f32;
                    expected[offset + 1] = ((data[offset] as f64 * angle.sin()
                        + data[offset + 1] as f64 * angle.cos())
                        * scale) as f32;
                }
            }
            close(&actual, &expected, 2e-6);
        }
    }

    #[test]
    fn covariates_and_missing_patches_keep_distinct_attention_masks() {
        let mut input = series();
        input.observed_mask = Some(vec![vec![false, false, true, true]]);
        input.past_covariates = vec![vec![10., 20., 30., 40.]];
        input.known_covariates = vec![vec![2., 4., 6., 8., 10., 12.]];
        let p = Prepared::new(&input, 2, 2, true, true).unwrap();
        assert_eq!((p.variables, p.patches, p.context_patches), (3, 3, 2));
        assert!(p.time_mask[3].is_infinite()); // observed target query cannot read an unobserved patch
        assert!(p.var_mask[1].is_infinite()); // first target patch cannot attend to observed covariate
        close(&p.embeddings[8..12], &[0., 0., 1., 1.], 1e-6); // target forecast placeholder
        close(&p.embeddings[32 + 2..32 + 4], &[0., 0.], 1e-6); // known covariate remains observed
        assert_eq!(p.locations[8], 7.);
    }

    fn checkpoint(directory: &Path) {
        let config = serde_json::json!({"patch_size":2,"d_model":4,"num_heads":1,"num_layers":2,"layer_group_size":2,"num_variate_layers_per_group":1,"variate_layer_first":false,"d_ff":4,"qk_dim":4,"v_dim":4,"attn_bias":true,"qk_norm":true,"norm_include_weight":true,"per_dim_scale":true,"use_xpos":true});
        std::fs::write(
            directory.join("config.json"),
            serde_json::to_vec(&config).unwrap(),
        )
        .unwrap();
        let mut weights: BTreeMap<String, (Vec<usize>, Vec<f32>)> = BTreeMap::new();
        let mut linear = |name: &str, input: usize, output: usize, bias: bool| {
            weights.insert(
                format!("{name}.weight"),
                (vec![output, input], vec![0.; output * input]),
            );
            if bias {
                weights.insert(format!("{name}.bias"), (vec![output], vec![0.; output]));
            }
        };
        for (prefix, input, hidden, output) in [
            ("patch_proj", 4, 16, 4),
            ("output_head.param_projection.proj", 4, 16, 18),
        ] {
            linear(&format!("{prefix}.linear1"), input, hidden, true);
            linear(&format!("{prefix}.linear2"), hidden, output, true);
            linear(&format!("{prefix}.skip_proj"), input, output, true);
        }
        for i in 0..2 {
            let prefix = format!("transformer.layers.{i}");
            linear(&format!("{prefix}.attn.in_proj"), 4, 12, true);
            linear(&format!("{prefix}.attn.out_proj"), 4, 4, true);
            linear(&format!("{prefix}.ffn.fc1"), 4, 8, false);
            linear(&format!("{prefix}.ffn.fc2"), 4, 4, false);
        }
        for i in 0..2 {
            let prefix = format!("transformer.layers.{i}");
            for suffix in [
                "norm1.weight",
                "norm2.weight",
                "attn.q_norm.weight",
                "attn.k_norm.weight",
            ] {
                weights.insert(format!("{prefix}.{suffix}"), (vec![4], vec![1.; 4]));
            }
            weights.insert(
                format!("{prefix}.attn._pds.per_dim_scale"),
                (vec![4], vec![0.; 4]),
            );
            for suffix in ["attn_tau", "mlp_tau"] {
                weights.insert(format!("{prefix}.{suffix}"), (vec![], vec![0.5]));
            }
        }
        weights.insert("transformer.out_norm.weight".into(), (vec![4], vec![1.; 4]));
        let output_bias = (0..18)
            .map(|i| (4. - (i % 9) as f32) * 0.1 * 4. * 2.0_f32.sqrt())
            .collect();
        weights.insert(
            "output_head.param_projection.proj.skip_proj.bias".into(),
            (vec![18], output_bias),
        );
        let bytes: Vec<_> = weights
            .iter()
            .map(|(name, (shape, values))| {
                (
                    name,
                    shape,
                    values
                        .iter()
                        .flat_map(|v| v.to_le_bytes())
                        .collect::<Vec<_>>(),
                )
            })
            .collect();
        let views: Vec<_> = bytes
            .iter()
            .map(|(name, shape, bytes)| {
                (
                    name.as_str(),
                    TensorView::new(Dtype::F32, (*shape).clone(), bytes).unwrap(),
                )
            })
            .collect();
        serialize_to_file(views, None, &directory.join("model.safetensors")).unwrap();
    }

    #[test]
    fn checkpoint_forecast_restores_units_sorts_knots_and_handles_partial_horizon() {
        let directory = tempfile::tempdir().unwrap();
        checkpoint(directory.path());
        let model = Toto2::load(directory.path(), &Device::flex()).unwrap();
        let request = ForecastRequest {
            series: vec![series()],
            horizon: 3,
            quantile_levels: vec![0.1, 0.35, 0.5, 0.9],
        };
        let output = model.forecast(&request).unwrap();
        assert_eq!(output.series[0].point, vec![vec![4.; 3]]);
        let scale = (20.0_f32 / 3.).sqrt();
        let expected = [
            4. - 0.4_f32.sinh() * scale,
            4. - (0.2_f32.sinh() + 0.1_f32.sinh()) * 0.5 * scale,
            4.,
            4. + 0.4_f32.sinh() * scale,
        ];
        for step in &output.series[0].quantiles[0] {
            close(step, &expected, 2e-6);
        }
        let weights = Weights::open(directory.path()).unwrap();
        assert!(
            tensor::<2>(
                &weights,
                "patch_proj.linear1.weight",
                [1, 1],
                &Device::flex()
            )
            .is_err()
        );
        assert!(tensor::<1>(&weights, "missing.weight", [1], &Device::flex()).is_err());
        assert!(
            model
                .forecast(&ForecastRequest {
                    quantile_levels: vec![0.01],
                    ..request
                })
                .is_err()
        );
    }

    #[test]
    fn malformed_checkpoint_dimensions_return_errors_without_overflow() {
        let directory = tempfile::tempdir().unwrap();
        checkpoint(directory.path());
        let base: serde_json::Value =
            serde_json::from_slice(&std::fs::read(directory.path().join("config.json")).unwrap())
                .unwrap();
        for field in [
            "d_model",
            "patch_size",
            "d_ff",
            "qk_dim",
            "v_dim",
            "num_heads",
            "num_groups",
        ] {
            let mut config = base.clone();
            config[field] = serde_json::json!(usize::MAX);
            std::fs::write(
                directory.path().join("config.json"),
                serde_json::to_vec(&config).unwrap(),
            )
            .unwrap();
            assert!(
                Toto2::load(directory.path(), &Device::flex()).is_err(),
                "{field}"
            );
        }
    }

    #[test]
    fn workload_guards_include_attention_covariates_and_padded_outputs() {
        let directory = tempfile::tempdir().unwrap();
        checkpoint(directory.path());
        let mut config: Config =
            serde_json::from_slice(&std::fs::read(directory.path().join("config.json")).unwrap())
                .unwrap();
        validate_workload(&config, 3, 4, 2).unwrap();
        assert!(validate_workload(&config, usize::MAX, 4, 2).is_err());
        config.num_heads = 2;
        assert!(
            validate_workload(&config, 1, (MAX_PATCHES - 1) * 2, 2)
                .unwrap_err()
                .to_string()
                .contains("time attention")
        );
        config.num_heads = 1;
        config.num_variate_layers_per_group = 2;
        assert!(
            validate_workload(&config, 8192, 2, 2)
                .unwrap_err()
                .to_string()
                .contains("variate attention")
        );
        config.d_ff = Some(crate::MAX_WORKSPACE_ELEMENTS / 4 + 1);
        assert!(
            validate_workload(&config, 1, 2, 2)
                .unwrap_err()
                .to_string()
                .contains("feedforward")
        );
        config.d_ff = Some(4);
        config.patch_size = 8_000_000;
        assert!(
            validate_workload(&config, 1, 1, 1)
                .unwrap_err()
                .to_string()
                .contains("native quantile output")
        );
        let time_only = Prepared::new(&series(), 2, 2, true, false).unwrap();
        assert!(time_only.var_mask.is_empty());
        let variate_only = Prepared::new(&series(), 2, 2, false, true).unwrap();
        assert!(variate_only.time_mask.is_empty());
    }

    #[test]
    fn checkpoints_without_variate_layers_reject_covariates() {
        let directory = tempfile::tempdir().unwrap();
        checkpoint(directory.path());
        let mut config: serde_json::Value =
            serde_json::from_slice(&std::fs::read(directory.path().join("config.json")).unwrap())
                .unwrap();
        config["num_variate_layers_per_group"] = 0.into();
        std::fs::write(
            directory.path().join("config.json"),
            serde_json::to_vec(&config).unwrap(),
        )
        .unwrap();
        let mut model = Toto2::load(directory.path(), &Device::flex()).unwrap();
        let caps = model.capabilities();
        assert!(!caps.joint_multivariate && !caps.past_covariates && !caps.known_covariates);
        let mut request = ForecastRequest {
            series: vec![series()],
            horizon: 2,
            quantile_levels: vec![],
        };
        model.forecast(&request).unwrap();
        request.series[0].past_covariates = vec![vec![0.; 4]];
        assert!(model.forecast(&request).is_err());
        request.series[0].past_covariates.clear();
        request.series[0].known_covariates = vec![vec![0.; 6]];
        assert!(model.forecast(&request).is_err());
        model.config.patch_size = 32;
        assert_eq!(
            model.capabilities().max_horizon,
            Some(crate::MAX_FORECAST_HORIZON)
        );
    }

    #[test]
    #[ignore = "requires a downloaded official checkpoint in TOTO2_CHECKPOINT"]
    fn official_checkpoint_load_and_multivariate_forecast() {
        let directory = std::env::var("TOTO2_CHECKPOINT").expect("set TOTO2_CHECKPOINT");
        let model = Toto2::load(Path::new(&directory), &Device::flex()).unwrap();
        let mut input = series();
        input.targets = vec![
            (0..64).map(|i| (i as f32 * 0.1).sin()).collect(),
            (0..64).map(|i| (i as f32 * 0.2).cos()).collect(),
        ];
        let request = ForecastRequest {
            series: vec![input],
            horizon: 35,
            quantile_levels: vec![0.1, 0.5, 0.9],
        };
        let output = model.forecast(&request).unwrap();
        output.validate(&request).unwrap();
        assert!(
            output.series[0]
                .point
                .iter()
                .flatten()
                .all(|v| v.is_finite())
        );
        assert_ne!(output.series[0].point[0], output.series[0].point[1]);
    }
}
