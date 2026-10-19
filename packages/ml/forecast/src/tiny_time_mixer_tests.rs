use super::*;
use crate::ForecastSeries;
use safetensors::{Dtype, tensor::TensorView};
use serde_json::json;

fn config() -> serde_json::Value {
    json!({
        "architectures": ["TinyTimeMixerForPrediction"], "model_type":"tinytimemixer",
        "context_length":8, "prediction_length":3, "patch_length":2, "patch_stride":2,
        "num_patches":4, "d_model":12, "num_layers":1, "expansion_factor":2,
        "adaptive_patching_levels":2, "gated_attn":true, "norm_eps":0.00001,
        "norm_mlp":"LayerNorm", "mode":"common_channel", "scaling":"std", "loss":"mse",
        "use_decoder":true, "decoder_d_model":8, "decoder_num_layers":1,
        "decoder_adaptive_patching_levels":0, "decoder_mode":"common_channel",
        "decoder_raw_residual":true, "num_input_channels":1
    })
}

fn checkpoint(
    config: serde_json::Value,
    signal: bool,
    malformed: Option<&str>,
) -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("config.json"), config.to_string()).unwrap();
    let parsed: Config = serde_json::from_value(config).unwrap();
    parsed.validate().unwrap();
    let mut state: Vec<(String, Vec<usize>, Vec<u8>)> = parsed
        .shapes()
        .into_iter()
        .enumerate()
        .map(|(index, (name, mut shape))| {
            if malformed == Some(name.as_str()) {
                shape[0] += 1;
            }
            let values: Vec<f32> = (0..shape.iter().product::<usize>())
                .map(|i| {
                    if name.ends_with("norm.norm.weight") {
                        0.9 + (i % 3) as f32 * 0.1
                    } else if !signal {
                        if name == "head.base_forecast_block.bias" {
                            i as f32 - 1.0
                        } else {
                            0.0
                        }
                    } else {
                        (((i * 17 + index * 11) % 43) as f32 - 21.0) * 0.013
                    }
                })
                .collect();
            (
                name,
                shape,
                values.into_iter().flat_map(f32::to_le_bytes).collect(),
            )
        })
        .collect();
    if malformed == Some("unexpected") {
        state.push((
            "unexpected.weight".into(),
            vec![1],
            1f32.to_le_bytes().to_vec(),
        ));
    }
    let views: Vec<_> = state
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

fn request(values: Vec<f32>) -> ForecastRequest {
    ForecastRequest {
        series: vec![ForecastSeries {
            id: "sales".into(),
            targets: vec![values],
            observed_mask: None,
            past_covariates: vec![],
            known_covariates: vec![],
        }],
        horizon: 3,
        quantile_levels: vec![],
    }
}

fn close(actual: &[f32], expected: &[f64], tolerance: f64) {
    assert_eq!(actual.len(), expected.len());
    for (i, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (*actual as f64 - expected).abs() < tolerance,
            "step {i}: {actual} != {expected}"
        );
    }
}

// Independent f64 scalar reference: no Burn operations or production layer helpers.
struct Reference {
    weights: BTreeMap<String, (Vec<usize>, Vec<f64>)>,
}
impl Reference {
    fn load(path: &Path) -> Self {
        let weights = Weights::open(path).unwrap();
        Self {
            weights: weights
                .names()
                .map(|n| {
                    (
                        n.into(),
                        (
                            weights.shape(n).unwrap(),
                            weights
                                .values(n)
                                .unwrap()
                                .into_iter()
                                .map(f64::from)
                                .collect(),
                        ),
                    )
                })
                .collect(),
        }
    }
    fn linear(&self, name: &str, values: &[f64], input: usize) -> Vec<f64> {
        let (shape, weights) = &self.weights[&format!("{name}.weight")];
        assert_eq!(shape[1], input);
        let biases = &self.weights[&format!("{name}.bias")].1;
        values
            .chunks_exact(input)
            .flat_map(|row| {
                (0..shape[0])
                    .map(|out| {
                        biases[out]
                            + row
                                .iter()
                                .enumerate()
                                .map(|(i, x)| x * weights[out * input + i])
                                .sum::<f64>()
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }
    fn mixer(
        &self,
        name: &str,
        values: Vec<f64>,
        patches: usize,
        features: usize,
        patch: bool,
        epsilon: f64,
        gated: bool,
    ) -> Vec<f64> {
        let weights = &self.weights[&format!("{name}.norm.norm.weight")].1;
        let biases = &self.weights[&format!("{name}.norm.norm.bias")].1;
        let mut normalized = Vec::with_capacity(values.len());
        for row in values.chunks_exact(features) {
            let mean = row.iter().sum::<f64>() / features as f64;
            let deviation = (row.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / features as f64
                + epsilon)
                .sqrt();
            normalized.extend(
                row.iter()
                    .enumerate()
                    .map(|(i, x)| (x - mean) / deviation * weights[i] + biases[i]),
            );
        }
        if patch {
            normalized = transpose(&normalized, patches, features);
        }
        let size = if patch { patches } else { features };
        let first = self.linear(&format!("{name}.mlp.fc1"), &normalized, size);
        let hidden_size = self.weights[&format!("{name}.mlp.fc1.bias")].1.len();
        let first: Vec<_> = first.into_iter().map(gelu).collect();
        let mut mixed = self.linear(&format!("{name}.mlp.fc2"), &first, hidden_size);
        if gated {
            let gates = self.linear(&format!("{name}.gating_block.attn_layer"), &mixed, size);
            for (row, gates) in mixed.chunks_exact_mut(size).zip(gates.chunks_exact(size)) {
                let max = gates.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                let denominator = gates.iter().map(|g| (g - max).exp()).sum::<f64>();
                for (value, gate) in row.iter_mut().zip(gates) {
                    *value *= (gate - max).exp() / denominator;
                }
            }
        }
        if patch {
            mixed = transpose(&mixed, features, patches);
        }
        values
            .into_iter()
            .zip(mixed)
            .map(|(residual, value)| residual + value)
            .collect()
    }
    fn block(
        &self,
        name: &str,
        mut values: Vec<f64>,
        patches: usize,
        width: usize,
        levels: usize,
        layers: usize,
        epsilon: f64,
        gated: bool,
    ) -> Vec<f64> {
        for group in 0..levels.max(1) {
            let level = if levels == 0 { 0 } else { levels - group - 1 };
            let mut factor = 2usize.pow(level as u32);
            if width / factor <= 4 {
                factor = 1;
            }
            let p = patches * factor;
            let d = width / factor;
            for layer in 0..layers {
                let prefix = if levels == 0 {
                    format!("{name}.mixers.{layer}")
                } else {
                    format!("{name}.mixers.{group}.mixer_layers.{layer}")
                };
                if p > 1 {
                    values = self.mixer(
                        &format!("{prefix}.patch_mixer"),
                        values,
                        p,
                        d,
                        true,
                        epsilon,
                        gated,
                    );
                }
                values = self.mixer(
                    &format!("{prefix}.feature_mixer"),
                    values,
                    p,
                    d,
                    false,
                    epsilon,
                    gated,
                );
            }
        }
        values
    }
    fn forecast(&self, path: &Path, input: &[f32], mask: Option<&[bool]>) -> Vec<f64> {
        let config: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path.join("config.json")).unwrap()).unwrap();
        let integer = |name: &str| config[name].as_u64().unwrap() as usize;
        let context = integer("context_length");
        let mut values = vec![0.0; context];
        let mut count = 0.0;
        let mut observed = vec![false; context];
        for i in 0..input.len().min(context) {
            let source = input.len() - 1 - i;
            if mask.is_none_or(|m| m[source]) {
                values[context - 1 - i] = input[source] as f64;
                observed[context - 1 - i] = true;
                count += 1.0;
            }
        }
        let mean = values.iter().sum::<f64>() / f64::max(count, 1.0);
        let variance = values
            .iter()
            .zip(&observed)
            .filter(|(_, observed)| **observed)
            .map(|(x, _)| (x - mean).powi(2))
            .sum::<f64>()
            / f64::max(count, 1.0);
        let scale = (variance + config["minimum_scale"].as_f64().unwrap_or(1e-5)).sqrt();
        let values: Vec<_> = values.into_iter().map(|x| (x - mean) / scale).collect();
        let length = integer("patch_length");
        let stride = integer("patch_stride");
        let patches = integer("num_patches");
        let start = context - (length + stride * (patches - 1));
        let patch_input: Vec<_> = (0..patches)
            .flat_map(|p| {
                values[start + p * stride..start + p * stride + length]
                    .iter()
                    .copied()
            })
            .collect();
        let width = integer("d_model");
        let epsilon = config["norm_eps"].as_f64().unwrap();
        let gated = config["gated_attn"].as_bool().unwrap();
        let mut hidden = self.linear("backbone.encoder.patcher", &patch_input, length);
        hidden = self.block(
            "backbone.encoder.mlp_mixer_encoder",
            hidden,
            patches,
            width,
            integer("adaptive_patching_levels"),
            integer("num_layers"),
            epsilon,
            gated,
        );
        let head_dim = if config["use_decoder"] == true {
            let decoder_width = integer("decoder_d_model");
            if width != decoder_width {
                hidden = self.linear("decoder.adapter", &hidden, width);
            }
            if config["decoder_raw_residual"] == true {
                let embedded = self.linear("decoder.decoder_raw_embedding", &patch_input, length);
                hidden
                    .iter_mut()
                    .zip(embedded)
                    .for_each(|(x, residual)| *x += residual);
            }
            hidden = self.block(
                "decoder.decoder_block",
                hidden,
                patches,
                decoder_width,
                integer("decoder_adaptive_patching_levels"),
                integer("decoder_num_layers"),
                epsilon,
                gated,
            );
            decoder_width
        } else {
            width
        };
        self.linear("head.base_forecast_block", &hidden, patches * head_dim)
            .into_iter()
            .map(|x| x * scale + mean)
            .collect()
    }
}

fn transpose(values: &[f64], rows: usize, columns: usize) -> Vec<f64> {
    (0..columns)
        .flat_map(|c| (0..rows).map(move |r| values[r * columns + c]))
        .collect()
}

fn gelu(x: f64) -> f64 {
    // Abramowitz and Stegun 7.1.26; erf error is below 1.5e-7.
    let z = x / std::f64::consts::SQRT_2;
    let t = 1.0 / (1.0 + 0.3275911 * z.abs());
    let erf = z.signum()
        * (1.0
            - (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t - 0.284496736) * t
                + 0.254829592)
                * t
                * (-z * z).exp());
    0.5 * x * (1.0 + erf)
}

#[test]
fn adaptive_mixing_decoder_and_mask_match_scalar_reference() {
    let dir = checkpoint(config(), true, None);
    let model = TinyTimeMixer::load(dir.path(), &Device::flex()).unwrap();
    let mut request = request(vec![12., 14., 99_999., 13., 20., 18.]);
    request.series[0].observed_mask = Some(vec![vec![true, true, false, true, true, true]]);
    let actual = model.forecast(&request).unwrap();
    let expected = Reference::load(dir.path()).forecast(
        dir.path(),
        &request.series[0].targets[0],
        request.series[0]
            .observed_mask
            .as_ref()
            .map(|m| m[0].as_slice()),
    );
    close(&actual.series[0].point[0], &expected, 2e-5);
    assert!(actual.quantile_levels.is_empty());
    assert!(actual.series[0].quantiles[0].iter().all(Vec::is_empty));
    assert_eq!(actual.point_statistic, PointStatistic::Mean);
    actual.validate(&request).unwrap();
}

#[test]
fn masked_padding_is_excluded_from_scaling_and_horizon_is_cropped() {
    let dir = checkpoint(config(), false, None);
    let model = TinyTimeMixer::load(dir.path(), &Device::flex()).unwrap();
    let mut request = request(vec![2., 1e9, 4.]);
    request.series[0].observed_mask = Some(vec![vec![true, false, true]]);
    request.horizon = 2;
    let actual = model.forecast(&request).unwrap();
    close(
        &actual.series[0].point[0],
        &[3.0 - (1.0_f64 + 1e-5).sqrt(), 3.0],
        1e-6,
    );
    request.series[0].targets[0][1] = -1e9;
    assert_eq!(
        actual.series[0].point,
        model.forecast(&request).unwrap().series[0].point
    );
}

#[test]
fn trims_left_history_and_runs_independent_series() {
    let dir = checkpoint(config(), true, None);
    let model = TinyTimeMixer::load(dir.path(), &Device::flex()).unwrap();
    let mut request = request(vec![1000., 1000., 1., 2., 3., 4., 5., 6., 7., 8.]);
    let mut other = request.series[0].clone();
    other.id = "other".into();
    other.targets[0] = vec![8., 7., 6., 5., 4., 3., 2., 1.];
    request.series.push(other);
    let actual = model.forecast(&request).unwrap();
    let reference = Reference::load(dir.path());
    for (forecast, series) in actual.series.iter().zip(&request.series) {
        close(
            &forecast.point[0],
            &reference.forecast(dir.path(), &series.targets[0], None),
            2e-5,
        );
    }
}

#[test]
fn no_decoder_no_gates_and_single_patch_are_supported() {
    let mut c = config();
    c["use_decoder"] = json!(false);
    c["gated_attn"] = json!(false);
    c["context_length"] = json!(2);
    c["num_patches"] = json!(1);
    c["adaptive_patching_levels"] = json!(0);
    let dir = checkpoint(c, true, None);
    let model = TinyTimeMixer::load(dir.path(), &Device::flex()).unwrap();
    let request = request(vec![8., 3.]);
    close(
        &model.forecast(&request).unwrap().series[0].point[0],
        &Reference::load(dir.path()).forecast(dir.path(), &request.series[0].targets[0], None),
        2e-5,
    );
}

#[test]
fn unsupported_variants_and_overflow_are_rejected_before_weight_loading() {
    for (key, value) in [
        ("loss", json!("nll")),
        ("mode", json!("mix_channel")),
        ("scaling", json!("mean")),
        ("multi_quantile_head", json!(true)),
        ("self_attn", json!(true)),
        ("enable_forecast_channel_mixing", json!(true)),
        ("resolution_prefix_tuning", json!(true)),
        ("multi_scale", json!(true)),
        ("gate_mode", json!("sigmoid")),
        ("num_patches", json!(5)),
        ("expansion_factor", json!(usize::MAX)),
        ("d_model", json!(usize::MAX)),
        ("adaptive_patching_levels", json!(usize::MAX)),
        ("norm_eps", json!(-0.1)),
        ("minimum_scale", json!(0.0)),
    ] {
        let mut c = config();
        c[key] = value;
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("config.json"), c.to_string()).unwrap();
        let error = TinyTimeMixer::load(dir.path(), &Device::flex())
            .err()
            .unwrap();
        assert!(matches!(error, Error::Invalid(_)), "{key}: {error}");
    }
}

#[test]
fn rejects_malformed_and_unexpected_weights() {
    for name in [
        "backbone.encoder.patcher.weight",
        "head.base_forecast_block.bias",
        "unexpected",
    ] {
        let dir = checkpoint(config(), true, Some(name));
        let error = TinyTimeMixer::load(dir.path(), &Device::flex())
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains(name), "{error}");
    }
}

#[test]
fn rejects_quantiles_covariates_joint_targets_and_excess_horizon() {
    let dir = checkpoint(config(), false, None);
    let model = TinyTimeMixer::load(dir.path(), &Device::flex()).unwrap();
    for case in 0..5 {
        let mut request = request(vec![1., 2., 3., 4.]);
        match case {
            0 => request.quantile_levels = vec![0.5],
            1 => request.series[0].past_covariates = vec![vec![0.; 4]],
            2 => request.series[0].known_covariates = vec![vec![0.; 7]],
            3 => request.series[0].targets.push(vec![1.; 4]),
            _ => request.horizon = 4,
        }
        assert!(model.forecast(&request).is_err());
    }
    assert_eq!(model.capabilities().max_horizon, Some(3));
    assert!(!model.capabilities().joint_multivariate);
}

#[test]
#[ignore = "Set FLOWLIKE_TTM_CHECKPOINT to the pinned IBM r2 checkpoint directory"]
fn official_checkpoint_matches_independent_scalar_inference() {
    let directory = std::env::var("FLOWLIKE_TTM_CHECKPOINT").expect("FLOWLIKE_TTM_CHECKPOINT");
    let path = Path::new(&directory);
    let model = TinyTimeMixer::load(path, &Device::flex()).unwrap();
    let values: Vec<_> = (0..512)
        .map(|i| 12.0 + (i as f32 * 0.17).sin() * 3.0 + i as f32 * 0.005)
        .collect();
    let mut request = request(values);
    request.horizon = 96;
    let actual = model.forecast(&request).unwrap();
    let expected = Reference::load(path).forecast(path, &request.series[0].targets[0], None);
    close(&actual.series[0].point[0], &expected, 2e-4);
    assert_eq!(actual.series[0].point[0].len(), 96);
    assert!(
        actual.series[0].point[0]
            .windows(2)
            .any(|p| (p[0] - p[1]).abs() > 1e-3)
    );
    actual.validate(&request).unwrap();
}
