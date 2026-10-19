use super::*;
use crate::ForecastSeries;
use safetensors::{
    Dtype,
    tensor::{TensorView, serialize_to_file},
};
use std::collections::BTreeMap;

fn config() -> Config {
    serde_json::from_value(serde_json::json!({
        "patch_size": 2, "d_model": 8, "num_heads": 2, "mlp_hidden_dim": 12,
        "block_schedule": ["time", "time_variate"], "quantile_knots": [0.1, 0.5, 0.9],
        "block_size": 2, "context_length": 8, "attn_bias": true, "mlp_bias": true
    }))
    .unwrap()
}

type Fixture = BTreeMap<String, (Vec<usize>, Vec<f32>)>;

fn fixture(nonzero: bool) -> Fixture {
    let c = config();
    let mut result = BTreeMap::new();
    let mut add = |name: &str, shape: Vec<usize>, norm: bool| {
        let count = shape.iter().product();
        let offset = name.bytes().fold(0_usize, |a, b| a + b as usize);
        let values = (0..count)
            .map(|i| {
                if norm {
                    1.0
                } else if nonzero {
                    (((i * 17 + offset) % 29) as f32 - 14.0) * 0.015
                } else {
                    0.0
                }
            })
            .collect();
        result.insert(name.to_owned(), (shape, values));
    };
    add("representation.forecast_query", vec![c.d_model], false);
    add("transformer.out_norm.scale", vec![c.d_model], true);
    let mut linear = |name: &str, input: usize, output: usize, bias: bool| {
        add(&format!("{name}.weight"), vec![output, input], false);
        if bias {
            add(&format!("{name}.bias"), vec![output], false);
        }
    };
    for (name, input, output, bias) in [
        (
            "representation.patch_proj",
            2 * c.patch_size,
            c.d_model,
            true,
        ),
        (
            "head.projection",
            c.d_model,
            c.patch_size * c.quantile_knots.len(),
            false,
        ),
    ] {
        linear(&format!("{name}.hidden_layer"), input, 4 * c.d_model, bias);
        linear(&format!("{name}.output_layer"), 4 * c.d_model, output, bias);
        linear(&format!("{name}.residual_layer"), input, output, bias);
    }
    for (i, kind) in c.block_schedule.iter().enumerate() {
        let prefix = format!("transformer.layers.{i}");
        for j in 0..axes(kind).unwrap().len() {
            for (suffix, output) in [
                ("wQKV", 3 * c.d_model),
                ("wO", c.d_model),
                ("_gate", c.d_model),
            ] {
                linear(
                    &format!("{prefix}.attns.{j}.{suffix}"),
                    c.d_model,
                    output,
                    true,
                );
            }
        }
        linear(
            &format!("{prefix}.mlp.w12"),
            c.d_model,
            2 * c.mlp_hidden_dim,
            true,
        );
        linear(
            &format!("{prefix}.mlp.w3"),
            c.mlp_hidden_dim,
            c.d_model,
            true,
        );
    }
    for (i, kind) in c.block_schedule.iter().enumerate() {
        add(
            &format!("transformer.layers.{i}.mlp_norm.scale"),
            vec![c.d_model],
            true,
        );
        for j in 0..axes(kind).unwrap().len() {
            add(
                &format!("transformer.layers.{i}.attn_norms.{j}.scale"),
                vec![c.d_model],
                true,
            );
        }
    }
    result
}

fn save_fixture(data: &Fixture) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("config.json"),
        serde_json::to_vec(&serde_json::json!({
            "patch_size": 2, "d_model": 8, "num_heads": 2, "mlp_hidden_dim": 12,
            "block_schedule": ["time", "time_variate"], "quantile_knots": [0.1, 0.5, 0.9],
            "block_size": 2, "context_length": 8, "attn_bias": true, "mlp_bias": true
        }))
        .unwrap(),
    )
    .unwrap();
    let bytes: BTreeMap<_, Vec<u8>> = data
        .iter()
        .map(|(name, (_, values))| (name, values.iter().flat_map(|v| v.to_le_bytes()).collect()))
        .collect();
    let views = data.iter().map(|(name, (shape, _))| {
        (
            name.as_str(),
            TensorView::new(Dtype::F32, shape.clone(), &bytes[name]).unwrap(),
        )
    });
    serialize_to_file(views, None, &dir.path().join("model.safetensors")).unwrap();
    dir
}

fn series(values: Vec<f32>) -> ForecastSeries {
    ForecastSeries {
        id: "signal".into(),
        targets: vec![values],
        observed_mask: None,
        past_covariates: vec![],
        known_covariates: vec![],
    }
}

fn near(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual {actual}, expected {expected}, tolerance {tolerance}"
    );
}

#[test]
fn causal_statistics_ignore_masked_values_and_use_sample_variance() {
    let (loc, scale) = causal_stats(
        &[0.0, 1.0, 3000.0, 3.0, 5.0, 7.0],
        &[false, true, false, true, true, true],
        2,
        1e-6,
    );
    assert_eq!(loc, [1.0, 2.0, 4.0]);
    near(scale[0], 1e-6, 1e-10);
    near(scale[1], 2_f32.sqrt(), 1e-6);
    near(scale[2], (20_f32 / 3.0).sqrt(), 1e-6);
    assert_eq!(
        instance_stats(&[0.0, 1.0, 1.0, 0.0], &[true; 4], 1e-6),
        (0.0, 1.0, true)
    );
    let (loc, scale, binary) = instance_stats(&[-1.0, 1.0, 3.0], &[true; 3], 1e-6);
    assert!(!binary);
    near(loc, 1.0, 1e-6);
    near(scale, (8_f32 / 3.0).sqrt(), 1e-6);
}

#[test]
fn monotone_quantiles_respect_uneven_probability_gaps() {
    let levels = [0.1, 0.3, 0.5, 0.9];
    let mut values = [0.0, 0.0, 2.0, 0.0];
    monotone(&mut values, &levels, 2.0);
    for (actual, expected) in values.into_iter().zip([
        2.0 - 0.8 * 2_f32.ln(),
        2.0 - 0.4 * 2_f32.ln(),
        2.0,
        2.0 + 0.8 * 2_f32.ln(),
    ]) {
        near(actual, expected, 1e-6);
    }
    let mut overflow = [f32::NEG_INFINITY, 1.0, 2.0, f32::INFINITY];
    clamp_nonfinite(&mut overflow);
    assert_eq!(overflow, [1.0, 1.0, 2.0, 2.0]);
}

#[test]
fn zero_checkpoint_matches_analytical_forecast_across_refinement_and_blocks() {
    let dir = save_fixture(&fixture(false));
    let model = Timestral::load(dir.path(), &Device::flex()).unwrap();
    let request = ForecastRequest {
        series: vec![series(vec![1.0, 3.0, 5.0, 7.0])],
        horizon: 5,
        quantile_levels: vec![],
    };
    let output = model.forecast(&request).unwrap();
    output.validate(&request).unwrap();
    let width = (0.8 * 2_f32.ln()).sinh();
    for (t, scale) in [
        (20_f32 / 3.0).sqrt(),
        (20_f32 / 3.0).sqrt(),
        2.0,
        2.0,
        (20_f32 / 7.0).sqrt(),
    ]
    .into_iter()
    .enumerate()
    {
        near(output.series[0].point[0][t], 4.0, 1e-6);
        for (actual, expected) in output.series[0].quantiles[0][t].iter().zip([
            4.0 - width * scale,
            4.0,
            4.0 + width * scale,
        ]) {
            near(*actual, expected, 3e-6);
        }
    }
}

#[test]
fn observation_masks_are_value_invariant_and_batch_entries_are_independent() {
    let dir = save_fixture(&fixture(true));
    let model = Timestral::load(dir.path(), &Device::flex()).unwrap();
    let mut first = series(vec![-2.0, 3.0, 10000.0, 1.0, -1.0]);
    first.observed_mask = Some(vec![vec![true, true, false, true, true]]);
    let req = ForecastRequest {
        series: vec![first.clone()],
        horizon: 7,
        quantile_levels: vec![0.5, 0.9],
    };
    let a = model.forecast(&req).unwrap();
    first.targets[0][2] = -99999.0;
    let mut other = series(vec![1.0; 13]);
    other.id = "other".into();
    let b = model
        .forecast(&ForecastRequest {
            series: vec![first, other],
            ..req
        })
        .unwrap();
    assert_eq!(a.series[0].point, b.series[0].point);
    assert_eq!(a.series[0].quantiles, b.series[0].quantiles);
    assert!(
        b.series
            .iter()
            .flat_map(|s| &s.quantiles)
            .flatten()
            .all(|q| q.windows(2).all(|w| w[0] <= w[1]))
    );
}

#[test]
fn covariate_firewall_and_padding_masks_match_the_dhf_contract() {
    let c = config();
    let masks = build_masks(
        &[
            vec![false, true, false, false],
            vec![true, true, false, false],
            vec![true; 4],
        ],
        &[0, 1, 2],
        2,
        2,
        true,
        &c,
        &Device::flex(),
    );
    let time = masks.time.into_data().try_to_vec::<bool>().unwrap();
    let allowed = |r: usize, query: usize, key: usize| !time[(r * 8 + query) * 8 + key];
    assert!(!allowed(0, 1, 0)); // Leading padding is never a key for another token.
    assert!(allowed(0, 2, 2));
    assert!(!allowed(0, 1, 2));
    assert!(allowed(2, 0, 3)); // Known covariates have bidirectional encoder attention.
    assert!(allowed(0, 6, 1));
    assert!(!allowed(0, 6, 2)); // Forecast decoder cannot read target horizon encoder.
    assert!(allowed(0, 7, 6));
    assert!(!allowed(0, 6, 7));
    let variate = masks.variate.into_data().try_to_vec::<bool>().unwrap();
    let allowed = |slot: usize, query: usize, key: usize| !variate[(slot * 3 + query) * 3 + key];
    assert!(allowed(1, 0, 2));
    assert!(!allowed(1, 1, 0));
    assert!(!allowed(1, 2, 1));
    assert!(allowed(1, 2, 2));
}

// Independent scalar attention checks the checkpoint's unusual QKV head interleaving,
// RoPE half rotation, masks, sigmoid gate, and projection without a second tensor graph.
fn scalar_linear(x: &[f32], data: &Fixture, name: &str) -> Vec<f32> {
    let (shape, weight) = &data[&format!("{name}.weight")];
    (0..shape[0])
        .map(|o| {
            let bias = data.get(&format!("{name}.bias")).map_or(0.0, |(_, b)| b[o]);
            bias + (0..shape[1])
                .map(|i| x[i] * weight[o * shape[1] + i])
                .sum::<f32>()
        })
        .collect()
}

#[test]
fn attention_matches_scalar_reference_for_time_and_variate_axes() {
    let data = fixture(true);
    let dir = save_fixture(&data);
    let device = Device::flex();
    let w = crate::weights::Weights::open(dir.path()).unwrap();
    let c = config();
    let (rows, seq, width, heads, hd) = (2, 3, 8, 2, 4);
    let input: Vec<f32> = (0..rows * seq * width)
        .map(|i| (i as f32 * 0.3).sin())
        .collect();
    let masks = build_masks(
        &[vec![true, true], vec![false, true]],
        &[0, 0],
        2,
        1,
        false,
        &c,
        &device,
    );
    for (time, prefix) in [
        (true, "transformer.layers.0.attns.0"),
        (false, "transformer.layers.1.attns.1"),
    ] {
        let attn = Attention::load(&w, prefix, time, &c, &device).unwrap();
        let mask = if time { &masks.time } else { &masks.variate };
        let actual = attn
            .forward(
                Tensor::from_data(TensorData::new(input.clone(), [rows, seq, width]), &device),
                mask,
                &masks.cos,
                &masks.sin,
                time.then_some(2),
            )
            .into_data()
            .try_to_vec::<f32>()
            .unwrap();
        let mask = mask.to_data().try_to_vec::<bool>().unwrap();
        let (batch, length) = if time { (rows, seq) } else { (seq, rows) };
        let at = |b: usize, t: usize| {
            let i = if time { b * seq + t } else { t * seq + b };
            &input[i * width..(i + 1) * width]
        };
        let mut expected = vec![0.0; input.len()];
        for b in 0..batch {
            let mut qkv: Vec<Vec<f32>> = (0..length)
                .map(|t| scalar_linear(at(b, t), &data, &format!("{prefix}.wQKV")))
                .collect();
            if time {
                for (t, row) in qkv.iter_mut().enumerate() {
                    for kind in 0..2 {
                        for h in 0..heads {
                            let base = kind * width;
                            // rope_dim=2 rotates dimension 0 against dimension 2.
                            let a = row[base + h];
                            let z = row[base + 2 * heads + h];
                            row[base + h] = a * (t as f32).cos() - z * (t as f32).sin();
                            row[base + 2 * heads + h] = z * (t as f32).cos() + a * (t as f32).sin();
                        }
                    }
                }
            }
            for t in 0..length {
                let mut out = vec![0.0; width];
                for h in 0..heads {
                    let logits: Vec<f32> = (0..length)
                        .map(|k| {
                            if mask[(b * length + t) * length + k] {
                                f32::NEG_INFINITY
                            } else {
                                (0..hd)
                                    .map(|d| qkv[t][d * heads + h] * qkv[k][width + d * heads + h])
                                    .sum::<f32>()
                                    / (hd as f32).sqrt()
                            }
                        })
                        .collect();
                    let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                    let sum: f32 = logits.iter().map(|v| (v - max).exp()).sum();
                    for d in 0..hd {
                        out[h * hd + d] = (0..length)
                            .map(|k| {
                                (logits[k] - max).exp() / sum * qkv[k][2 * width + d * heads + h]
                            })
                            .sum();
                    }
                }
                let gate = scalar_linear(at(b, t), &data, &format!("{prefix}._gate"));
                for (x, g) in out.iter_mut().zip(gate) {
                    *x *= 1.0 / (1.0 + (-g).exp());
                }
                let out = scalar_linear(&out, &data, &format!("{prefix}.wO"));
                let i = if time { b * seq + t } else { t * seq + b };
                expected[i * width..(i + 1) * width].copy_from_slice(&out);
            }
        }
        for (a, e) in actual.into_iter().zip(expected) {
            near(a, e, 2e-6);
        }
    }
}

#[test]
fn known_future_covariates_change_target_forecasts_without_future_target_inputs() {
    let dir = save_fixture(&fixture(true));
    let model = Timestral::load(dir.path(), &Device::flex()).unwrap();
    let mut s = series(vec![-2.0, 1.0, 3.0, -1.0]);
    s.past_covariates = vec![vec![1.0, -1.0, 2.0, 0.0]];
    s.known_covariates = vec![vec![0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0]];
    let request = ForecastRequest {
        series: vec![s],
        horizon: 3,
        quantile_levels: vec![],
    };
    let a = model.forecast(&request).unwrap();
    let mut changed = request;
    changed.series[0].known_covariates[0][4..].fill(5.0);
    let b = model.forecast(&changed).unwrap();
    assert!(
        a.series[0].point[0]
            .iter()
            .zip(&b.series[0].point[0])
            .any(|(a, b)| (a - b).abs() > 1e-6)
    );
}

#[test]
fn rejects_incomplete_or_incompatible_checkpoint_and_non_native_quantiles() {
    let mut data = fixture(false);
    data.remove("representation.forecast_query");
    let dir = save_fixture(&data);
    assert!(Timestral::load(dir.path(), &Device::flex()).is_err());
    let mut data = fixture(false);
    data.insert(
        "representation.forecast_query".into(),
        (vec![3], vec![0.0; 3]),
    );
    let dir = save_fixture(&data);
    assert!(Timestral::load(dir.path(), &Device::flex()).is_err());
    let dir = save_fixture(&fixture(false));
    let model = Timestral::load(dir.path(), &Device::flex()).unwrap();
    let request = ForecastRequest {
        series: vec![series(vec![1.0, 2.0])],
        horizon: 2,
        quantile_levels: vec![0.2],
    };
    assert!(model.forecast(&request).is_err());
}

#[test]
fn positive_context_clips_lower_quantiles_at_zero() {
    let dir = save_fixture(&fixture(false));
    let model = Timestral::load(dir.path(), &Device::flex()).unwrap();
    let request = ForecastRequest {
        series: vec![series(vec![0.0, 0.0, 0.0, 20.0])],
        horizon: 2,
        quantile_levels: vec![],
    };
    let result = model.forecast(&request).unwrap();
    assert_eq!(result.series[0].quantiles[0][0][0], 0.0);
    assert!(result.series[0].point[0][0] > 0.0);
}

#[test]
fn malformed_config_dimensions_return_errors_before_loading_weights() {
    for (field, value) in [
        ("patch_size", serde_json::json!(usize::MAX)),
        ("d_model", serde_json::json!(usize::MAX - 7)),
        ("mlp_hidden_dim", serde_json::json!(usize::MAX)),
        ("block_size", serde_json::json!(usize::MAX)),
        ("context_length", serde_json::json!(usize::MAX)),
        ("minimum_scale", serde_json::json!(1e-200)),
        ("norm_eps", serde_json::json!(1e100)),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = serde_json::json!({
            "patch_size": 2, "d_model": 8, "num_heads": 2, "mlp_hidden_dim": 12,
            "block_schedule": ["time", "time_variate"], "quantile_knots": [0.1, 0.5, 0.9],
            "block_size": 2, "context_length": 8
        });
        cfg[field] = value;
        std::fs::write(
            dir.path().join("config.json"),
            serde_json::to_vec(&cfg).unwrap(),
        )
        .unwrap();
        let outcome = std::panic::catch_unwind(|| Timestral::load(dir.path(), &Device::flex()));
        assert!(
            matches!(outcome, Ok(Err(Error::Invalid(_)))),
            "malformed field {field} must fail config validation"
        );
    }
}

#[test]
fn workspace_limits_cover_split_attention_and_covariate_outputs() {
    let c = config();
    c.validate_workspace(2, 4096, 1, false).unwrap();
    assert!(c.validate_workspace(2, 4097, 1, false).is_err());
    assert!(c.validate_workspace(4096, 4, 2, true).is_err());
    let mut input = series(vec![1.0, 2.0]);
    input.past_covariates = vec![vec![1.0, 2.0]; 200];
    assert!(Prepared::new(&input, crate::MAX_FORECAST_HORIZON, &c).is_err());
}

#[test]
#[ignore = "requires an official checkpoint; set FLOWLIKE_TIMESTRAL_CHECKPOINT"]
fn official_checkpoint_forecasts_masked_history_and_covariates() {
    let directory = std::env::var("FLOWLIKE_TIMESTRAL_CHECKPOINT").unwrap();
    let model = Timestral::load(Path::new(&directory), &Device::flex()).unwrap();
    let history = 96;
    let horizon = 40;
    let request = ForecastRequest {
        series: vec![crate::ForecastSeries {
            id: "seasonal".into(),
            targets: vec![
                (0..history)
                    .map(|i| 10.0 + (i as f32 * 0.15).sin())
                    .collect(),
            ],
            observed_mask: Some(vec![(0..history).map(|i| i % 7 != 0).collect()]),
            past_covariates: vec![(0..history).map(|i| (i as f32 * 0.15).cos()).collect()],
            known_covariates: vec![
                (0..history + horizon)
                    .map(|i| i as f32 / history as f32)
                    .collect(),
            ],
        }],
        horizon,
        quantile_levels: vec![0.1, 0.5, 0.9],
    };
    let result = model.forecast(&request).unwrap();
    result.validate(&request).unwrap();
    assert!(
        result.series[0].quantiles[0]
            .iter()
            .all(|q| q.windows(2).all(|pair| pair[0] <= pair[1]))
    );
}
