use super::*;
use crate::inspection::auto_training_tables::TemporalUnit;
use crate::inspection::forecasting::tables::preparation::{
    PreparedForecast, future_timestamp, prepare, validate_output_budget,
};
use flow_like_ml_forecast::{ForecastRequest, ForecastResult};
use flow_like_types::{Result, anyhow, bail};
use std::collections::BTreeMap;

pub(super) const MAXIMUM_ROWS: usize = 50_000;
const MAXIMUM_BYTES: usize = 64 * 1024 * 1024;

pub(super) fn budget(request: &StudioForecastRequest) -> Result<TableBudget> {
    request.budget.validate()?;
    if request.model_bit_id.trim().is_empty() || request.model_bit_id.len() > 256 {
        bail!("Choose a forecasting model from the active profile");
    }
    if request.backtest_horizon > flow_like_ml_forecast::MAX_FORECAST_HORIZON {
        bail!("Backtest horizon exceeds the forecasting limit");
    }
    if request.backtest_offset > MAXIMUM_ROWS {
        bail!("Backtest offset exceeds the maximum historical row count");
    }
    if request.backtest_horizon == 0 && request.backtest_offset != 0 {
        bail!("Backtest offset requires a positive backtest horizon");
    }
    request.mapping.validate()?;
    if request.mapping.known_covariates.is_empty() != request.future_sql.is_none() {
        bail!("Provide future_sql exactly when known covariates are mapped");
    }
    for sql in std::iter::once(&request.sql).chain(request.future_sql.iter()) {
        if sql.trim().is_empty() || sql.len() > 256 * 1024 {
            bail!("Forecast SQL must be nonempty and at most 256 KiB");
        }
    }
    for params in [&request.params, &request.future_params] {
        validate_serialized_size(params, 1024 * 1024)?;
    }
    Ok(TableBudget {
        maximum_rows: request.budget.maximum_rows.min(MAXIMUM_ROWS),
        maximum_bytes: request.budget.maximum_bytes.min(MAXIMUM_BYTES),
        maximum_tensor_elements: request
            .budget
            .maximum_tensor_elements
            .min(flow_like_ml_forecast::MAX_FORECAST_ELEMENTS),
    })
}

pub(super) fn inputs(
    rows: Vec<Value>,
    units: &BTreeMap<String, TemporalUnit>,
    future: Option<(Vec<Value>, &BTreeMap<String, TemporalUnit>)>,
    request: &StudioForecastRequest,
    budget: &TableBudget,
) -> Result<PreparedForecast> {
    validate_serialized_size(
        &(&rows, &future.as_ref().map(|(rows, _)| rows)),
        budget.maximum_bytes,
    )?;
    let prepared = prepare(
        rows,
        units,
        future,
        &request.mapping,
        request.horizon,
        request.quantile_levels.clone(),
        budget,
    )?;
    // Bound chart materialization before loading weights or allocating chart rows.
    validate_chart_budget(
        &prepared,
        &request.mapping,
        request.backtest_horizon,
        0,
        budget,
    )?;
    Ok(prepared)
}

pub(super) fn backtest(
    prepared: &PreparedForecast,
    horizon: usize,
    offset: usize,
) -> Result<Option<PreparedForecast>> {
    if horizon == 0 {
        if offset != 0 {
            bail!("Backtest offset requires a positive backtest horizon");
        }
        return Ok(None);
    }
    let excluded = horizon
        .checked_add(offset)
        .ok_or_else(|| anyhow!("Backtest horizon and offset overflow"))?;
    let mut result = prepared.clone();
    result.request.horizon = horizon;
    for (series, clock) in result.request.series.iter_mut().zip(&mut result.clocks) {
        let length = series.targets[0].len();
        let train = length
            .checked_sub(excluded)
            .filter(|n| *n >= 2)
            .ok_or_else(|| {
                anyhow!(
                    "Series '{}' needs at least two observations before its {horizon}-step holdout with offset {offset}",
                    series.id
                )
            })?;
        for target in &mut series.targets {
            target.truncate(train);
        }
        if let Some(mask) = &mut series.observed_mask {
            for target in mask {
                target.truncate(train);
            }
        }
        for covariate in &mut series.past_covariates {
            covariate.truncate(train);
        }
        // Known covariates stop at the evaluated window's end, excluding later history.
        for covariate in &mut series.known_covariates {
            covariate.truncate(length - offset);
        }
        clock.0 = historical_timestamp(clock.0, clock.1, excluded)?;
    }
    result.request.validate()?;
    Ok(Some(result))
}

pub(super) fn validate_capabilities(
    request: &ForecastRequest,
    capabilities: &ForecastCapabilities,
    budget: &TableBudget,
) -> Result<()> {
    validate_output_budget(request, capabilities.quantile_levels.len(), budget)?;
    if capabilities.quantile_levels.is_empty() && !request.quantile_levels.is_empty() {
        bail!("This model provides point forecasts only; leave quantile levels empty");
    }
    if capabilities
        .max_horizon
        .is_some_and(|maximum| request.horizon > maximum)
    {
        bail!("Requested horizon exceeds this model's maximum horizon");
    }
    for series in &request.series {
        if series.targets[0].len() > capabilities.max_context {
            bail!(
                "Series '{}' has {} rows but this model accepts {} historical steps; filter the SQL history explicitly",
                series.id,
                series.targets[0].len(),
                capabilities.max_context
            );
        }
        if (!series.past_covariates.is_empty() && !capabilities.past_covariates)
            || (!series.known_covariates.is_empty() && !capabilities.known_covariates)
        {
            bail!("The selected model does not support the mapped covariates");
        }
        if series.targets.len() > 1 && !capabilities.joint_multivariate {
            bail!("The selected model does not support joint target forecasting");
        }
    }
    Ok(())
}

fn historical_timestamp(last: i64, frequency: i64, steps_before: usize) -> Result<i64> {
    i64::try_from(steps_before)
        .ok()
        .and_then(|steps| frequency.checked_mul(steps))
        .and_then(|offset| last.checked_sub(offset))
        .ok_or_else(|| anyhow!("Historical timestamps overflow epoch milliseconds"))
}

pub(super) fn validate_chart_budget(
    prepared: &PreparedForecast,
    mapping: &ForecastTableMapping,
    backtest_horizon: usize,
    quantiles: usize,
    budget: &TableBudget,
) -> Result<()> {
    let mut count = 0usize;
    let mut elements = 0usize;
    let mut bytes = 0usize;
    for series in &prepared.request.series {
        let length = series.targets[0].len();
        let history = length.checked_mul(
            mapping.targets.len() + mapping.past_covariates.len() + mapping.known_covariates.len(),
        );
        let future = prepared
            .request
            .horizon
            .checked_mul(mapping.known_covariates.len());
        let predictions = prepared
            .request
            .horizon
            .checked_add(backtest_horizon)
            .and_then(|steps| steps.checked_mul(mapping.targets.len()));
        let rows = history
            .and_then(|h| future.and_then(|f| h.checked_add(f)))
            .and_then(|rows| predictions.and_then(|p| rows.checked_add(p)))
            .ok_or_else(|| anyhow!("Forecast chart dimensions overflow"))?;
        count = count
            .checked_add(rows)
            .ok_or_else(|| anyhow!("Forecast chart dimensions overflow"))?;
        elements = predictions
            .and_then(|p| p.checked_mul(quantiles))
            .and_then(|q| q.checked_add(rows))
            .and_then(|n| elements.checked_add(n))
            .ok_or_else(|| anyhow!("Forecast chart dimensions overflow"))?;
        // IDs and names repeat in every chart point. Count their encoded length
        // before cloning them so a long ID cannot multiply into a large response.
        let id_bytes = flow_like_types::json::to_vec(&series.id)?.len();
        for (names, steps, quantiles) in [
            (&mapping.targets, length, 0),
            (
                &mapping.targets,
                prepared.request.horizon + backtest_horizon,
                quantiles,
            ),
            (&mapping.past_covariates, length, 0),
            (
                &mapping.known_covariates,
                length + prepared.request.horizon,
                0,
            ),
        ] {
            for name in names {
                let name_bytes = flow_like_types::json::to_vec(name)?.len();
                let point_bytes = quantiles
                    .checked_mul(32)
                    .and_then(|q| q.checked_add(256))
                    .and_then(|n| n.checked_add(id_bytes))
                    .and_then(|n| n.checked_add(name_bytes))
                    .and_then(|n| n.checked_mul(steps))
                    .ok_or_else(|| anyhow!("Forecast chart byte count overflows"))?;
                bytes = bytes
                    .checked_add(point_bytes)
                    .ok_or_else(|| anyhow!("Forecast chart byte count overflows"))?;
            }
        }
    }
    if count > budget.maximum_rows || elements > budget.maximum_tensor_elements {
        bail!(
            "History, covariates and forecast chart points exceed the row or tensor budget; reduce the SQL history, targets or horizon"
        );
    }
    if bytes > budget.maximum_bytes {
        bail!(
            "History, covariates and forecast chart points exceed the byte budget; reduce the history, horizon or identifier lengths"
        );
    }
    Ok(())
}

fn forecast_points(
    prepared: &PreparedForecast,
    result: &ForecastResult,
    actual: Option<&PreparedForecast>,
) -> Result<Vec<StudioForecastPoint>> {
    result.validate(&prepared.request)?;
    let mut output = Vec::new();
    for (index, series) in result.series.iter().enumerate() {
        let (last, frequency) = prepared.clocks[index];
        for (target, name) in prepared.targets.iter().enumerate() {
            for step in 0..prepared.request.horizon {
                let actual = actual.and_then(|full| {
                    let full = &full.request.series[index];
                    let offset = prepared.request.series[index].targets[target].len() + step;
                    full.observed_mask
                        .as_ref()
                        .is_none_or(|m| m[target][offset])
                        .then_some(full.targets[target][offset])
                });
                output.push(StudioForecastPoint {
                    series_id: series.id.clone(),
                    timestamp_ms: future_timestamp(last, frequency, step + 1)?,
                    target: name.clone(),
                    point: series.point[target][step],
                    quantiles: series.quantiles[target][step].clone(),
                    actual,
                });
            }
        }
    }
    Ok(output)
}

pub(super) fn metrics(points: &[StudioForecastPoint]) -> Vec<StudioBacktestMetric> {
    let mut grouped = BTreeMap::<(&str, &str), (usize, f64, f64, f64, bool, f64)>::new();
    for point in points {
        let totals = grouped
            .entry((&point.series_id, &point.target))
            .or_default();
        if let Some(actual) = point.actual {
            let error = (f64::from(actual) - f64::from(point.point)).abs();
            totals.0 += 1;
            totals.1 += error;
            totals.2 += error * error;
            totals.5 += f64::from(actual).abs();
            if actual == 0.0 {
                totals.4 = true;
            } else {
                totals.3 += error / f64::from(actual).abs();
            }
        }
    }
    grouped
        .into_iter()
        .map(
            |((series, target), (n, absolute, squared, percentage, zero, actual_magnitude))| {
                StudioBacktestMetric {
                    series_id: series.into(),
                    target: target.into(),
                    observations: n,
                    mae: (n > 0).then(|| absolute / n as f64),
                    rmse: (n > 0).then(|| (squared / n as f64).sqrt()),
                    mape: (n > 0 && !zero).then(|| 100.0 * percentage / n as f64),
                    wape: (actual_magnitude > 0.0).then(|| 100.0 * absolute / actual_magnitude),
                }
            },
        )
        .collect()
}

pub(super) fn finish(
    request: &StudioForecastRequest,
    prepared: PreparedForecast,
    result: ForecastResult,
    holdout: Option<(PreparedForecast, ForecastResult)>,
    model: StudioForecastModel,
    capabilities: ForecastCapabilities,
    input_rows: usize,
    budget: &TableBudget,
) -> Result<StudioForecastResult> {
    validate_chart_budget(
        &prepared,
        &request.mapping,
        request.backtest_horizon,
        result.quantile_levels.len(),
        budget,
    )?;
    let forecasts = forecast_points(&prepared, &result, None)?;
    let backtest = match holdout {
        None => None,
        Some((held, predicted)) => {
            if result.quantile_levels != predicted.quantile_levels
                || result.point_statistic != predicted.point_statistic
            {
                bail!("Backtest and forecast returned inconsistent output statistics");
            }
            let forecasts = forecast_points(&held, &predicted, Some(&prepared))?;
            let metrics = metrics(&forecasts);
            Some(StudioBacktestResult {
                horizon: held.request.horizon,
                offset: request.backtest_offset,
                forecasts,
                metrics,
            })
        }
    };
    let mut history = Vec::new();
    let mut covariates = Vec::new();
    for (series, &(last, frequency)) in prepared.request.series.iter().zip(&prepared.clocks) {
        let length = series.targets[0].len();
        for (target, name) in prepared.targets.iter().enumerate() {
            for step in 0..length {
                history.push(StudioHistoryPoint {
                    series_id: series.id.clone(),
                    timestamp_ms: historical_timestamp(last, frequency, length - step - 1)?,
                    target: name.clone(),
                    value: series
                        .observed_mask
                        .as_ref()
                        .is_none_or(|m| m[target][step])
                        .then_some(series.targets[target][step]),
                });
            }
        }
        for (kind, names, rows) in [
            (
                StudioCovariateKind::Past,
                &request.mapping.past_covariates,
                &series.past_covariates,
            ),
            (
                StudioCovariateKind::Known,
                &request.mapping.known_covariates,
                &series.known_covariates,
            ),
        ] {
            for (name, values) in names.iter().zip(rows) {
                for (step, &value) in values.iter().enumerate() {
                    covariates.push(StudioCovariatePoint {
                        series_id: series.id.clone(),
                        name: name.clone(),
                        kind,
                        value,
                        future: step >= length,
                        timestamp_ms: if step < length {
                            historical_timestamp(last, frequency, length - step - 1)?
                        } else {
                            future_timestamp(last, frequency, step - length + 1)?
                        },
                    });
                }
            }
        }
    }
    let output = StudioForecastResult {
        run_id: flow_like_types::create_id(),
        model,
        capabilities,
        input_rows,
        horizon: prepared.request.horizon,
        quantile_levels: result.quantile_levels,
        point_statistic: result.point_statistic,
        history,
        forecasts,
        covariates,
        backtest,
    };
    validate_serialized_size(&output, budget.maximum_bytes)?;
    Ok(output)
}

pub(super) fn validate_serialized_size(value: &impl Serialize, maximum: usize) -> Result<()> {
    struct Counter {
        count: usize,
        maximum: usize,
    }
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.count = self
                .count
                .checked_add(bytes.len())
                .filter(|n| *n <= self.maximum)
                .ok_or_else(|| std::io::Error::other("Forecast data exceed the byte budget"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Counter { count: 0, maximum }, value)?;
    Ok(())
}
