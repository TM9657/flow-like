//! Native inference for pretrained time-series forecasting checkpoints.

mod types;
pub use types::*;

#[cfg(feature = "engine")]
pub mod chronos;
#[cfg(feature = "engine")]
pub mod timesfm;
#[cfg(feature = "engine")]
pub mod timestral;
#[cfg(feature = "engine")]
pub mod tiny_time_mixer;
#[cfg(feature = "engine")]
pub mod toto;
#[cfg(feature = "engine")]
pub mod weights;

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(feature = "cpu")]
pub fn load_cpu(family: ForecastFamily, directory: &std::path::Path) -> Result<ForecastModel> {
    ForecastModel::load(family, directory, &burn::tensor::Device::flex())
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Tensor(String),
}

#[cfg(feature = "engine")]
pub enum ForecastModel {
    Timestral(timestral::Timestral),
    Chronos2(chronos::Chronos2),
    Toto2(toto::Toto2),
    Timesfm25(timesfm::TimesFm),
    TinyTimeMixer(tiny_time_mixer::TinyTimeMixer),
}

#[cfg(feature = "engine")]
impl ForecastModel {
    pub fn capabilities(&self) -> ForecastCapabilities {
        let mut capabilities = match self {
            Self::Timestral(model) => model.capabilities(),
            Self::Chronos2(model) => model.capabilities(),
            Self::Toto2(model) => model.capabilities(),
            Self::Timesfm25(model) => model.capabilities(),
            Self::TinyTimeMixer(model) => model.capabilities(),
        };
        capabilities.max_horizon = Some(
            capabilities
                .max_horizon
                .unwrap_or(MAX_FORECAST_HORIZON)
                .min(MAX_FORECAST_HORIZON),
        );
        capabilities
    }

    /// Loads an upstream config.json and its SafeTensors checkpoint files.
    pub fn load(
        family: ForecastFamily,
        directory: &std::path::Path,
        device: &burn::tensor::Device,
    ) -> Result<Self> {
        match family {
            ForecastFamily::Timestral => {
                timestral::Timestral::load(directory, device).map(Self::Timestral)
            }
            ForecastFamily::Chronos2 => {
                chronos::Chronos2::load(directory, device).map(Self::Chronos2)
            }
            ForecastFamily::Toto2 => toto::Toto2::load(directory, device).map(Self::Toto2),
            ForecastFamily::Timesfm25 => {
                timesfm::TimesFm::load(directory, device).map(Self::Timesfm25)
            }
            ForecastFamily::TinyTimeMixer => {
                tiny_time_mixer::TinyTimeMixer::load(directory, device).map(Self::TinyTimeMixer)
            }
        }
    }

    pub fn forecast(&self, request: &ForecastRequest) -> Result<ForecastResult> {
        request.validate()?;
        let result = match self {
            Self::Timestral(model) => model.forecast(request),
            Self::Chronos2(model) => model.forecast(request),
            Self::Toto2(model) => model.forecast(request),
            Self::Timesfm25(model) => model.forecast(request),
            Self::TinyTimeMixer(model) => model.forecast(request),
        }?;
        result.validate(request)?;
        if request.quantile_levels.is_empty()
            && result.quantile_levels != self.capabilities().quantile_levels
        {
            return Err(Error::Invalid(
                "Forecast returned different quantiles than its advertised native levels".into(),
            ));
        }
        Ok(result)
    }
}
