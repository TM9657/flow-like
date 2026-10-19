use flow_like::bit::{Bit, ForecastingAssetRef};
use flow_like::flow_like_storage::{
    contracts::column_metadata::ColumnReader,
    databases::workbench::{WorkbenchSurface, saved_query::SavedQuerySurface},
};
use flow_like_catalog_ml::inspection::forecasting::studio::{
    self, StudioForecastRequest, StudioForecastResult,
};
use serde::Deserialize;
use tauri::AppHandle;

use crate::{
    functions::TauriFunctionError,
    state::{TauriFlowLikeState, TauriSettingsState},
};

#[derive(Debug, Deserialize)]
pub struct ForecastQueryInput {
    pub profile_id: String,
    #[serde(flatten)]
    pub request: StudioForecastRequest,
}

fn activated_reference(
    references: &[String],
    model_id: &str,
    default_hub: &str,
) -> flow_like_types::Result<ForecastingAssetRef> {
    let matching = references
        .iter()
        .filter(|reference| {
            reference
                .rsplit_once(':')
                .map_or(reference.as_str(), |(_, id)| id)
                == model_id
        })
        .collect::<Vec<_>>();
    let first = matching.first().ok_or_else(|| {
        flow_like_types::anyhow!(
            "Add this forecasting model to the current profile before running it"
        )
    })?;
    let selected = ForecastingAssetRef {
        bit: (*first).clone(),
        relative_path: String::new(),
    };
    let (hub, id) = selected.reference_parts(default_hub)?;
    let identity = Bit {
        id: id.into(),
        hub: hub.into(),
        ..Default::default()
    };
    for reference in matching {
        let candidate = ForecastingAssetRef {
            bit: reference.clone(),
            relative_path: String::new(),
        };
        if !candidate.matches_reference(&identity, default_hub)? {
            return Err(flow_like_types::anyhow!(
                "This model ID is activated from more than one hub; remove the duplicate before forecasting"
            ));
        }
    }
    Ok(selected)
}

#[tauri::command(async)]
pub async fn query_forecast(
    app_handle: AppHandle,
    app_id: String,
    payload: ForecastQueryInput,
    user_scoped: Option<bool>,
) -> Result<StudioForecastResult, TauriFunctionError> {
    let connection =
        super::graph::graph_connection(&app_handle, &app_id, user_scoped.unwrap_or(false)).await?;
    let profile = TauriSettingsState::current_profile(&app_handle).await?;
    if profile.hub_profile.id != payload.profile_id {
        return Err(TauriFunctionError::new(
            "The active profile changed. Reload forecasting and try again.",
        ));
    }
    let reference = activated_reference(
        &profile.hub_profile.bits,
        &payload.request.model_bit_id,
        &profile.hub_profile.hub,
    )?;
    if !studio::inference_available() {
        return Err(TauriFunctionError::new(
            "Native forecasting is unavailable in this build. Use a desktop or server build with forecasting enabled.",
        ));
    }
    let state = TauriFlowLikeState::construct(&app_handle).await?;
    let (hub, id) = reference.reference_parts(&profile.hub_profile.hub)?;
    let bit = profile
        .hub_profile
        .get_bit(id.into(), Some(hub.into()), state.http_client.clone())
        .await?;
    if !reference.matches_reference(&bit, &profile.hub_profile.hub)? {
        return Err(TauriFunctionError::new(
            "The selected forecasting Bit resolved from a different hub",
        ));
    }
    let views =
        super::saved_queries::collect_views(&connection, SavedQuerySurface::Native, None).await?;
    Ok(studio::run_forecast(
        state,
        bit,
        &connection,
        WorkbenchSurface::Native,
        views,
        &ColumnReader::Unrestricted,
        payload.request,
    )
    .await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forecast_model_resolution_requires_activation_and_unambiguous_hub_identity() {
        assert!(
            activated_reference(&["hub.example:other".into()], "model", "hub.example").is_err()
        );
        assert!(
            activated_reference(
                &["hub.example:model".into(), "other.example:model".into()],
                "model",
                "hub.example"
            )
            .is_err()
        );
        let reference = activated_reference(
            &[
                "hub.example:model".into(),
                "https://hub.example:model".into(),
            ],
            "model",
            "hub.example",
        )
        .unwrap();
        assert_eq!(reference.reference_parts("hub.example").unwrap().1, "model");
        assert!(
            activated_reference(&["hub.example:../model".into()], "../model", "hub.example")
                .is_err()
        );
    }
}
