use super::*;
use crate::inspection::forecasting::{
    studio::{StudioBacktestMetric, StudioCovariateKind, StudioForecastPoint, preparation},
    tables::preparation::{PreparedForecast, future_timestamp},
};
use flow_like_ml_forecast::{ForecastResult, SeriesForecast};
use flow_like_types::{Result, Value, anyhow, bail, json::json};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn identifier(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}

fn metric_value_matches(expected: Option<f64>, saved: Option<f64>) -> bool {
    match (expected, saved) {
        (None, None) => true,
        (Some(expected), Some(saved)) if expected.is_finite() && saved.is_finite() => {
            // JSON transport can change the last few binary digits of an f64.
            let tolerance = 8.0 * f64::EPSILON * expected.abs().max(saved.abs());
            (expected - saved).abs() <= tolerance
        }
        _ => false,
    }
}

fn metrics_match(expected: &[StudioBacktestMetric], saved: &[StudioBacktestMetric]) -> bool {
    expected.len() == saved.len()
        && expected.iter().zip(saved).all(|(expected, saved)| {
            expected.series_id == saved.series_id
                && expected.target == saved.target
                && expected.observations == saved.observations
                && metric_value_matches(expected.mae, saved.mae)
                && metric_value_matches(expected.rmse, saved.rmse)
                && metric_value_matches(expected.mape, saved.mape)
                // Older snapshots have no WAPE field. Validate it whenever supplied.
                && (saved.wape.is_none() || metric_value_matches(expected.wape, saved.wape))
        })
}

fn points(
    prepared: &PreparedForecast,
    points: &[StudioForecastPoint],
    result: &StudioForecastResult,
    actual: Option<&PreparedForecast>,
) -> Result<()> {
    let mut indexed = BTreeMap::new();
    for point in points {
        if indexed
            .insert((&point.series_id, &point.target, point.timestamp_ms), point)
            .is_some()
        {
            bail!("Saved forecast contains duplicate forecast points");
        }
    }
    let mut series = Vec::new();
    for (index, source) in prepared.request.series.iter().enumerate() {
        let (last, frequency) = prepared.clocks[index];
        let mut point_values = Vec::new();
        let mut quantiles = Vec::new();
        for (target, name) in prepared.targets.iter().enumerate() {
            let mut target_points = Vec::new();
            let mut target_quantiles = Vec::new();
            for step in 0..prepared.request.horizon {
                let time = future_timestamp(last, frequency, step + 1)?;
                let point = indexed.remove(&(&source.id, name, time)).ok_or_else(|| {
                    anyhow!(
                        "Saved forecast is missing a prediction for its target, series or timestamp"
                    )
                })?;
                let expected_actual = actual.and_then(|full| {
                    let full = &full.request.series[index];
                    let position = source.targets[target].len() + step;
                    full.observed_mask
                        .as_ref()
                        .is_none_or(|mask| mask[target][position])
                        .then_some(full.targets[target][position])
                });
                if point.actual != expected_actual {
                    bail!("Saved forecast actuals do not match its holdout history");
                }
                target_points.push(point.point);
                target_quantiles.push(point.quantiles.clone());
            }
            point_values.push(target_points);
            quantiles.push(target_quantiles);
        }
        series.push(SeriesForecast {
            id: source.id.clone(),
            point: point_values,
            quantiles,
        });
    }
    if !indexed.is_empty() {
        bail!("Saved forecast contains unexpected prediction points");
    }
    ForecastResult {
        series,
        quantile_levels: result.quantile_levels.clone(),
        point_statistic: result.point_statistic,
    }
    .validate(&prepared.request)?;
    Ok(())
}

pub(super) fn payload(payload: &CreateStudioForecast) -> Result<()> {
    if payload.name.trim().is_empty()
        || payload.name.chars().count() > 256
        || payload.name.chars().any(char::is_control)
    {
        bail!("Forecast name must contain 1 to 256 characters without control characters");
    }
    if payload.description.as_ref().is_some_and(|description| {
        description.chars().count() > 4000
            || description
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    }) {
        bail!("Forecast description must contain at most 4000 characters");
    }
    preparation::validate_serialized_size(payload, MAX_SNAPSHOT_BYTES)?;
    let request = &payload.request;
    let result = &payload.result;
    let budget = preparation::budget(request)?;
    if result.model.bit_id != request.model_bit_id
        || result.horizon != request.horizon
        || result.model.family != result.capabilities.family
        || !identifier(&result.run_id)
        || (!request.quantile_levels.is_empty()
            && request.quantile_levels != result.quantile_levels)
        || (request.quantile_levels.is_empty()
            && result.quantile_levels != result.capabilities.quantile_levels)
    {
        bail!("Saved forecast result does not match its request or output limits");
    }
    let mapping = &request.mapping;
    let mut history = BTreeMap::<(String, i64), serde_json::Map<String, Value>>::new();
    let mut future = BTreeMap::<(String, i64), serde_json::Map<String, Value>>::new();
    let mut ids = BTreeSet::new();
    let row = |id: &str, timestamp: i64| {
        let mut row = serde_json::Map::new();
        row.insert(mapping.timestamp.clone(), json!(timestamp));
        if let Some(column) = &mapping.series_id {
            row.insert(column.clone(), json!(id));
        }
        row
    };
    for point in &result.history {
        if point.series_id.trim().is_empty()
            || (mapping.series_id.is_none() && point.series_id != "series")
            || !mapping.targets.contains(&point.target)
            || point.value.is_some_and(|v| !v.is_finite())
        {
            bail!("Saved forecast contains an invalid historical target");
        }
        ids.insert(point.series_id.clone());
        let record = history
            .entry((point.series_id.clone(), point.timestamp_ms))
            .or_insert_with(|| row(&point.series_id, point.timestamp_ms));
        if record
            .insert(point.target.clone(), json!(point.value))
            .is_some()
        {
            bail!("Saved forecast contains duplicate historical target points");
        }
    }
    for point in &result.covariates {
        let names = match point.kind {
            StudioCovariateKind::Past => &mapping.past_covariates,
            StudioCovariateKind::Known => &mapping.known_covariates,
        };
        if !ids.contains(&point.series_id)
            || !names.contains(&point.name)
            || !point.value.is_finite()
            || (point.future && matches!(point.kind, StudioCovariateKind::Past))
        {
            bail!("Saved forecast contains invalid covariate tracks");
        }
        let key = (point.series_id.clone(), point.timestamp_ms);
        let record = if point.future {
            future
                .entry(key)
                .or_insert_with(|| row(&point.series_id, point.timestamp_ms))
        } else {
            history
                .get_mut(&key)
                .ok_or_else(|| anyhow!("Saved covariate has no matching historical timestamp"))?
        };
        if record
            .insert(point.name.clone(), json!(point.value))
            .is_some()
        {
            bail!("Saved forecast contains duplicate covariate points");
        }
    }
    if result.input_rows != history.len() + future.len() {
        bail!("Saved forecast input row count is inconsistent");
    }
    // Saved result points are already normalized, regardless of the source format.
    let units = BTreeMap::from([(
        mapping.timestamp.clone(),
        crate::inspection::auto_training_tables::TemporalUnit::Milliseconds,
    )]);
    let future = (!mapping.known_covariates.is_empty())
        .then(|| (future.into_values().map(Value::Object).collect(), &units));
    let prepared = preparation::inputs(
        history.into_values().map(Value::Object).collect(),
        &units,
        future,
        request,
        &budget,
    )?;
    preparation::validate_capabilities(&prepared.request, &result.capabilities, &budget)?;
    points(&prepared, &result.forecasts, result, None)?;
    match (
        preparation::backtest(&prepared, request.backtest_horizon, request.backtest_offset)?,
        &result.backtest,
    ) {
        (None, None) => {}
        (Some(held), Some(saved))
            if saved.horizon == request.backtest_horizon
                && saved.offset == request.backtest_offset =>
        {
            preparation::validate_capabilities(&held.request, &result.capabilities, &budget)?;
            points(&held, &saved.forecasts, result, Some(&prepared))?;
            let expected = preparation::metrics(&saved.forecasts);
            if !metrics_match(&expected, &saved.metrics) {
                bail!("Saved backtest metrics do not match its predictions and observations");
            }
        }
        _ => bail!("Saved forecast holdout does not match its request"),
    }
    Ok(())
}
