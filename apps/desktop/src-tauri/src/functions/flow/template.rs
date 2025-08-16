use flow_like::{
    app::App,
    bit::Metadata,
    flow::board::{Board, VersionType},
};
use tauri::AppHandle;

use crate::{
    functions::TauriFunctionError,
    state::{TauriFlowLikeState, TauriSettingsState},
};

#[tauri::command(async)]
pub async fn get_template(
    handler: AppHandle,
    app_id: String,
    template_id: String,
    version: Option<(u32, u32, u32)>,
) -> Result<Board, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&handler).await?;

    if let Ok(app) = App::load(app_id, flow_like_state).await {
        let template = app.get_template(&template_id, version).await?;
        return Ok(template);
    }

    Err(TauriFunctionError::new("Event not found"))
}

#[tauri::command(async)]
pub async fn get_template_versions(
    handler: AppHandle,
    app_id: String,
    template_id: String,
) -> Result<Vec<(u32, u32, u32)>, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&handler).await?;

    if let Ok(app) = App::load(app_id, flow_like_state).await {
        let event = app.get_template_versions(&template_id).await?;
        return Ok(event);
    }

    Err(TauriFunctionError::new("Event not found"))
}

/// Fetches all the templates for a given app.
/// If no app is provided, return all templates, the user has access to.
#[tauri::command(async)]
pub async fn get_templates(
    handler: AppHandle,
    app_id: Option<String>,
    language: Option<String>,
) -> Result<Vec<(String, String, Option<Metadata>)>, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&handler).await?;

    if let Some(app_id) = app_id
        && let Ok(app) = App::load(app_id.clone(), flow_like_state.clone()).await
    {
        let templates = &app.templates;
        let mut loaded_templates = Vec::with_capacity(templates.len());

        println!(
            "Loading templates for app: {}, candidates: {}",
            app_id,
            templates.len()
        );

        for template in templates {
            let template = app.get_template(template, None).await;
            if let Ok(loaded_template) = template {
                let metadata = app
                    .get_template_meta(&loaded_template.id, language.clone())
                    .await
                    .ok();
                println!("Loaded template: {}", loaded_template.id);
                loaded_templates.push((app.id.clone(), loaded_template.id, metadata));
            } else {
                tracing::warn!("Failed to load template in app {}", app_id.clone());
            }
        }

        return Ok(loaded_templates);
    }

    let profile = TauriSettingsState::current_profile(&handler).await?;

    let mut loaded_templates = Vec::with_capacity(100);
    for app in profile.hub_profile.apps.unwrap_or_default().iter() {
        let app_id = app.app_id.clone();
        if let Ok(app) = App::load(app_id.clone(), flow_like_state.clone()).await {
            for template in &app.templates {
                let template = app.get_template(template, None).await;
                if let Ok(loaded_template) = template {
                    let metadata = app
                        .get_template_meta(&loaded_template.id, language.clone())
                        .await
                        .ok();
                    loaded_templates.push((app.id.clone(), loaded_template.id, metadata));
                } else {
                    tracing::warn!("Failed to load template in app {}", app_id.clone());
                }
            }
        }
    }

    Ok(loaded_templates)
}

#[tauri::command(async)]
pub async fn upsert_template(
    handler: AppHandle,
    app_id: String,
    board_id: String,
    template_id: Option<String>,
    board_version: Option<(u32, u32, u32)>,
    version_type: Option<VersionType>,
) -> Result<(String, (u32, u32, u32)), TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&handler).await?;
    let version_type = version_type.unwrap_or(VersionType::Patch);
    if let Ok(mut app) = App::load(app_id.clone(), flow_like_state).await {
        let template = app
            .upsert_template(template_id, version_type, board_id, board_version)
            .await?;
        return Ok(template);
    }

    Err(TauriFunctionError::new("Failed to upsert template"))
}

#[tauri::command(async)]
pub async fn push_template_data(
    handler: AppHandle,
    app_id: String,
    template_id: String,
    data: Board,
    version: Option<(u32, u32, u32)>,
) -> Result<(), TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&handler).await?;
    if let Ok(app) = App::load(app_id.clone(), flow_like_state).await {
        app.push_template_data(template_id, data, version).await?;
        return Ok(());
    }

    Err(TauriFunctionError::new("Failed to upsert template data"))
}

#[tauri::command(async)]
pub async fn delete_template(
    handler: AppHandle,
    app_id: String,
    template_id: String,
) -> Result<(), TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&handler).await?;

    if let Ok(mut app) = App::load(app_id.clone(), flow_like_state).await {
        app.delete_template(&template_id).await?;
        return Ok(());
    }

    Err(TauriFunctionError::new("Failed to delete event"))
}

#[tauri::command(async)]
pub async fn get_template_meta(
    handler: AppHandle,
    app_id: String,
    template_id: String,
    language: Option<String>,
) -> Result<Metadata, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&handler).await?;

    if let Ok(app) = App::load(app_id, flow_like_state).await {
        let metadata = app.get_template_meta(&template_id, language).await?;
        return Ok(metadata);
    }

    Err(TauriFunctionError::new("Failed to get template metadata"))
}

#[tauri::command(async)]
pub async fn push_template_meta(
    handler: AppHandle,
    app_id: String,
    template_id: String,
    metadata: Metadata,
    language: Option<String>,
) -> Result<(), TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&handler).await?;

    if let Ok(app) = App::load(app_id, flow_like_state).await {
        app.push_template_meta(&template_id, language, metadata)
            .await?;
        return Ok(());
    }

    Err(TauriFunctionError::new("Failed to get template metadata"))
}
