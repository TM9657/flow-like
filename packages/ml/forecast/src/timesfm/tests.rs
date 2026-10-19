use super::*;
use crate::ForecastSeries;
use safetensors::{
    Dtype,
    tensor::{TensorView, serialize_to_file},
};
use std::collections::BTreeMap;

fn config_json() -> serde_json::Value {
    serde_json::json!({
        "architectures": ["TimesFmModelForPrediction"], "model_type": "timesfm",
        "context_length": 32, "head_dim": 4, "hidden_size": 8,
        "horizon_length": 4, "intermediate_size": 12,
        "num_attention_heads": 2, "num_hidden_layers": 2, "patch_length": 2,
        "quantile_horizon_length": 8, "quantiles": QUANTILES,
        "rms_norm_eps": 0.000001, "torch_compile": false
    })
}

fn config() -> Config {
    serde_json::from_value(config_json()).unwrap()
}

type Fixture = BTreeMap<String, (Vec<usize>, Vec<f32>)>;

fn fixture(nonzero: bool) -> Fixture {
    config()
        .specs()
        .into_iter()
        .map(|(name, shape)| {
            let seed = name.bytes().fold(0_usize, |sum, byte| sum + byte as usize);
            let values = (0..shape.iter().product())
                .map(|i| {
                    if name.ends_with(".scale") {
                        0.9 + (i % 5) as f32 * 0.04
                    } else if nonzero {
                        (((i * 7 + seed) % 23) as f32 - 11.0) * 0.035
                    } else {
                        0.0
                    }
                })
                .collect();
            (name, (shape, values))
        })
        .collect()
}

fn write_fixture(fixture: &Fixture) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("config.json"),
        serde_json::to_vec(&config_json()).unwrap(),
    )
    .unwrap();
    let bytes: BTreeMap<_, Vec<u8>> = fixture
        .iter()
        .map(|(name, (_, values))| (name, values.iter().flat_map(|v| v.to_le_bytes()).collect()))
        .collect();
    let views = fixture.iter().map(|(name, (shape, _))| {
        (
            name.as_str(),
            TensorView::new(Dtype::F32, shape.clone(), &bytes[name]).unwrap(),
        )
    });
    serialize_to_file(views, None, &dir.path().join("model.safetensors")).unwrap();
    dir
}

fn request(values: Vec<f32>, horizon: usize) -> ForecastRequest {
    ForecastRequest {
        series: vec![ForecastSeries {
            id: "signal".into(),
            targets: vec![values],
            observed_mask: None,
            past_covariates: vec![],
            known_covariates: vec![],
        }],
        horizon,
        quantile_levels: vec![],
    }
}

fn near(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual {actual}, expected {expected}, tolerance {tolerance}"
    );
}

#[test]
fn preprocessing_interpolates_missing_values_before_truncating_and_padding() {
    let c = config();
    let prepared = prepare(
        &[99.0, 1.0, 99.0, 3.0, 99.0],
        Some(&[false, true, false, true, false]),
        &c,
    )
    .unwrap();
    assert_eq!(prepared.missing, [false; 4]);
    near(prepared.mean as f32, 9.0 / 24.0, 1e-6);
    near(
        prepared.sigma as f32,
        ((23.0_f32 - 81.0 / 24.0) / 23.0).sqrt(),
        1e-6,
    );
    for (value, expected) in prepared.values.iter().zip([1.0, 2.0, 3.0, 3.0]) {
        near(
            (*value as f64 * prepared.sigma + prepared.mean) as f32,
            expected,
            1e-6,
        );
    }
    let long: Vec<f32> = (0..30).map(|i| i as f32).collect();
    let p = prepare(&long, None, &c).unwrap();
    assert_eq!(p.values.len(), 24);
    near((p.values[0] as f64 * p.sigma + p.mean) as f32, 6.0, 1e-6);
    let padded = prepare(&[1.0, 2.0, 3.0], None, &c).unwrap();
    assert_eq!(padded.missing, [true, false, false, false]);
    assert!(prepare(&[1.0], Some(&[false]), &c).is_err());
}

#[test]
fn virtual_padding_preserves_outer_normalization_tolerance() {
    let prepared = prepare(&[2e-6, 4e-6], None, &config()).unwrap();
    assert!(prepared.sigma < 1e-6);
    near(prepared.mean as f32, 0.25e-6, 1e-12);
    // The full configured context falls below the tolerance, so revin uses
    // a divisor of one even though the unpadded sample deviation exceeds it.
    near(prepared.values[0], 1.75e-6, 1e-12);
    near(prepared.values[1], 3.75e-6, 1e-12);
    assert_eq!(prepared.values.len(), 2);
}

#[test]
fn running_statistics_use_population_variance_and_ignore_padding() {
    let mut s = Stats::default();
    s.update(&[1000.0, 2.0], &[true, false]);
    assert_eq!((s.count, s.mean, s.sigma), (1.0, 2.0, 0.0));
    s.update(&[4.0, 6.0], &[false; 2]);
    assert_eq!(s.count, 3.0);
    near(s.mean as f32, 4.0, 1e-6);
    near(s.sigma as f32, (8.0_f32 / 3.0).sqrt(), 1e-6);
    let before = s;
    s.update(&[-1000.0, 1000.0], &[true; 2]);
    assert_eq!(
        (s.count, s.mean, s.sigma),
        (before.count, before.mean, before.sigma)
    );
}

// Independent scalar equations keep this test separate from Burn tensor kernels.
fn reference_linear(f: &Fixture, name: &str, x: &[f64]) -> Vec<f64> {
    let (shape, weights) = &f[&format!("{name}.weight")];
    (0..shape[0])
        .map(|row| {
            x.iter()
                .enumerate()
                .map(|(col, x)| weights[row * shape[1] + col] as f64 * x)
                .sum::<f64>()
        })
        .collect()
}
fn reference_norm(f: &Fixture, name: &str, x: &[f64]) -> Vec<f64> {
    let scale = &f[&format!("{name}.scale")].1;
    let divisor = (x.iter().map(|x| x * x).sum::<f64>() / x.len() as f64 + 1e-6).sqrt();
    x.iter()
        .zip(scale)
        .map(|(x, s)| x / divisor * *s as f64)
        .collect()
}
fn reference_layer(f: &Fixture, inputs: &[Vec<f64>], masked: usize) -> Vec<Vec<f64>> {
    let c = config();
    let p = "stacked_xf.0";
    let width = c.hidden_size;
    let head = c.head_dim;
    let mut queries = Vec::new();
    let mut keys = Vec::new();
    let mut values = Vec::new();
    for (index, input) in inputs.iter().enumerate() {
        let normed = reference_norm(f, &format!("{p}.pre_attn_ln"), input);
        let projected = reference_linear(f, &format!("{p}.attn.qkv_proj"), &normed);
        let mut q = Vec::new();
        let mut k = Vec::new();
        for (offset, target, norm) in [(0, &mut q, "query_ln"), (width, &mut k, "key_ln")] {
            for h in 0..c.num_attention_heads {
                let row = &projected[offset + h * head..offset + (h + 1) * head];
                let mut rotated = vec![0.0; head];
                for dim in 0..head / 2 {
                    let angle = (index as f64 - masked as f64)
                        / 10_000_f64.powf(2.0 * dim as f64 / head as f64);
                    rotated[dim] = row[dim] * angle.cos() - row[dim + head / 2] * angle.sin();
                    rotated[dim + head / 2] =
                        row[dim + head / 2] * angle.cos() + row[dim] * angle.sin();
                }
                target.extend(reference_norm(f, &format!("{p}.attn.{norm}"), &rotated));
            }
        }
        let scales = &f[&format!("{p}.attn.per_dim_scale.per_dim_scale")].1;
        for (i, value) in q.iter_mut().enumerate() {
            *value *=
                1.442_695_041 / (head as f64).sqrt() * (scales[i % head] as f64).exp().ln_1p();
        }
        queries.push(q);
        keys.push(k);
        values.push(projected[2 * width..].to_vec());
    }
    inputs
        .iter()
        .enumerate()
        .map(|(i, input)| {
            let mut attended = vec![0.0; width];
            for h in 0..c.num_attention_heads {
                let start = h * head;
                let scores: Vec<f64> = (masked..=i)
                    .map(|j| {
                        (0..head)
                            .map(|d| queries[i][start + d] * keys[j][start + d])
                            .sum()
                    })
                    .collect();
                let maximum = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                let exps: Vec<f64> = scores.iter().map(|s| (s - maximum).exp()).collect();
                let total = exps.iter().sum::<f64>();
                for (j, exp) in (masked..=i).zip(exps) {
                    for d in 0..head {
                        attended[start + d] += exp / total * values[j][start + d];
                    }
                }
            }
            let projected = reference_linear(f, &format!("{p}.attn.out"), &attended);
            let post = reference_norm(f, &format!("{p}.post_attn_ln"), &projected);
            let residual: Vec<f64> = input.iter().zip(post).map(|(x, y)| x + y).collect();
            let pre = reference_norm(f, &format!("{p}.pre_ff_ln"), &residual);
            let hidden: Vec<f64> = reference_linear(f, &format!("{p}.ff0"), &pre)
                .into_iter()
                .map(|x| x / (1.0 + (-x).exp()))
                .collect();
            let out = reference_linear(f, &format!("{p}.ff1"), &hidden);
            let post = reference_norm(f, &format!("{p}.post_ff_ln"), &out);
            residual.iter().zip(post).map(|(x, y)| x + y).collect()
        })
        .collect()
}

#[test]
fn transformer_matches_scalar_reference_and_cached_decoding() {
    let data = fixture(true);
    let dir = write_fixture(&data);
    let device = Device::flex();
    let weights = Weights::open(dir.path()).unwrap();
    let layer = Layer::load(&weights, 0, &config(), &device).unwrap();
    let inputs: Vec<Vec<f64>> = (0..4)
        .map(|r| {
            (0..8)
                .map(|c| ((r * 3 + c * 7) as f64 - 8.0) * 0.1)
                .collect()
        })
        .collect();
    let expected = reference_layer(&data, &inputs, 1);
    let tensor = |rows: &[Vec<f64>]| {
        Tensor::<3>::from_data(
            TensorData::new(
                rows.iter().flatten().map(|x| *x as f32).collect::<Vec<_>>(),
                [1, rows.len(), 8],
            ),
            &device,
        )
    };
    let output = layer
        .forward(tensor(&inputs), 1, &mut None)
        .into_data()
        .try_to_vec::<f32>()
        .unwrap();
    for (actual, expected) in output.iter().zip(expected.iter().flatten()) {
        near(*actual, *expected as f32, 2e-5);
    }
    let unpadded = layer
        .forward(tensor(&inputs[1..]), 0, &mut None)
        .into_data()
        .try_to_vec::<f32>()
        .unwrap();
    for (actual, padded) in unpadded.iter().zip(&output[8..]) {
        near(*actual, *padded, 2e-5);
    }
    let mut cache = None;
    let _ = layer.forward(tensor(&inputs[..2]), 1, &mut cache);
    let tail = layer
        .forward(tensor(&inputs[2..]), 0, &mut cache)
        .into_data()
        .try_to_vec::<f32>()
        .unwrap();
    for (cached, full) in tail.iter().zip(&output[16..]) {
        near(*cached, *full, 2e-5);
    }
    assert_eq!(cache.as_ref().unwrap().key.dims(), [1, 4, 2, 4]);
}

#[test]
fn forecast_runs_multiple_decode_blocks_and_preserves_quantile_contract() {
    let dir = write_fixture(&fixture(false));
    let model = TimesFm::load(dir.path(), &Device::flex()).unwrap();
    let mut req = request(vec![99.0, 1.0, 99.0, 3.0, 99.0], 7);
    req.series[0].observed_mask = Some(vec![vec![false, true, false, true, false]]);
    req.quantile_levels = vec![0.1, 0.5, 0.9];
    let result = model.forecast(&req).unwrap();
    assert_eq!(result.point_statistic, PointStatistic::Median);
    assert_eq!(result.quantile_levels, req.quantile_levels);
    assert_eq!(result.series[0].point[0].len(), 7);
    for row in &result.series[0].quantiles[0] {
        assert_eq!(row.len(), 3);
        for x in row {
            near(*x, 2.25, 1e-6);
        }
    }
    let caps = model.capabilities();
    assert_eq!(caps.max_context, 24);
    assert_eq!(caps.max_horizon, Some(8));
    assert_eq!(caps.native_horizon, 4);
    let mut bad = req.clone();
    bad.horizon = 9;
    assert!(model.forecast(&bad).is_err());
    bad = req.clone();
    bad.quantile_levels = vec![0.05];
    assert!(model.forecast(&bad).is_err());
    bad = req.clone();
    bad.series[0].past_covariates.push(vec![1.0; 5]);
    assert!(model.forecast(&bad).is_err());
    bad = req;
    bad.series[0].targets.push(vec![1.0; 5]);
    bad.series[0].observed_mask = None;
    assert!(model.forecast(&bad).is_err());
}

#[test]
fn continuous_head_uses_median_centered_spreads_and_repairs_crossings() {
    let prepared = Prepared {
        values: vec![],
        missing: vec![],
        mean: 10.0,
        sigma: 2.0,
        positive: false,
    };
    let mut forward = [0.0; CHANNELS];
    forward[0] = 1000.0;
    forward[MEDIAN] = 6.0;
    let mut reverse = [0.0; CHANNELS];
    reverse[MEDIAN] = -2.0;
    let spread = [80.0, -4.0, -3.0, 5.0, -1.0, 0.0, 1.0, -5.0, 3.0, 4.0];
    let reverse_spread = [0.0; CHANNELS];
    let result = combine(
        &[forward],
        &[spread],
        &[reverse],
        &[reverse_spread],
        &prepared,
    );
    assert_eq!(result[0][MEDIAN], 18.0);
    assert_eq!(
        result[0][1..],
        [14.0, 15.0, 17.0, 17.0, 18.0, 19.0, 19.0, 21.0, 22.0]
    );
    // The head's mean channel never becomes a returned quantile or point.
    assert_eq!(result[0][0], 1010.0);
}

#[test]
fn malformed_config_and_checkpoint_shapes_fail_before_tensor_loading() {
    let mut c = config_json();
    c["hidden_size"] = serde_json::json!(usize::MAX);
    assert!(
        serde_json::from_value::<Config>(c)
            .unwrap()
            .validate()
            .is_err()
    );
    let mut c = config_json();
    c["context_length"] = serde_json::json!(usize::MAX);
    assert!(
        serde_json::from_value::<Config>(c)
            .unwrap()
            .validate()
            .is_err()
    );
    let mut c = config_json();
    c["model_type"] = serde_json::json!("timesfm3");
    assert!(
        serde_json::from_value::<Config>(c)
            .unwrap()
            .validate()
            .is_err()
    );
    let mut data = fixture(true);
    data.get_mut("stacked_xf.0.attn.query_ln.scale").unwrap().0 = vec![2, 2];
    let dir = write_fixture(&data);
    assert!(
        matches!(TimesFm::load(dir.path(), &Device::flex()), Err(Error::Invalid(message)) if message.contains("must have shape"))
    );
    let dir = tempfile::tempdir().unwrap();
    let mut c = config_json();
    c["patch_length"] = serde_json::json!(0);
    std::fs::write(
        dir.path().join("config.json"),
        serde_json::to_vec(&c).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        TimesFm::load(dir.path(), &Device::flex()),
        Err(Error::Invalid(_))
    ));
}

#[test]
#[ignore = "requires official TimesFM 2.5 weights in FLOW_LIKE_TIMESFM_CHECKPOINT"]
fn official_checkpoint_forecasts_with_native_continuous_quantiles() {
    let directory = std::env::var("FLOW_LIKE_TIMESFM_CHECKPOINT")
        .expect("set FLOW_LIKE_TIMESFM_CHECKPOINT to a local official checkpoint");
    let model = TimesFm::load(Path::new(&directory), &Device::flex()).unwrap();
    let values = (0..96)
        .map(|i| 10.0 + (i as f32 * 0.13).sin() + i as f32 * 0.025)
        .collect();
    let request = request(values, 132);
    let result = model.forecast(&request).unwrap();
    result.validate(&request).unwrap();
    assert_eq!(result.quantile_levels, QUANTILES);
    assert_eq!(result.point_statistic, PointStatistic::Median);
    for (point, row) in result.series[0].point[0]
        .iter()
        .zip(&result.series[0].quantiles[0])
    {
        assert!(point.is_finite() && *point >= 0.0);
        assert_eq!(*point, row[4]);
        assert!(row.windows(2).all(|q| q[0] <= q[1]));
    }
    assert!(
        result.series[0].quantiles[0]
            .iter()
            .any(|q| q[8] - q[0] > 0.001)
    );
    println!(
        "TimesFM 2.5 median first5={:?}, final={}",
        &result.series[0].point[0][..5],
        result.series[0].point[0][131]
    );
}
