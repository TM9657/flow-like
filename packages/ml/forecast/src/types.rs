use crate::{Error, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const MAX_FORECAST_HORIZON: usize = 65_536;
pub const MAX_FORECAST_ELEMENTS: usize = 16_777_216;

/// Maximum elements in one native inference workspace tensor.
pub const MAX_WORKSPACE_ELEMENTS: usize = 67_108_864;

#[cfg(any(feature = "engine", test))]
pub(crate) fn validate_workspace(dimensions: &[usize], name: &str) -> Result<()> {
    let elements = dimensions
        .iter()
        .try_fold(1_usize, |count, dimension| count.checked_mul(*dimension));
    require(
        elements.is_some_and(|count| count <= MAX_WORKSPACE_ELEMENTS),
        format!(
            "{name} exceeds the {MAX_WORKSPACE_ELEMENTS} element workspace limit; reduce history, variables or horizon"
        ),
    )
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ForecastFamily {
    Timestral,
    Chronos2,
    Toto2,
    #[serde(rename = "timesfm2_5")]
    Timesfm25,
    TinyTimeMixer,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ForecastCapabilities {
    pub family: ForecastFamily,
    /// Maximum historical steps supported by the adapter.
    pub max_context: usize,
    /// Hard horizon ceiling, if any. Some models also limit context plus horizon.
    pub max_horizon: Option<usize>,
    /// Steps in a native decoding block; Toto 2 can emit several blocks in one pass.
    pub native_horizon: usize,
    pub quantile_levels: Vec<f32>,
    pub joint_multivariate: bool,
    pub past_covariates: bool,
    pub known_covariates: bool,
}

/// Variables in one entry are modeled jointly; separate entries are independent.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ForecastSeries {
    pub id: String,
    /// Rows are target variables; columns are equally spaced historical steps.
    pub targets: Vec<Vec<f32>>,
    /// False marks missing observations. Values must still be finite on the wire.
    #[serde(default)]
    pub observed_mask: Option<Vec<Vec<bool>>>,
    /// Numeric auxiliary rows covering the historical window.
    #[serde(default)]
    pub past_covariates: Vec<Vec<f32>>,
    /// Numeric auxiliary rows covering history followed by the complete forecast horizon.
    #[serde(default)]
    pub known_covariates: Vec<Vec<f32>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ForecastRequest {
    pub series: Vec<ForecastSeries>,
    /// Number of future steps at the same sampling interval as the history.
    pub horizon: usize,
    /// Empty selects the checkpoint's native quantiles.
    #[serde(default)]
    pub quantile_levels: Vec<f32>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PointStatistic {
    Mean,
    Median,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct SeriesForecast {
    pub id: String,
    /// Target variable, then forecast step.
    pub point: Vec<Vec<f32>>,
    /// Target variable, forecast step, then quantile level.
    pub quantiles: Vec<Vec<Vec<f32>>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ForecastResult {
    pub series: Vec<SeriesForecast>,
    pub quantile_levels: Vec<f32>,
    pub point_statistic: PointStatistic,
}

fn require(ok: bool, message: impl Into<String>) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(Error::Invalid(message.into()))
    }
}

fn validate_levels(levels: &[f32]) -> Result<()> {
    require(
        levels.iter().all(|q| q.is_finite() && *q > 0.0 && *q < 1.0)
            && levels.windows(2).all(|q| q[0] < q[1]),
        "Quantile levels must be finite, strictly increasing and between zero and one",
    )
}

impl ForecastRequest {
    pub fn validate(&self) -> Result<()> {
        require(
            self.horizon > 0 && self.horizon <= MAX_FORECAST_HORIZON,
            format!("Forecast horizon must be between 1 and {MAX_FORECAST_HORIZON} steps"),
        )?;
        require(!self.series.is_empty(), "At least one series is required")?;
        validate_levels(&self.quantile_levels)?;
        let mut ids = HashSet::new();
        let mut input_elements = 0_usize;
        let mut output_elements = 0_usize;
        for series in &self.series {
            output_elements = series
                .targets
                .len()
                .checked_mul(self.horizon)
                .and_then(|n| output_elements.checked_add(n))
                .ok_or_else(|| Error::Invalid("Forecast dimensions overflow".into()))?;
            require(
                output_elements <= MAX_FORECAST_ELEMENTS,
                "Forecast exceeds the output element limit",
            )?;
            require(
                !series.id.trim().is_empty() && ids.insert(&series.id),
                "Series IDs must be nonempty and unique",
            )?;
            let history = series.targets.first().map_or(0, Vec::len);
            require(
                history > 0,
                format!("Series {} has no historical targets", series.id),
            )?;
            let known_length = history
                .checked_add(self.horizon)
                .ok_or_else(|| Error::Invalid("History plus horizon overflows".into()))?;
            for (name, rows, length) in [
                ("targets", &series.targets, history),
                ("past_covariates", &series.past_covariates, history),
                ("known_covariates", &series.known_covariates, known_length),
            ] {
                input_elements = rows
                    .len()
                    .checked_mul(length)
                    .and_then(|n| input_elements.checked_add(n))
                    .ok_or_else(|| Error::Invalid("Forecast input dimensions overflow".into()))?;
                require(
                    input_elements <= MAX_FORECAST_ELEMENTS,
                    "Forecast exceeds the input element limit",
                )?;
                require(
                    rows.iter()
                        .all(|row| row.len() == length && row.iter().all(|v| v.is_finite())),
                    format!(
                        "Series {} {name} must contain finite rows of length {length}",
                        series.id
                    ),
                )?;
            }
            if let Some(mask) = &series.observed_mask {
                require(
                    mask.len() == series.targets.len()
                        && mask.iter().all(|row| row.len() == history),
                    format!(
                        "Series {} observation mask must match its targets",
                        series.id
                    ),
                )?;
                require(
                    mask.iter().all(|row| row.iter().any(|observed| *observed)),
                    format!(
                        "Every target in series {} needs at least one observation",
                        series.id
                    ),
                )?;
            }
        }
        Ok(())
    }

    pub fn validate_for_quantiles(&self, native_quantiles: usize) -> Result<()> {
        self.validate()?;
        let levels = native_quantiles
            .max(self.quantile_levels.len())
            .checked_add(1)
            .ok_or_else(|| Error::Invalid("Forecast quantile dimensions overflow".into()))?;
        let targets = self
            .series
            .iter()
            .try_fold(0_usize, |count, series| {
                count.checked_add(series.targets.len())
            })
            .ok_or_else(|| Error::Invalid("Forecast target dimensions overflow".into()))?;
        let elements = targets
            .checked_mul(self.horizon)
            .and_then(|n| n.checked_mul(levels))
            .ok_or_else(|| Error::Invalid("Forecast output dimensions overflow".into()))?;
        require(
            elements <= MAX_FORECAST_ELEMENTS,
            format!(
                "Forecast points and quantiles exceed the {MAX_FORECAST_ELEMENTS} element limit; reduce the batch, variables or horizon"
            ),
        )
    }
}

impl ForecastResult {
    pub fn validate(&self, request: &ForecastRequest) -> Result<()> {
        validate_levels(&self.quantile_levels)?;
        // Point-only models retain the same target/horizon shape with empty quantile vectors.
        require(
            request.quantile_levels.is_empty() || request.quantile_levels == self.quantile_levels,
            "Forecast returned different quantile levels than requested",
        )?;
        require(
            self.series.len() == request.series.len(),
            "Forecast series count does not match the request",
        )?;
        for (result, input) in self.series.iter().zip(&request.series) {
            require(
                result.id == input.id
                    && result.point.len() == input.targets.len()
                    && result.quantiles.len() == input.targets.len(),
                "Forecast IDs or target dimensions do not match the request",
            )?;
            require(
                result
                    .point
                    .iter()
                    .all(|row| row.len() == request.horizon && row.iter().all(|v| v.is_finite())),
                "Forecast point values must be finite and cover the horizon",
            )?;
            require(
                result.quantiles.iter().all(|row| {
                    row.len() == request.horizon
                        && row.iter().all(|step| {
                            step.len() == self.quantile_levels.len()
                                && step.iter().all(|v| v.is_finite())
                        })
                }),
                "Forecast quantiles must be finite and match the horizon and levels",
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> ForecastRequest {
        ForecastRequest {
            series: vec![ForecastSeries {
                id: "sensor".into(),
                targets: vec![vec![1.0, 0.0, 3.0]],
                observed_mask: Some(vec![vec![true, false, true]]),
                past_covariates: vec![],
                known_covariates: vec![vec![1.0; 5]],
            }],
            horizon: 2,
            quantile_levels: vec![0.1, 0.5, 0.9],
        }
    }

    #[test]
    fn missing_observations_round_trip_without_nonfinite_json() {
        let request = request();
        let round_trip: ForecastRequest =
            serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();
        round_trip.validate().unwrap();
        assert!(!round_trip.series[0].observed_mask.as_ref().unwrap()[0][1]);
    }

    #[test]
    fn rejects_misaligned_covariates_and_empty_targets() {
        let mut request = request();
        request.series[0].known_covariates[0].pop();
        assert!(request.validate().is_err());
        request.series[0].known_covariates.clear();
        request.series[0].observed_mask = Some(vec![vec![false; 3]]);
        assert!(request.validate().is_err());
    }

    #[test]
    fn rejects_duplicate_ids_and_invalid_quantiles() {
        let mut request = request();
        request.series.push(request.series[0].clone());
        assert!(request.validate().is_err());
        request.series.pop();
        request.quantile_levels = vec![0.9, 0.1];
        assert!(request.validate().is_err());
        request.quantile_levels = vec![f32::NAN];
        assert!(request.validate().is_err());
    }

    #[test]
    fn point_only_results_keep_target_and_horizon_shape_without_invented_quantiles() {
        let mut request = request();
        request.quantile_levels.clear();
        let mut result = ForecastResult {
            quantile_levels: vec![],
            point_statistic: PointStatistic::Mean,
            series: vec![SeriesForecast {
                id: "sensor".into(),
                point: vec![vec![4.0, 5.0]],
                quantiles: vec![vec![vec![], vec![]]],
            }],
        };
        result.validate(&request).unwrap();
        request.quantile_levels = vec![0.5];
        assert!(result.validate(&request).is_err());
        request.quantile_levels.clear();
        result.series[0].quantiles[0][0].push(4.0);
        assert!(result.validate(&request).is_err());
        result.series[0].quantiles[0][0].clear();
        result.series[0].quantiles[0].pop();
        assert!(result.validate(&request).is_err());
    }

    #[test]
    fn rejects_unbounded_horizons_and_quantile_outputs_before_allocation() {
        let mut request = request();
        request.series[0].known_covariates.clear();
        request.horizon = usize::MAX - 2;
        assert!(request.validate().is_err());
        request.horizon = MAX_FORECAST_HORIZON;
        request.series[0].targets = vec![vec![1.0]; 16];
        request.series[0].observed_mask = None;
        request.validate().unwrap();
        assert!(request.validate_for_quantiles(21).is_err());
    }

    #[test]
    fn workspace_limits_reject_dense_attention_and_dimension_overflow() {
        validate_workspace(&[4, 8, 128, 128], "attention").unwrap();
        assert!(validate_workspace(&[1, 16, 8192, 8192], "attention").is_err());
        assert!(validate_workspace(&[usize::MAX, 2], "attention").is_err());
    }
}
