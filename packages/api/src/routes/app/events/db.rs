//! Event database operations
//!
//! Provides functions to sync events between the bucket (source of truth for versions)
//! and the database (fast lookup mirror).
//!
//! For older events not yet in the database, fallback functions load from the bucket
//! and sync to the database for future fast lookups.

use crate::entity::event;
use flow_like::app::App;
use flow_like::flow::event::{CanaryEvent, Event as CoreEvent, EventInput, ReleaseNotes};
use flow_like_types::anyhow;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder,
};
use serde_json::json;

/// Filter out secret variable values from an event.
/// Secret variables will have their `default_value` set to `None`.
/// This should be used when returning events to clients, as secrets
/// are only used server-side during execution.
pub fn filter_event_secrets(mut event: CoreEvent) -> CoreEvent {
    // Filter secret variables in main variables
    for var in event.variables.values_mut() {
        if var.secret {
            var.default_value = None;
        }
    }

    // Filter secret variables in canary if present
    if let Some(ref mut canary) = event.canary {
        for var in canary.variables.values_mut() {
            if var.secret {
                var.default_value = None;
            }
        }
    }

    event
}

/// Convert a core Event to database Event model
pub fn event_to_db_model(app_id: &str, event: &CoreEvent) -> event::ActiveModel {
    let board_version = event
        .board_version
        .map(|(major, minor, patch)| format!("{}.{}.{}", major, minor, patch));

    let event_version = format!(
        "{}.{}.{}",
        event.event_version.0, event.event_version.1, event.event_version.2
    );

    let variables = if event.variables.is_empty() {
        None
    } else {
        serde_json::to_value(&event.variables).ok()
    };

    let config = if event.config.is_empty() {
        None
    } else {
        // Store config as base64 in JSON
        Some(
            json!({ "base64": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &event.config) }),
        )
    };

    let inputs = if event.inputs.is_empty() {
        None
    } else {
        serde_json::to_value(&event.inputs).ok()
    };

    let notes = event
        .notes
        .as_ref()
        .and_then(|n| serde_json::to_value(n).ok());
    let canary = event
        .canary
        .as_ref()
        .and_then(|c| serde_json::to_value(c).ok());

    event::ActiveModel {
        id: Set(event.id.clone()),
        app_id: Set(app_id.to_string()),
        name: Set(event.name.clone()),
        description: Set(if event.description.is_empty() {
            None
        } else {
            Some(event.description.clone())
        }),
        event_type: Set(event.event_type.clone()),
        active: Set(event.active),
        priority: Set(event.priority as i32),
        board_id: Set(Some(event.board_id.clone())),
        board_version: Set(board_version),
        node_id: Set(Some(event.node_id.clone())),
        page_id: Set(event.default_page_id.clone()),
        route: Set(event.route.clone()),
        is_default: Set(event.is_default),
        event_version: Set(event_version),
        variables: Set(variables),
        config: Set(config),
        inputs: Set(inputs),
        notes: Set(notes),
        canary: Set(canary),
        created_at: Set(chrono::DateTime::from_timestamp(
            event
                .created_at
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            0,
        )
        .unwrap_or_default()
        .naive_utc()),
        updated_at: Set(chrono::Utc::now().naive_utc()),
    }
}

/// Convert a database Event model to core Event
pub fn db_model_to_event(model: event::Model) -> flow_like_types::Result<CoreEvent> {
    let board_version = model.board_version.and_then(|v| {
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() == 3 {
            Some((
                parts[0].parse().ok()?,
                parts[1].parse().ok()?,
                parts[2].parse().ok()?,
            ))
        } else {
            None
        }
    });

    let event_version = {
        let parts: Vec<&str> = model.event_version.split('.').collect();
        if parts.len() == 3 {
            (
                parts[0].parse().unwrap_or(0),
                parts[1].parse().unwrap_or(0),
                parts[2].parse().unwrap_or(0),
            )
        } else {
            (0, 0, 0)
        }
    };

    let variables = model
        .variables
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    let config = model
        .config
        .and_then(|c| {
            if let Some(b64) = c.get("base64").and_then(|v| v.as_str()) {
                base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64).ok()
            } else {
                None
            }
        })
        .unwrap_or_default();

    let inputs: Vec<EventInput> = model
        .inputs
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    let notes: Option<ReleaseNotes> = model.notes.and_then(|v| serde_json::from_value(v).ok());

    let canary: Option<CanaryEvent> = model.canary.and_then(|v| serde_json::from_value(v).ok());

    let created_at = std::time::UNIX_EPOCH
        + std::time::Duration::from_secs(model.created_at.and_utc().timestamp() as u64);
    let updated_at = std::time::UNIX_EPOCH
        + std::time::Duration::from_secs(model.updated_at.and_utc().timestamp() as u64);

    Ok(CoreEvent {
        id: model.id,
        name: model.name,
        description: model.description.unwrap_or_default(),
        board_id: model.board_id.unwrap_or_default(),
        board_version,
        node_id: model.node_id.unwrap_or_default(),
        variables,
        config,
        active: model.active,
        canary,
        priority: model.priority as u32,
        event_type: model.event_type,
        notes,
        event_version,
        created_at,
        updated_at,
        default_page_id: model.page_id,
        inputs,
        route: model.route,
        is_default: model.is_default,
    })
}

/// Sync an event to the database (upsert)
pub async fn sync_event_to_db(
    db: &DatabaseConnection,
    app_id: &str,
    event: &CoreEvent,
) -> flow_like_types::Result<()> {
    let model = event_to_db_model(app_id, event);

    // Try to find existing
    let existing = event::Entity::find_by_id(&event.id).one(db).await?;

    if existing.is_some() {
        model.update(db).await?;
    } else {
        model.insert(db).await?;
    }

    Ok(())
}

/// Delete an event from the database
pub async fn delete_event_from_db(
    db: &DatabaseConnection,
    event_id: &str,
) -> flow_like_types::Result<()> {
    event::Entity::delete_by_id(event_id).exec(db).await?;
    Ok(())
}

/// Get an event from the database by ID
pub async fn get_event_from_db(
    db: &DatabaseConnection,
    event_id: &str,
) -> flow_like_types::Result<CoreEvent> {
    let model = event::Entity::find_by_id(event_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Event not found: {}", event_id))?;

    db_model_to_event(model)
}

/// Get an event from the database by ID, returning None if not found
pub async fn get_event_from_db_opt(
    db: &DatabaseConnection,
    event_id: &str,
) -> flow_like_types::Result<Option<CoreEvent>> {
    let model = event::Entity::find_by_id(event_id).one(db).await?;

    match model {
        Some(m) => Ok(Some(db_model_to_event(m)?)),
        None => Ok(None),
    }
}

/// Get all events for an app from the database
pub async fn get_events_for_app(
    db: &DatabaseConnection,
    app_id: &str,
) -> flow_like_types::Result<Vec<CoreEvent>> {
    let models = event::Entity::find()
        .filter(event::Column::AppId.eq(app_id))
        .order_by_desc(event::Column::Priority)
        .order_by_asc(event::Column::Name)
        .all(db)
        .await?;

    models.into_iter().map(db_model_to_event).collect()
}

/// Get all active events for an app from the database
pub async fn get_active_events_for_app(
    db: &DatabaseConnection,
    app_id: &str,
) -> flow_like_types::Result<Vec<CoreEvent>> {
    let models = event::Entity::find()
        .filter(event::Column::AppId.eq(app_id))
        .filter(event::Column::Active.eq(true))
        .order_by_desc(event::Column::Priority)
        .order_by_asc(event::Column::Name)
        .all(db)
        .await?;

    models.into_iter().map(db_model_to_event).collect()
}

/// Get events by type for an app
pub async fn get_events_by_type(
    db: &DatabaseConnection,
    app_id: &str,
    event_type: &str,
) -> flow_like_types::Result<Vec<CoreEvent>> {
    let models = event::Entity::find()
        .filter(event::Column::AppId.eq(app_id))
        .filter(event::Column::EventType.eq(event_type))
        .filter(event::Column::Active.eq(true))
        .order_by_desc(event::Column::Priority)
        .all(db)
        .await?;

    models.into_iter().map(db_model_to_event).collect()
}

/// Get the event that links to a specific board
pub async fn get_event_by_board(
    db: &DatabaseConnection,
    app_id: &str,
    board_id: &str,
) -> flow_like_types::Result<Option<CoreEvent>> {
    let model = event::Entity::find()
        .filter(event::Column::AppId.eq(app_id))
        .filter(event::Column::BoardId.eq(board_id))
        .one(db)
        .await?;

    match model {
        Some(m) => Ok(Some(db_model_to_event(m)?)),
        None => Ok(None),
    }
}

/// Get the event that links to a specific page (A2UI events)
pub async fn get_event_by_page(
    db: &DatabaseConnection,
    app_id: &str,
    page_id: &str,
) -> flow_like_types::Result<Option<CoreEvent>> {
    let model = event::Entity::find()
        .filter(event::Column::AppId.eq(app_id))
        .filter(event::Column::PageId.eq(page_id))
        .one(db)
        .await?;

    match model {
        Some(m) => Ok(Some(db_model_to_event(m)?)),
        None => Ok(None),
    }
}

/// Get the event for a specific route path
pub async fn get_event_by_route(
    db: &DatabaseConnection,
    app_id: &str,
    route: &str,
) -> flow_like_types::Result<Option<CoreEvent>> {
    let model = event::Entity::find()
        .filter(event::Column::AppId.eq(app_id))
        .filter(event::Column::Route.eq(route))
        .one(db)
        .await?;

    match model {
        Some(m) => Ok(Some(db_model_to_event(m)?)),
        None => Ok(None),
    }
}

/// Get the default event for an app (the one shown at "/" or when no route matches)
pub async fn get_default_event(
    db: &DatabaseConnection,
    app_id: &str,
) -> flow_like_types::Result<Option<CoreEvent>> {
    let model = event::Entity::find()
        .filter(event::Column::AppId.eq(app_id))
        .filter(event::Column::IsDefault.eq(true))
        .one(db)
        .await?;

    match model {
        Some(m) => Ok(Some(db_model_to_event(m)?)),
        None => Ok(None),
    }
}

// ============================================================================
// Fallback functions - load from bucket if not in DB, then sync to DB
// ============================================================================

/// Get an event by ID with fallback to bucket if not in DB
/// If found in bucket, syncs to DB for future fast lookups
pub async fn get_event_with_fallback(
    db: &DatabaseConnection,
    app: &App,
    event_id: &str,
) -> flow_like_types::Result<CoreEvent> {
    // Try DB first
    if let Some(event) = get_event_from_db_opt(db, event_id).await? {
        return Ok(event);
    }

    // Fallback to bucket
    let event = app.get_event(event_id, None).await?;

    // Sync to DB for future lookups
    if let Err(e) = sync_event_to_db(db, &app.id, &event).await {
        tracing::warn!("Failed to sync event {} to DB: {}", event_id, e);
    }

    Ok(event)
}

/// Get an event by ID with fallback, returning None if not found anywhere
pub async fn get_event_with_fallback_opt(
    db: &DatabaseConnection,
    app: &App,
    event_id: &str,
) -> flow_like_types::Result<Option<CoreEvent>> {
    // Try DB first
    if let Some(event) = get_event_from_db_opt(db, event_id).await? {
        return Ok(Some(event));
    }

    // Fallback to bucket
    match app.get_event(event_id, None).await {
        Ok(event) => {
            // Sync to DB for future lookups
            if let Err(e) = sync_event_to_db(db, &app.id, &event).await {
                tracing::warn!("Failed to sync event {} to DB: {}", event_id, e);
            }
            Ok(Some(event))
        }
        Err(_) => Ok(None),
    }
}

/// Get all events for an app with fallback to bucket
/// If bucket has events not in DB, syncs them
pub async fn get_events_with_fallback(
    db: &DatabaseConnection,
    app: &App,
) -> flow_like_types::Result<Vec<CoreEvent>> {
    // Try DB first
    let db_events = get_events_for_app(db, &app.id).await?;

    if !db_events.is_empty() {
        return Ok(db_events);
    }

    // DB is empty, load from bucket using event IDs and sync
    let mut bucket_events = Vec::new();
    for event_id in &app.events {
        match app.get_event(event_id, None).await {
            Ok(event) => {
                if let Err(e) = sync_event_to_db(db, &app.id, &event).await {
                    tracing::warn!("Failed to sync event {} to DB: {}", event.id, e);
                }
                bucket_events.push(event);
            }
            Err(e) => {
                tracing::warn!("Failed to load event {} from bucket: {}", event_id, e);
            }
        }
    }

    Ok(bucket_events)
}

/// Get event by route with fallback - searches bucket events if not in DB
pub async fn get_event_by_route_with_fallback(
    db: &DatabaseConnection,
    app: &App,
    route: &str,
) -> flow_like_types::Result<Option<CoreEvent>> {
    // Try DB first
    if let Some(event) = get_event_by_route(db, &app.id, route).await? {
        return Ok(Some(event));
    }

    // Fallback: load all events from bucket using IDs and find by route
    for event_id in &app.events {
        match app.get_event(event_id, None).await {
            Ok(event) => {
                // Sync to DB
                if let Err(e) = sync_event_to_db(db, &app.id, &event).await {
                    tracing::warn!("Failed to sync event {} to DB: {}", event.id, e);
                }

                if event.route.as_deref() == Some(route) {
                    return Ok(Some(event));
                }
            }
            Err(e) => {
                tracing::warn!("Failed to load event {} from bucket: {}", event_id, e);
            }
        }
    }

    Ok(None)
}

/// Get default event with fallback - searches bucket events if not in DB
pub async fn get_default_event_with_fallback(
    db: &DatabaseConnection,
    app: &App,
) -> flow_like_types::Result<Option<CoreEvent>> {
    // Try DB first
    if let Some(event) = get_default_event(db, &app.id).await? {
        return Ok(Some(event));
    }

    // Fallback: load all events from bucket using IDs and find default
    for event_id in &app.events {
        match app.get_event(event_id, None).await {
            Ok(event) => {
                // Sync to DB
                if let Err(e) = sync_event_to_db(db, &app.id, &event).await {
                    tracing::warn!("Failed to sync event {} to DB: {}", event.id, e);
                }

                if event.is_default {
                    return Ok(Some(event));
                }
            }
            Err(e) => {
                tracing::warn!("Failed to load event {} from bucket: {}", event_id, e);
            }
        }
    }

    Ok(None)
}
