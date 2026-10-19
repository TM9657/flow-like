use super::*;
use crate::inspection::{
    auto_training_tables::TableBudget,
    forecasting::{
        studio::{
            StudioBacktestMetric, StudioBacktestResult, StudioForecastModel, StudioForecastPoint,
            StudioHistoryPoint, input_error_message,
        },
        tables::ForecastTableMapping,
    },
};
use flow_like_ml_forecast::{ForecastCapabilities, ForecastFamily, PointStatistic};
use flow_like_storage::{
    arrow_array::{ArrayRef, Float32Array, Int64Array, RecordBatch},
    arrow_schema::{DataType, Field, Schema},
    databases::{
        vector::schema::column_metadata_update,
        workbench::{
            WorkbenchSurface, WorkbenchView,
            saved_query::{
                SavedQueryDef, SavedQueryKind, SavedQuerySaveResult, SavedQuerySurface,
                save_saved_query, save_saved_query_if_unchanged,
            },
            source_access,
        },
    },
    lancedb::{self, Connection, Table},
};
use flow_like_storage_contracts::column_metadata::{ColumnMetadata, ColumnReader};
use flow_like_types::{Result, json::json};
use std::{path::PathBuf, sync::Arc};

struct TemporaryDatabase(PathBuf);

impl TemporaryDatabase {
    async fn open() -> Result<(Self, Connection)> {
        let temporary = Self(std::env::temp_dir().join(format!(
            "flow-like-saved-forecasts-{}",
            flow_like_types::create_id()
        )));
        let connection = lancedb::connect(temporary.0.to_string_lossy().as_ref())
            .execute()
            .await?;
        Ok((temporary, connection))
    }
}

impl Drop for TemporaryDatabase {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn point(timestamp_ms: i64, actual: Option<f32>) -> StudioForecastPoint {
    StudioForecastPoint {
        series_id: "series".into(),
        timestamp_ms,
        target: "target".into(),
        point: 2.0,
        quantiles: vec![1.0, 2.0, 3.0],
        actual,
    }
}

fn payload(name: &str, source: &str) -> CreateStudioForecast {
    CreateStudioForecast {
        name: name.into(),
        description: Some("A saved comparison".into()),
        request: StudioForecastRequest {
            model_bit_id: "forecast-model".into(),
            sql: format!("SELECT time, target FROM {source}"),
            params: json!({}),
            mapping: ForecastTableMapping {
                timestamp: "time".into(),
                timestamp_format: None,
                series_id: None,
                targets: vec!["target".into()],
                past_covariates: vec![],
                known_covariates: vec![],
                frequency_ms: None,
            },
            horizon: 2,
            quantile_levels: vec![0.1, 0.5, 0.9],
            future_sql: None,
            future_params: json!({}),
            backtest_horizon: 2,
            backtest_offset: 0,
            budget: TableBudget::default(),
        },
        result: StudioForecastResult {
            run_id: "completed-run".into(),
            model: StudioForecastModel {
                bit_id: "forecast-model".into(),
                family: ForecastFamily::Chronos2,
                repository: Some("amazon/chronos-2".into()),
                revision: Some("saved-revision".into()),
            },
            capabilities: ForecastCapabilities {
                family: ForecastFamily::Chronos2,
                max_context: 4,
                max_horizon: Some(16),
                native_horizon: 16,
                quantile_levels: vec![0.1, 0.5, 0.9],
                joint_multivariate: true,
                past_covariates: true,
                known_covariates: true,
            },
            input_rows: 4,
            horizon: 2,
            quantile_levels: vec![0.1, 0.5, 0.9],
            point_statistic: PointStatistic::Median,
            history: [
                (10, Some(1.0)),
                (20, Some(2.0)),
                (30, Some(3.0)),
                (40, None),
            ]
            .into_iter()
            .map(|(timestamp_ms, value)| StudioHistoryPoint {
                series_id: "series".into(),
                timestamp_ms,
                target: "target".into(),
                value,
            })
            .collect(),
            forecasts: vec![point(50, None), point(60, None)],
            covariates: vec![],
            backtest: Some(StudioBacktestResult {
                horizon: 2,
                offset: 0,
                forecasts: vec![point(30, Some(3.0)), point(40, None)],
                metrics: vec![StudioBacktestMetric {
                    series_id: "series".into(),
                    target: "target".into(),
                    observations: 1,
                    mae: Some(1.0),
                    rmse: Some(1.0),
                    mape: Some(33.33333333333333),
                    wape: Some(100.0 / 3.0),
                }],
            }),
        },
    }
}

fn historical_payload() -> CreateStudioForecast {
    let mut snapshot = payload("Earlier evaluation", "history");
    snapshot.request.backtest_offset = 2;
    snapshot.result.input_rows = 8;
    snapshot.result.capabilities.max_context = 8;
    snapshot.result.history = [
        Some(1.0),
        Some(2.0),
        Some(3.0),
        Some(4.0),
        Some(5.0),
        None,
        Some(700.0),
        Some(800.0),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, value)| StudioHistoryPoint {
        series_id: "series".into(),
        timestamp_ms: (index as i64 + 1) * 10,
        target: "target".into(),
        value,
    })
    .collect();
    snapshot.result.forecasts = vec![point(90, None), point(100, None)];
    snapshot.result.backtest = Some(StudioBacktestResult {
        horizon: 2,
        offset: 2,
        forecasts: vec![point(50, Some(5.0)), point(60, None)],
        metrics: vec![StudioBacktestMetric {
            series_id: "series".into(),
            target: "target".into(),
            observations: 1,
            mae: Some(3.0),
            rmse: Some(3.0),
            mape: Some(60.0),
            wape: Some(60.0),
        }],
    });
    snapshot
}

async fn restrict(table: &Table, column: &str, attributes: &[&str]) -> Result<()> {
    table
        .update_field_metadata(&[column_metadata_update(
            column,
            &ColumnMetadata {
                read_attributes: attributes.iter().map(|value| (*value).into()).collect(),
                ..Default::default()
            },
        )])
        .await?;
    Ok(())
}

async fn source(connection: &Connection, name: &str, attributes: &[&str]) -> Result<Table> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("time", DataType::Int64, false),
        Field::new("target", DataType::Float32, true),
        Field::new("secret", DataType::Float32, false),
    ]));
    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(Int64Array::from(vec![10, 20, 30, 40])) as ArrayRef,
            Arc::new(Float32Array::from(vec![
                Some(1.0),
                Some(2.0),
                Some(3.0),
                None,
            ])),
            Arc::new(Float32Array::from(vec![1.0; 4])),
        ],
    )?;
    let table = connection.create_table(name, vec![batch]).execute().await?;
    restrict(&table, "target", attributes).await?;
    restrict(&table, "secret", &["finance"]).await?;
    Ok(table)
}

#[test]
fn snapshot_validation_preserves_nulls_and_rejects_inconsistent_outputs() {
    let original = payload("Forecast", "history");
    validation::payload(&original).unwrap();
    let serialized = serde_json::to_value(&original).unwrap();
    let decoded: CreateStudioForecast = serde_json::from_value(serialized.clone()).unwrap();
    validation::payload(&decoded).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), serialized);
    let mut crossed = original.clone();
    crossed.result.forecasts[0].quantiles = vec![3.0, 2.0, 1.0];
    validation::payload(&crossed).unwrap();

    let mutations: &[(&str, fn(&mut CreateStudioForecast))] = &[
        ("missing name", |p| p.name.clear()),
        ("different model", |p| {
            p.result.model.bit_id = "another-model".into()
        }),
        ("different horizon", |p| p.result.horizon = 1),
        ("duplicate history", |p| {
            p.result.history.push(p.result.history[0].clone())
        }),
        ("irregular history", |p| {
            p.result.history[1].timestamp_ms = 21
        }),
        ("missing prediction", |p| {
            p.result.forecasts.pop();
        }),
        ("wrong target", |p| {
            p.result.forecasts[0].target = "other".into()
        }),
        ("missing quantile", |p| {
            p.result.forecasts[0].quantiles.pop();
        }),
        ("nonfinite quantile", |p| {
            p.result.forecasts[0].quantiles[0] = f32::INFINITY
        }),
        ("nonfinite value", |p| {
            p.result.forecasts[0].point = f32::NAN
        }),
        ("unexpected future actual", |p| {
            p.result.forecasts[0].actual = Some(2.0)
        }),
        ("changed holdout actual", |p| {
            p.result.backtest.as_mut().unwrap().forecasts[0].actual = Some(4.0)
        }),
        ("fabricated metric", |p| {
            p.result.backtest.as_mut().unwrap().metrics[0].mae = Some(0.0)
        }),
        ("missing holdout", |p| p.result.backtest = None),
        ("wrong input count", |p| p.result.input_rows = 3),
    ];
    for (name, mutate) in mutations {
        let mut invalid = original.clone();
        mutate(&mut invalid);
        assert!(validation::payload(&invalid).is_err(), "accepted {name}");
    }
}

#[test]
fn point_only_snapshots_preserve_forecasts_and_backtest_metrics_without_bands() {
    let mut snapshot = payload("Point forecast", "history");
    snapshot.request.quantile_levels.clear();
    snapshot.result.quantile_levels.clear();
    snapshot.result.capabilities.quantile_levels.clear();
    snapshot.result.capabilities.family = ForecastFamily::TinyTimeMixer;
    snapshot.result.model.family = ForecastFamily::TinyTimeMixer;
    snapshot.result.model.repository = Some("ibm-granite/granite-timeseries-ttm-r2".into());
    snapshot.result.point_statistic = PointStatistic::Mean;
    for point in &mut snapshot.result.forecasts {
        point.quantiles.clear();
    }
    for point in &mut snapshot.result.backtest.as_mut().unwrap().forecasts {
        point.quantiles.clear();
    }
    validation::payload(&snapshot).unwrap();
    let round_trip: CreateStudioForecast =
        serde_json::from_slice(&serde_json::to_vec(&snapshot).unwrap()).unwrap();
    validation::payload(&round_trip).unwrap();
    assert_eq!(
        round_trip.result.backtest.unwrap().metrics[0].mae,
        Some(1.0)
    );

    snapshot.result.forecasts[0].quantiles.push(2.0);
    assert!(validation::payload(&snapshot).is_err());
    snapshot.result.forecasts[0].quantiles.clear();
    snapshot.request.quantile_levels.push(0.5);
    assert!(validation::payload(&snapshot).is_err());
}

#[test]
fn snapshot_metrics_allow_json_rounding_but_reject_changed_values_or_identity() {
    let original = payload("Forecast", "history");
    let mut rounded = original.clone();
    let metric = &mut rounded.result.backtest.as_mut().unwrap().metrics[0];
    for value in [
        &mut metric.mae,
        &mut metric.rmse,
        &mut metric.mape,
        &mut metric.wape,
    ] {
        let previous = value.unwrap();
        *value = Some(f64::from_bits(previous.to_bits() + 1));
    }
    let rounded: CreateStudioForecast =
        serde_json::from_str(&serde_json::to_string(&rounded).unwrap()).unwrap();
    validation::payload(&rounded).unwrap();
    assert_eq!(
        serde_json::to_value(&rounded.result.backtest.as_ref().unwrap().forecasts).unwrap(),
        serde_json::to_value(&original.result.backtest.as_ref().unwrap().forecasts).unwrap()
    );

    let mutations: &[(&str, fn(&mut StudioBacktestMetric))] = &[
        ("changed metric", |m| m.mae = Some(m.mae.unwrap() + 1e-10)),
        ("nonfinite mae", |m| m.mae = Some(f64::NAN)),
        ("nonfinite rmse", |m| m.rmse = Some(f64::INFINITY)),
        ("nonfinite mape", |m| m.mape = Some(f64::NEG_INFINITY)),
        ("nonfinite wape", |m| m.wape = Some(f64::NAN)),
        ("changed wape", |m| m.wape = Some(1.0)),
        ("missing metric", |m| m.mape = None),
        ("different series", |m| {
            m.series_id = "another-series".into()
        }),
        ("different target", |m| m.target = "another-target".into()),
        ("different observation count", |m| m.observations += 1),
    ];
    for (name, mutate) in mutations {
        let mut invalid = original.clone();
        mutate(&mut invalid.result.backtest.as_mut().unwrap().metrics[0]);
        assert!(validation::payload(&invalid).is_err(), "accepted {name}");
    }
    let mut duplicated = original;
    let metrics = &mut duplicated.result.backtest.as_mut().unwrap().metrics;
    metrics.push(metrics[0].clone());
    assert!(validation::payload(&duplicated).is_err());
}

#[test]
fn saved_holdout_must_fit_model_capabilities_even_when_the_outlook_fits() {
    let mut snapshot = payload("Short outlook", "history");
    snapshot.request.horizon = 1;
    snapshot.result.horizon = 1;
    snapshot.result.forecasts.truncate(1);
    snapshot.result.capabilities.max_horizon = Some(2);
    validation::payload(&snapshot).unwrap();

    snapshot.result.capabilities.max_horizon = Some(1);
    let error = validation::payload(&snapshot).unwrap_err();
    assert!(error.to_string().contains("maximum horizon"));

    snapshot.request.backtest_horizon = 0;
    snapshot.result.backtest = None;
    validation::payload(&snapshot).unwrap();
}

#[test]
fn historical_snapshot_validation_uses_only_the_selected_holdout_window() {
    let original = historical_payload();
    validation::payload(&original).unwrap();
    let mutations: &[(&str, fn(&mut CreateStudioForecast))] = &[
        ("different result offset", |p| {
            p.result.backtest.as_mut().unwrap().offset = 0
        }),
        ("different request offset", |p| {
            p.request.backtest_offset = 1
        }),
        ("trailing holdout timestamp", |p| {
            p.result.backtest.as_mut().unwrap().forecasts[0].timestamp_ms = 70
        }),
        ("trailing holdout actual", |p| {
            p.result.backtest.as_mut().unwrap().forecasts[0].actual = Some(700.0)
        }),
        ("fabricated missing observation", |p| {
            p.result.backtest.as_mut().unwrap().forecasts[1].actual = Some(0.0)
        }),
        ("offset without holdout", |p| {
            p.request.backtest_horizon = 0;
            p.result.backtest = None;
        }),
        ("offset outside history", |p| p.request.backtest_offset = 5),
    ];
    for (name, mutate) in mutations {
        let mut invalid = original.clone();
        mutate(&mut invalid);
        assert!(validation::payload(&invalid).is_err(), "accepted {name}");
    }
}

#[test]
fn missing_legacy_wape_is_allowed_but_undefined_wape_cannot_be_fabricated() {
    let mut legacy = payload("Legacy metrics", "history");
    legacy.result.backtest.as_mut().unwrap().metrics[0].wape = None;
    validation::payload(&legacy).unwrap();
    legacy.result.history[2].value = Some(0.0);
    let held = legacy.result.backtest.as_mut().unwrap();
    held.forecasts[0].actual = Some(0.0);
    held.metrics[0].mae = Some(2.0);
    held.metrics[0].rmse = Some(2.0);
    held.metrics[0].mape = None;
    validation::payload(&legacy).unwrap();
    legacy.result.backtest.as_mut().unwrap().metrics[0].wape = Some(0.0);
    assert!(validation::payload(&legacy).is_err());
}

#[tokio::test]
async fn historical_offsets_and_legacy_snapshot_definitions_roundtrip() -> Result<()> {
    let (_directory, connection) = TemporaryDatabase::open().await?;
    source(&connection, "history", &[]).await?;
    let reader = ColumnReader::Unrestricted;
    let historical = create(&connection, "app", "alice", &reader, historical_payload()).await?;
    let restored = get(&connection, "app", &historical.id, &reader)
        .await?
        .unwrap();
    assert_eq!(restored.request.backtest_offset, 2);
    assert_eq!(restored.result.backtest.as_ref().unwrap().offset, 2);
    assert_eq!(
        serde_json::to_value(&restored)?,
        serde_json::to_value(&historical)?
    );

    let legacy = create(
        &connection,
        "app",
        "alice",
        &reader,
        payload("Legacy prediction", "history"),
    )
    .await?;
    let mut definition = serde_json::to_value(&legacy)?;
    definition["request"]
        .as_object_mut()
        .unwrap()
        .remove("backtest_offset");
    let held = definition["result"]["backtest"].as_object_mut().unwrap();
    held.remove("offset");
    for metric in held["metrics"].as_array_mut().unwrap() {
        metric.as_object_mut().unwrap().remove("wape");
    }
    let encoded = serde_json::to_string(&definition)?.replace('\'', "''");
    connection
        .open_table(SAVED_FORECASTS_TABLE)
        .execute()
        .await?
        .update()
        .only_if(format!("id = '{}'", legacy.id))
        .column("definition_json", format!("'{encoded}'"))
        .execute()
        .await?;
    let restored = get(&connection, "app", &legacy.id, &reader).await?.unwrap();
    assert_eq!(restored.request.backtest_offset, 0);
    let restored_holdout = restored.result.backtest.as_ref().unwrap();
    assert_eq!(restored_holdout.offset, 0);
    assert_eq!(restored_holdout.metrics[0].wape, None);
    assert_eq!(
        serde_json::to_value(&restored_holdout.forecasts)?,
        serde_json::to_value(&legacy.result.backtest.as_ref().unwrap().forecasts)?
    );
    create(
        &connection,
        "app",
        "alice",
        &reader,
        CreateStudioForecast {
            name: "Resaved legacy prediction".into(),
            description: None,
            request: restored.request,
            result: restored.result,
        },
    )
    .await?;
    assert_eq!(list(&connection, "app", &reader, 0, 100).await?.len(), 3);
    Ok(())
}

#[tokio::test]
async fn snapshots_are_immutable_and_isolated_by_app_and_scoped_connection() -> Result<()> {
    let (_project_dir, project) = TemporaryDatabase::open().await?;
    let (_personal_dir, personal) = TemporaryDatabase::open().await?;
    source(&project, "history", &[]).await?;
    source(&personal, "history", &[]).await?;
    let reader = ColumnReader::with_attributes(["viewer"]);
    let original = payload("  First prediction  ", "history");
    let first = create(&project, "app-a", "alice", &reader, original.clone()).await?;
    let mut rerun = payload("Second prediction", "history");
    rerun.result.run_id = "another-run".into();
    let second = create(&project, "app-a", "bob", &reader, rerun).await?;
    assert_ne!(first.id, second.id);
    assert_eq!(first.name, "First prediction");
    assert_eq!(
        (first.app_id.as_str(), first.created_by.as_str()),
        ("app-a", "alice")
    );
    let restored = get(&project, "app-a", &first.id, &reader).await?.unwrap();
    assert_eq!(
        serde_json::to_value(&restored.result)?,
        serde_json::to_value(&original.result)?
    );
    assert_eq!(
        serde_json::to_value(&restored.request)?,
        serde_json::to_value(&original.request)?
    );
    assert_eq!(restored.result.history[3].value, None);
    let summaries = list(&project, "app-a", &reader, 0, 100).await?;
    assert_eq!(summaries.len(), 2);
    assert_eq!(summaries[0].id, second.id);
    assert!(get(&project, "app-b", &first.id, &reader).await?.is_none());
    assert!(list(&project, "app-b", &reader, 0, 100).await?.is_empty());
    assert!(get(&personal, "app-a", &first.id, &reader).await?.is_none());
    assert!(!delete(&project, "app-b", &first.id, &reader).await?);
    assert!(delete(&project, "app-a", &first.id, &reader).await?);
    assert!(!delete(&project, "app-a", &first.id, &reader).await?);
    assert!(get(&project, "app-a", &second.id, &reader).await?.is_some());
    Ok(())
}

#[tokio::test]
async fn snapshot_access_requires_both_captured_and_current_source_policies() -> Result<()> {
    let (_directory, connection) = TemporaryDatabase::open().await?;
    let table = source(&connection, "history", &["finance"]).await?;
    let finance = ColumnReader::with_attributes(["finance"]);
    let viewer = ColumnReader::with_attributes(["viewer"]);
    assert!(
        create(
            &connection,
            "app",
            "viewer",
            &viewer,
            payload("Denied", "history")
        )
        .await
        .is_err()
    );
    let restricted = create(
        &connection,
        "app",
        "alice",
        &finance,
        payload("Restricted", "history"),
    )
    .await?;

    restrict(&table, "target", &[]).await?;
    assert!(
        get(&connection, "app", &restricted.id, &viewer)
            .await?
            .is_none()
    );
    assert!(
        get(&connection, "app", &restricted.id, &finance)
            .await?
            .is_some()
    );
    let public = create(
        &connection,
        "app",
        "alice",
        &viewer,
        payload("Public", "history"),
    )
    .await?;

    restrict(&table, "target", &["audit"]).await?;
    let audit = ColumnReader::with_attributes(["audit"]);
    assert!(
        get(&connection, "app", &public.id, &viewer)
            .await?
            .is_none()
    );
    assert!(get(&connection, "app", &public.id, &audit).await?.is_some());
    assert!(
        get(&connection, "app", &restricted.id, &audit)
            .await?
            .is_none()
    );
    assert!(
        get(&connection, "app", &restricted.id, &finance)
            .await?
            .is_none()
    );
    assert!(!delete(&connection, "app", &restricted.id, &finance).await?);
    let both = ColumnReader::with_attributes(["audit", "finance"]);
    assert!(
        get(&connection, "app", &restricted.id, &both)
            .await?
            .is_some()
    );

    connection.drop_table("history", &[]).await?;
    assert!(
        get(&connection, "app", &public.id, &ColumnReader::Unrestricted)
            .await?
            .is_none()
    );
    assert!(list(&connection, "app", &both, 0, 100).await?.is_empty());
    assert!(!delete(&connection, "app", &public.id, &both).await?);
    assert!(delete(&connection, "app", &public.id, &ColumnReader::Unrestricted).await?);
    Ok(())
}

#[tokio::test]
async fn pagination_counts_only_authorized_snapshots() -> Result<()> {
    let (_directory, connection) = TemporaryDatabase::open().await?;
    source(&connection, "history", &[]).await?;
    source(&connection, "restricted", &["finance"]).await?;
    let reader = ColumnReader::with_attributes(["viewer"]);
    let first = create(
        &connection,
        "app",
        "alice",
        &reader,
        payload("First", "history"),
    )
    .await?;
    let hidden = create(
        &connection,
        "app",
        "alice",
        &ColumnReader::Unrestricted,
        payload("Hidden", "restricted"),
    )
    .await?;
    let last = create(
        &connection,
        "app",
        "alice",
        &reader,
        payload("Last", "history"),
    )
    .await?;
    let page = list(&connection, "app", &reader, 0, 1).await?;
    assert_eq!(page[0].id, last.id);
    let page = list(&connection, "app", &reader, 1, 1).await?;
    assert_eq!(page[0].id, first.id);
    assert!(list(&connection, "app", &reader, 2, 1).await?.is_empty());
    assert!(
        get(&connection, "app", &hidden.id, &reader)
            .await?
            .is_none()
    );
    assert!(!delete(&connection, "app", &hidden.id, &reader).await?);
    for (offset, limit) in [(0, 0), (0, 101), (1001, 1)] {
        let error = list(&connection, "app", &reader, offset, limit)
            .await
            .unwrap_err();
        assert!(input_error_message(&error).is_some());
    }
    Ok(())
}

#[tokio::test]
async fn saved_view_filters_and_join_columns_retain_their_access_requirements() -> Result<()> {
    let (_directory, connection) = TemporaryDatabase::open().await?;
    source(&connection, "history", &[]).await?;
    let filters = source(&connection, "filters", &[]).await?;
    let mut view = SavedQueryDef {
        id: "source-view".into(),
        app_id: "app".into(),
        name: "selected_history".into(),
        description: None,
        kind: SavedQueryKind::View,
        surface: SavedQuerySurface::Native,
        overlay_id: None,
        sql: "SELECT h.time, h.target FROM history h JOIN filters f ON h.time = f.time WHERE f.secret > 0".into(),
        param_schema: None,
        viz_config: None,
        default_limit: None,
        created_at: "2026-01-01T00:00:00Z".into(),
        updated_at: "2026-01-01T00:00:00Z".into(),
    };
    assert_eq!(
        save_saved_query(&connection, &view).await?,
        SavedQuerySaveResult::Saved
    );
    let viewer = ColumnReader::with_attributes(["viewer"]);
    let finance = ColumnReader::with_attributes(["finance"]);
    assert!(
        create(
            &connection,
            "app",
            "viewer",
            &viewer,
            payload("Denied", &view.name)
        )
        .await
        .is_err()
    );
    let saved = create(
        &connection,
        "app",
        "alice",
        &finance,
        payload("Joined", &view.name),
    )
    .await?;
    assert!(get(&connection, "app", &saved.id, &viewer).await?.is_none());
    view.sql = "SELECT time, target FROM history".into();
    let previous_revision = view.updated_at.clone();
    view.updated_at = "2026-01-02T00:00:00Z".into();
    assert_eq!(
        save_saved_query_if_unchanged(&connection, &view, &previous_revision).await?,
        SavedQuerySaveResult::Saved
    );
    assert!(get(&connection, "app", &saved.id, &viewer).await?.is_none());
    restrict(&filters, "secret", &["audit"]).await?;
    assert!(
        get(&connection, "app", &saved.id, &finance)
            .await?
            .is_none()
    );
    let both = ColumnReader::with_attributes(["audit", "finance"]);
    assert!(get(&connection, "app", &saved.id, &both).await?.is_some());
    Ok(())
}

#[tokio::test]
async fn source_requirements_keep_sensitive_expressions_and_prune_unused_view_columns() -> Result<()>
{
    let (_directory, connection) = TemporaryDatabase::open().await?;
    source(&connection, "history", &[]).await?;
    source(&connection, "filters", &[]).await?;
    let views = [WorkbenchView {
        name: "all_history".into(),
        sql: "SELECT * FROM history".into(),
    }];
    let projected = source_access::capture(
        &connection,
        &WorkbenchSurface::Native,
        &views,
        &ColumnReader::Unrestricted,
        "SELECT time, target FROM all_history",
        &json!({}),
    )
    .await?;
    assert!(
        projected
            .iter()
            .any(|r| r.column.as_deref() == Some("target"))
    );
    assert!(
        projected
            .iter()
            .all(|r| r.column.as_deref() != Some("secret"))
    );
    assert!(
        source_access::allows(
            &connection,
            &ColumnReader::with_attributes(["viewer"]),
            &projected,
        )
        .await?
    );

    for (sql, table) in [
        (
            "SELECT time, target FROM history WHERE secret > 0",
            "history",
        ),
        ("SELECT MIN(secret) AS target FROM history", "history"),
        ("SELECT COUNT(secret) AS target FROM history", "history"),
        (
            "SELECT h.time, h.target FROM history h JOIN filters f ON h.time = f.secret",
            "filters",
        ),
        (
            "SELECT time, target FROM history WHERE time IN (SELECT time FROM filters WHERE secret > 0)",
            "filters",
        ),
    ] {
        let requirements = source_access::capture(
            &connection,
            &WorkbenchSurface::Native,
            &[],
            &ColumnReader::Unrestricted,
            sql,
            &json!({}),
        )
        .await?;
        assert!(
            requirements.iter().any(|r| r.table == table
                && r.column.as_deref() == Some("secret")
                && r.read_attributes == ["finance"]),
            "missing sensitive dependency for {sql}: {requirements:?}"
        );
        assert!(
            !source_access::allows(
                &connection,
                &ColumnReader::with_attributes(["viewer"]),
                &requirements,
            )
            .await?
        );
    }
    Ok(())
}

#[tokio::test]
async fn summaries_do_not_decode_full_snapshot_blobs_and_ignore_invalid_policy() -> Result<()> {
    let (_directory, connection) = TemporaryDatabase::open().await?;
    source(&connection, "history", &[]).await?;
    let reader = ColumnReader::with_attributes(["viewer"]);
    let saved = create(
        &connection,
        "app",
        "alice",
        &reader,
        payload("Snapshot", "history"),
    )
    .await?;
    let table = connection
        .open_table(SAVED_FORECASTS_TABLE)
        .execute()
        .await?;
    table
        .update()
        .column("definition_json", "'invalid snapshot JSON'")
        .execute()
        .await?;
    let summaries = list(&connection, "app", &reader, 0, 100).await?;
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].id, saved.id);
    let summary = serde_json::to_value(&summaries[0])?;
    assert!(summary.get("request").is_none());
    assert!(summary.get("result").is_none());
    assert!(get(&connection, "app", &saved.id, &reader).await.is_err());
    table
        .update()
        .column("source_access_json", "'invalid policy JSON'")
        .execute()
        .await?;
    assert!(list(&connection, "app", &reader, 0, 100).await?.is_empty());
    assert!(get(&connection, "app", &saved.id, &reader).await?.is_none());
    assert!(!delete(&connection, "app", &saved.id, &reader).await?);
    assert!(delete(&connection, "app", &saved.id, &ColumnReader::Unrestricted).await?);
    Ok(())
}

#[tokio::test]
async fn invalid_snapshot_is_rejected_before_creating_storage() -> Result<()> {
    let (_directory, connection) = TemporaryDatabase::open().await?;
    let mut invalid = payload("Invalid", "missing_source");
    invalid.result.forecasts.clear();
    let error = create(
        &connection,
        "app",
        "alice",
        &ColumnReader::Unrestricted,
        invalid,
    )
    .await
    .unwrap_err();
    assert!(input_error_message(&error).is_some());
    assert!(connection.table_names().execute().await?.is_empty());
    Ok(())
}

#[test]
fn saved_results_keep_normalized_timestamps_when_source_uses_an_explicit_format() {
    use crate::inspection::forecasting::tables::ForecastTimestampFormat;
    for format in [
        ForecastTimestampFormat::Auto,
        ForecastTimestampFormat::UnixSeconds,
        ForecastTimestampFormat::DayFirst,
    ] {
        let mut snapshot = payload("Formatted dates", "history");
        snapshot.request.mapping.timestamp_format = Some(format);
        validation::payload(&snapshot).unwrap();
        let decoded: CreateStudioForecast =
            serde_json::from_value(serde_json::to_value(&snapshot).unwrap()).unwrap();
        assert_eq!(decoded.request.mapping.timestamp_format, Some(format));
        validation::payload(&decoded).unwrap();
    }
}
