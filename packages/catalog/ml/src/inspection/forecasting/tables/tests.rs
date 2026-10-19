use super::{preparation::*, *};
use crate::inspection::auto_training_tables::TemporalUnit;
use flow_like_ml_forecast::{ForecastFamily, ForecastResult, SeriesForecast};
use flow_like_types::Value;

fn mapping() -> ForecastTableMapping {
    ForecastTableMapping {
        timestamp: "time".into(),
        timestamp_format: None,
        series_id: Some("id".into()),
        targets: vec!["sales".into()],
        past_covariates: vec![],
        known_covariates: vec![],
        frequency_ms: None,
    }
}

fn prepare_rows(rows: Vec<Value>, mapping: &ForecastTableMapping) -> Result<PreparedForecast> {
    prepare(
        rows,
        &BTreeMap::new(),
        None,
        mapping,
        2,
        vec![0.1, 0.5, 0.9],
        &TableBudget::default(),
    )
}

fn rows() -> Vec<Value> {
    vec![
        json!({"id":"b","time":20,"sales":9}),
        json!({"id":"a","time":20,"sales":null}),
        json!({"id":"a","time":10,"sales":3}),
        json!({"id":"b","time":10,"sales":7}),
    ]
}

#[test]
fn groups_sort_chronologically_and_keep_null_target_masks() {
    let prepared = prepare_rows(rows(), &mapping()).unwrap();
    assert_eq!(
        prepared
            .request
            .series
            .iter()
            .map(|series| series.id.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    assert_eq!(prepared.request.series[0].targets, [vec![3.0, 0.0]]);
    assert_eq!(
        prepared.request.series[0].observed_mask,
        Some(vec![vec![true, false]])
    );
    assert_eq!(prepared.request.series[1].targets, [vec![7.0, 9.0]]);
    assert_eq!(prepared.clocks, [(20, 10), (20, 10)]);
}

#[test]
fn known_covariates_cover_history_and_exactly_aligned_future() {
    let mut mapping = mapping();
    mapping.past_covariates = vec!["temperature".into()];
    mapping.known_covariates = vec!["price".into()];
    let history = vec![
        json!({"id":"a","time":10,"sales":3,"temperature":15,"price":2}),
        json!({"id":"a","time":20,"sales":4,"temperature":18,"price":3}),
    ];
    let future = vec![
        json!({"id":"a","time":40,"price":5}),
        json!({"id":"a","time":30,"price":4}),
    ];
    let units = BTreeMap::new();
    let prepared = prepare(
        history.clone(),
        &units,
        Some((future.clone(), &units)),
        &mapping,
        2,
        vec![],
        &TableBudget::default(),
    )
    .unwrap();
    assert_eq!(
        prepared.request.series[0].known_covariates,
        [vec![2.0, 3.0, 4.0, 5.0]]
    );
    assert_eq!(
        prepared.request.series[0].past_covariates,
        [vec![15.0, 18.0]]
    );
    assert!(
        prepare(
            history.clone(),
            &units,
            None,
            &mapping,
            2,
            vec![],
            &TableBudget::default()
        )
        .is_err()
    );
    for bad in [
        vec![json!({"id":"a","time":30,"price":4})],
        vec![
            json!({"id":"a","time":30,"price":4}),
            json!({"id":"a","time":50,"price":5}),
        ],
        vec![
            json!({"id":"z","time":30,"price":4}),
            json!({"id":"z","time":40,"price":5}),
        ],
        vec![
            json!({"id":"a","time":30,"price":null}),
            json!({"id":"a","time":40,"price":5}),
        ],
    ] {
        assert!(
            prepare(
                history.clone(),
                &units,
                Some((bad, &units)),
                &mapping,
                2,
                vec![],
                &TableBudget::default()
            )
            .is_err()
        );
    }
    let mut extra = future;
    extra.push(json!({"id":"z","time":30,"price":4}));
    assert!(
        prepare(
            history,
            &units,
            Some((extra, &units)),
            &mapping,
            2,
            vec![],
            &TableBudget::default()
        )
        .is_err()
    );
}

#[test]
fn rejects_duplicates_irregular_spacing_and_unobserved_targets() {
    let mut values = rows();
    values.push(values[0].clone());
    assert!(
        prepare_rows(values, &mapping())
            .unwrap_err()
            .to_string()
            .contains("duplicate")
    );
    let values = vec![
        json!({"id":"a","time":10,"sales":1}),
        json!({"id":"a","time":20,"sales":2}),
        json!({"id":"a","time":40,"sales":3}),
    ];
    assert!(
        prepare_rows(values, &mapping())
            .unwrap_err()
            .to_string()
            .contains("regular")
    );
    let mut strict = mapping();
    strict.frequency_ms = Some(5);
    assert!(prepare_rows(rows(), &strict).is_err());
    assert!(
        prepare_rows(
            vec![
                json!({"id":"a","time":10,"sales":null}),
                json!({"id":"a","time":20,"sales":null})
            ],
            &mapping()
        )
        .is_err()
    );
    assert!(
        prepare_rows(
            vec![json!({"id":"a","time":10}), json!({"id":"a","time":20})],
            &mapping()
        )
        .unwrap_err()
        .to_string()
        .contains("missing")
    );
    let single = vec![json!({"id":"a","time":10,"sales":1})];
    assert!(prepare_rows(single.clone(), &mapping()).is_err());
    assert_eq!(prepare_rows(single, &strict).unwrap().clocks, [(10, 5)]);
}

#[test]
fn arrow_timestamps_convert_without_silently_truncating_precision() {
    let units = BTreeMap::from([("time".into(), TemporalUnit::Seconds)]);
    let prepared = prepare(
        rows(),
        &units,
        None,
        &mapping(),
        2,
        vec![],
        &TableBudget::default(),
    )
    .unwrap();
    assert_eq!(prepared.clocks, [(20_000, 10_000), (20_000, 10_000)]);
    let units = BTreeMap::from([("time".into(), TemporalUnit::Microseconds)]);
    assert!(
        prepare(
            rows(),
            &units,
            None,
            &mapping(),
            2,
            vec![],
            &TableBudget::default()
        )
        .unwrap_err()
        .to_string()
        .contains("precision")
    );
    let mut values = rows();
    for row in &mut values {
        row["time"] = json!(row["time"].as_i64().unwrap() * 1_000);
    }
    assert_eq!(
        prepare(
            values,
            &units,
            None,
            &mapping(),
            2,
            vec![],
            &TableBudget::default()
        )
        .unwrap()
        .clocks,
        [(20, 10), (20, 10)]
    );
}

#[test]
fn future_series_ids_cannot_silently_cast_between_strings_and_integers() {
    let mut mapping = mapping();
    mapping.known_covariates = vec!["price".into()];
    let units = BTreeMap::new();
    for (history_id, future_id) in [(json!(1), json!("1")), (json!("1"), json!(1))] {
        let history = vec![
            json!({"id":history_id,"time":10,"sales":1,"price":2}),
            json!({"id":history_id,"time":20,"sales":2,"price":2}),
        ];
        let future = vec![
            json!({"id":future_id,"time":30,"price":3}),
            json!({"id":future_id,"time":40,"price":3}),
        ];
        let error = prepare(
            history,
            &units,
            Some((future, &units)),
            &mapping,
            2,
            vec![],
            &TableBudget::default(),
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("same string or integer type"),
            "{error}"
        );
    }
}

#[test]
fn rejects_budget_overflow_bad_mappings_and_nonfinite_f32_conversion() {
    let budget = TableBudget {
        maximum_tensor_elements: 7,
        ..Default::default()
    };
    assert!(
        prepare(
            rows(),
            &BTreeMap::new(),
            None,
            &mapping(),
            2,
            vec![],
            &budget
        )
        .is_err()
    );
    let budget = TableBudget {
        maximum_rows: 3,
        ..Default::default()
    };
    assert!(
        prepare(
            rows(),
            &BTreeMap::new(),
            None,
            &mapping(),
            2,
            vec![],
            &budget
        )
        .is_err()
    );
    let mut bad_mapping = mapping();
    bad_mapping.past_covariates = vec!["sales".into()];
    assert!(prepare_rows(rows(), &bad_mapping).is_err());
    let mut values = rows();
    values[0]["sales"] = json!(1e100);
    assert!(prepare_rows(values, &mapping()).is_err());
    let values = vec![
        json!({"id":"a","time":i64::MAX - 1,"sales":1}),
        json!({"id":"a","time":i64::MAX,"sales":2}),
    ];
    assert!(
        prepare_rows(values, &mapping())
            .unwrap_err()
            .to_string()
            .contains("overflow")
    );
    let prepared = prepare_rows(rows(), &mapping()).unwrap();
    assert!(
        validate_output_budget(
            &prepared.request,
            21,
            &TableBudget {
                maximum_tensor_elements: 20,
                ..Default::default()
            }
        )
        .is_err()
    );
}

fn handle() -> ForecastModelHandle {
    ForecastModelHandle {
        cache_key: "model".into(),
        family: ForecastFamily::Timestral,
        repository: Some("mistralai/Timestral-5M".into()),
        revision: Some("commit".into()),
        bit_id: Some("bit".into()),
    }
}

fn result(prepared: &PreparedForecast) -> ForecastResult {
    ForecastResult {
        series: prepared
            .request
            .series
            .iter()
            .map(|series| SeriesForecast {
                id: series.id.clone(),
                point: vec![vec![4.0, 5.0]],
                quantiles: vec![vec![vec![3.0, 4.0, 5.0], vec![4.0, 5.0, 6.0]]],
            })
            .collect(),
        quantile_levels: vec![0.1, 0.5, 0.9],
        point_statistic: PointStatistic::Median,
    }
}

#[test]
fn output_rows_keep_named_targets_timestamps_quantiles_and_model_provenance() {
    let prepared = prepare_rows(rows(), &mapping()).unwrap();
    let rows = output_rows(
        &prepared,
        &result(&prepared),
        &handle(),
        "run",
        &TableBudget::default(),
    )
    .unwrap();
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0]["series_id"], "a");
    assert_eq!(rows[0]["target"], "sales");
    assert_eq!(rows[0]["timestamp_ms"], 30);
    assert_eq!(rows[1]["timestamp_ms"], 40);
    assert_eq!(rows[0]["point"], 4.0);
    assert_eq!(rows[0]["q_0_1"], 3.0);
    assert_eq!(rows[0]["q_0_5"], 4.0);
    assert_eq!(rows[0]["q_0_9"], 5.0);
    assert_eq!(rows[0]["point_statistic"], "median");
    assert_eq!(rows[0]["model_bit_id"], "bit");
    assert_eq!(rows[0]["model_revision"], "commit");
    assert_ne!(rows[0]["sample_id"], rows[1]["sample_id"]);
    assert_eq!(quantile_columns(&[0.1, 0.5, 0.9]).get("q_0_5"), Some(&0.5));
    assert!(
        output_rows(
            &prepared,
            &result(&prepared),
            &handle(),
            &"x".repeat(2048),
            &TableBudget {
                maximum_bytes: 1024,
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn point_only_table_output_preserves_groups_without_quantile_columns() {
    let mut prepared = prepare_rows(rows(), &mapping()).unwrap();
    prepared.request.quantile_levels.clear();
    let mut prediction = result(&prepared);
    prediction.quantile_levels.clear();
    prediction.point_statistic = PointStatistic::Mean;
    for series in &mut prediction.series {
        for target in &mut series.quantiles {
            for step in target {
                step.clear();
            }
        }
    }
    prediction.validate(&prepared.request).unwrap();
    let rows = output_rows(
        &prepared,
        &prediction,
        &handle(),
        "run",
        &TableBudget::default(),
    )
    .unwrap();
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0]["series_id"], "a");
    assert_eq!(rows[2]["series_id"], "b");
    assert!(rows.iter().all(|row| row["point_statistic"] == "mean"));
    assert!(rows.iter().all(|row| {
        row.as_object()
            .unwrap()
            .keys()
            .all(|key| !key.starts_with("q_"))
    }));
    assert!(quantile_columns(&[]).is_empty());
}

#[test]
fn forecast_exposes_table_pins_and_values_node_keeps_array_contract() {
    let node = ForecastNode.get_node();
    for pin in [
        "source",
        "model",
        "mapping",
        "horizon",
        "quantile_levels",
        "future_source",
        "destination",
        "budget",
        "table",
        "metadata",
    ] {
        assert!(node.get_pin_by_name(pin).is_some(), "Missing pin {pin}");
    }
    assert!(node.get_pin_by_name("request").is_none());
    assert_eq!(
        node.get_pin_by_name("source").unwrap().schema,
        node.get_pin_by_name("table").unwrap().schema
    );
    let values = super::super::ForecastValuesNode.get_node();
    assert_eq!(values.name, "ml_forecast_values");
    assert!(values.get_pin_by_name("request").is_some());
    assert!(values.get_pin_by_name("result").is_some());
}

#[cfg(feature = "execute")]
mod lance {
    use super::*;
    use crate::inspection::auto_training_tables::{
        TableWritePolicy, pin_table_source, read_columns, read_rows, write_experiment_rows,
    };
    use ahash::AHashMap;
    use flow_like::{
        flow::{
            board::ExecutionStage,
            execution::{
                LogLevel, Run, context::ExecutionContextCache, internal_node::InternalNode,
            },
        },
        profile::Profile,
        state::{FlowLikeConfig, FlowLikeState, FlowLikeStores},
        utils::http::HTTPClient,
    };
    use flow_like_catalog_core::CachedDB;
    use flow_like_storage::{
        databases::vector::{
            VectorStore, buffered::BufferedVectorStore, lancedb::LanceDBVectorStore,
        },
        files::store::{FlowLikeStore, local_store::LocalObjectStore},
        lancedb::Connection,
        object_store::path::Path,
    };
    use flow_like_types::sync::{Mutex, RwLock};
    use std::sync::{Arc, Weak};

    #[test]
    fn destination_types_cannot_change_timestamp_units_or_truncate_forecasts() {
        use flow_like_storage::arrow_schema::{DataType, Field, Schema, TimeUnit};
        let row = json!({"sample_id":"run:0", "timestamp_ms":1234, "point":1.25, "q_0_1":1.1});
        let schema = |timestamp, point| {
            Schema::new(vec![
                Field::new("sample_id", DataType::Utf8, false),
                Field::new("timestamp_ms", timestamp, false),
                Field::new("point", point, false),
                Field::new("q_0_1", DataType::Float64, false),
            ])
        };
        validate_destination_schema(&schema(DataType::Int64, DataType::Float32), &row).unwrap();
        assert!(
            validate_destination_schema(
                &schema(
                    DataType::Timestamp(TimeUnit::Second, None),
                    DataType::Float64
                ),
                &row
            )
            .is_err()
        );
        assert!(
            validate_destination_schema(&schema(DataType::Int64, DataType::Int64), &row).is_err()
        );
    }

    async fn context() -> (tempfile::TempDir, ExecutionContext, Connection) {
        let directory = tempfile::tempdir().unwrap();
        let location = directory
            .path()
            .join("database")
            .to_string_lossy()
            .into_owned();
        let mut config = FlowLikeConfig::new();
        let reopen = location.clone();
        config.register_build_project_database(Arc::new(move |_| {
            flow_like_storage::lancedb::connect(&reopen)
        }));
        let state = Arc::new(FlowLikeState::new(
            config,
            HTTPClient::new_without_refetch(),
        ));
        let logic: Arc<dyn NodeLogic> = Arc::new(ForecastNode);
        let node = Arc::new(InternalNode::new(
            logic.get_node(),
            AHashMap::new(),
            logic,
            AHashMap::new(),
        ));
        let mut nodes = AHashMap::new();
        nodes.insert(node.node_id().to_string(), node.clone());
        let run: Weak<Mutex<Run>> = Weak::new();
        let mut context = ExecutionContext::new(
            Arc::new(nodes),
            &run,
            &state,
            &node,
            &Arc::new(Mutex::new(AHashMap::new())),
            &Arc::new(RwLock::new(AHashMap::new())),
            LogLevel::Debug,
            ExecutionStage::Dev,
            Arc::new(Profile::default()),
            None,
            Arc::new(RwLock::new(Vec::new())),
            None,
            None,
            Arc::new(AHashMap::new()),
            None,
        )
        .await;
        let mut stores = FlowLikeStores::default();
        stores.app_storage_store = Some(FlowLikeStore::Local(Arc::new(
            LocalObjectStore::new(directory.path().to_path_buf()).unwrap(),
        )));
        context.execution_cache = Some(ExecutionContextCache {
            stores,
            app_id: "forecast-test".into(),
            model_usage_app_id: None,
            board_dir: Path::from("board"),
            board_id: "board".into(),
            node_id: node.shared_node_id(),
            sub: "tester".into(),
            shadow: false,
        });
        let connection = flow_like_storage::lancedb::connect(&location)
            .execute()
            .await
            .unwrap();
        (directory, context, connection)
    }

    async fn cache_table(
        context: &ExecutionContext,
        connection: Connection,
        name: &str,
        rows: Vec<Value>,
    ) -> NodeDBConnection {
        let mut store = LanceDBVectorStore::from_connection(connection, name.into()).await;
        store.insert(rows).await.unwrap();
        let database = NodeDBConnection {
            cache_key: name.into(),
        };
        context.cache.write().await.insert(
            database.cache_key.clone(),
            Arc::new(CachedDB {
                db: Arc::new(RwLock::new(BufferedVectorStore::new(store, 0))),
            }),
        );
        database
    }

    #[tokio::test]
    async fn forecast_materializes_a_pinned_lance_table_and_preserves_input_snapshot() {
        let (_directory, mut context, connection) = context().await;
        let original = cache_table(&context, connection.clone(), "history", rows()).await;
        let source = pin_table_source(&mut context, &original).await.unwrap();
        let budget = TableBudget::default();
        original
            .load(&mut context)
            .await
            .unwrap()
            .insert_from(&context, vec![json!({"id":"a","time":30,"sales":100})])
            .await
            .unwrap();
        let history = read_columns(&mut context, &source, &budget, &mapping().columns(false))
            .await
            .unwrap();
        assert_eq!(history.len(), 4);
        let prepared = prepare(
            history,
            &source.temporal_units,
            None,
            &mapping(),
            2,
            vec![0.1, 0.5, 0.9],
            &budget,
        )
        .unwrap();
        let produced =
            output_rows(&prepared, &result(&prepared), &handle(), "run", &budget).unwrap();
        let output = write_experiment_rows(
            &mut context,
            &source,
            produced.clone(),
            "run",
            "forecast",
            "",
            &TableWritePolicy {
                table_prefix: Some("ml_forecast".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert!(output.reference.pinned && output.reference.read_only);
        assert_ne!(output.reference.table, source.reference.table);
        let pinned_output = pin_table_source(&mut context, &output.database)
            .await
            .unwrap();
        let actual = read_rows(&mut context, &pinned_output, &budget, 16)
            .await
            .unwrap();
        assert_eq!(actual.len(), 4);
        let mut actual = actual;
        actual.sort_by_key(|row| row["sample_id"].as_str().unwrap().to_string());
        assert_eq!(actual, produced);
        let destination = cache_table(&context, connection.clone(), "destination", produced).await;
        let expected = destination_reference(&mut context, Some(&destination), &source, None)
            .await
            .unwrap();
        let next = output_rows(&prepared, &result(&prepared), &handle(), "next", &budget).unwrap();
        let destination_schema = destination
            .load(&mut context)
            .await
            .unwrap()
            .db
            .read()
            .await
            .schema()
            .await
            .unwrap();
        validate_destination_schema(&destination_schema, &next[0]).unwrap();
        let appended = write_experiment_rows(
            &mut context,
            &source,
            next,
            "next",
            "forecast",
            "",
            &TableWritePolicy {
                allowed_destination: Some(destination),
                expected_destination: expected,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let appended = pin_table_source(&mut context, &appended.database)
            .await
            .unwrap();
        assert_eq!(
            read_rows(&mut context, &appended, &budget, 16)
                .await
                .unwrap()
                .len(),
            8
        );
        let latest = pin_table_source(&mut context, &original).await.unwrap();
        assert_eq!(
            read_rows(&mut context, &latest, &budget, 16)
                .await
                .unwrap()
                .len(),
            5
        );
        assert!(
            destination_reference(&mut context, Some(&original), &source, None)
                .await
                .unwrap_err()
                .to_string()
                .contains("must differ")
        );
        let alias = NodeDBConnection {
            cache_key: "history-alias".into(),
        };
        let cached = context
            .cache
            .read()
            .await
            .get(&original.cache_key)
            .unwrap()
            .clone();
        context
            .cache
            .write()
            .await
            .insert(alias.cache_key.clone(), cached);
        assert!(
            destination_reference(&mut context, Some(&alias), &source, None)
                .await
                .is_err()
        );
        let future = cache_table(
            &context,
            connection,
            "future",
            vec![json!({"id":"a","time":40,"price":1})],
        )
        .await;
        let pinned_future = pin_table_source(&mut context, &future).await.unwrap();
        assert!(
            destination_reference(&mut context, Some(&future), &source, Some(&pinned_future))
                .await
                .is_err()
        );
    }
}

#[test]
fn numeric_text_targets_and_covariates_are_converted_without_losing_nulls() {
    let mut mapping = mapping();
    mapping.past_covariates = vec!["temperature".into()];
    let input = vec![
        json!({"id":"a", "time":10, "sales":" 6.243e1 ", "temperature":"-1.25"}),
        json!({"id":"a", "time":20, "sales":null, "temperature":".5"}),
    ];
    let prepared = prepare_rows(input, &mapping).unwrap();
    assert_eq!(prepared.request.series[0].targets, [vec![62.43, 0.0]]);
    assert_eq!(
        prepared.request.series[0].observed_mask,
        Some(vec![vec![true, false]])
    );
    for invalid in [
        "", " ", "NaN", "Infinity", "1,234", "$12", "1e100", "sensor",
    ] {
        let input = vec![
            json!({"id":"a", "time":10, "sales":invalid}),
            json!({"id":"a", "time":20, "sales":"2"}),
        ];
        assert!(
            prepare_rows(input, &super::tests::mapping()).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn forecast_timestamp_formats_normalize_before_grouping() {
    let mut mapping = mapping();
    mapping.timestamp_format = Some(ForecastTimestampFormat::Auto);
    let expected = 1_792_413_000_000_i64;
    for values in [
        [json!(1792413000), json!(1792413001)],
        [json!(1792413000000_i64), json!(1792413001000_i64)],
        [json!("1792413000000000"), json!("1792413001000000")],
        [json!("1792413000000000000"), json!("1792413001000000000")],
        [
            json!("2026-10-19T12:30:00Z"),
            json!("2026-10-19T14:30:01+02:00"),
        ],
        [json!("2026/10/19 12:30:00"), json!("19 Oct 2026 12:30:01")],
        [
            json!("October 19, 2026 12:30:00"),
            json!("Mon, 19 Oct 2026 12:30:01 +0000"),
        ],
    ] {
        let rows = vec![
            json!({"id":"a","time":values[1],"sales":"2"}),
            json!({"id":"a","time":values[0],"sales":"1"}),
        ];
        let prepared = prepare_rows(rows, &mapping).unwrap();
        assert_eq!(prepared.clocks, [(expected + 1000, 1000)]);
        assert_eq!(prepared.request.series[0].targets, [vec![1.0, 2.0]]);
    }
}

#[test]
fn timestamp_order_units_and_precision_are_explicit_when_ambiguous() {
    let mut mapping = mapping();
    mapping.timestamp_format = Some(ForecastTimestampFormat::Auto);
    let input = |first: Value, second: Value| {
        vec![
            json!({"id":"a","time":first,"sales":1}),
            json!({"id":"a","time":second,"sales":2}),
        ]
    };
    for (first, second) in [
        (json!(10), json!(20)),
        (json!(1792413000), json!(1792413001000_i64)),
        (json!("04/05/2026"), json!("05/05/2026")),
        (json!("2026-02-30"), json!("2026-03-01")),
        (
            json!("2026-10-19T12:30:00.0001Z"),
            json!("2026-10-19T12:30:01Z"),
        ),
        (json!(null), json!("2026-10-19")),
        (json!("1792413000.0001"), json!("1792413001")),
    ] {
        assert!(prepare_rows(input(first, second), &mapping).is_err());
    }
    mapping.timestamp_format = Some(ForecastTimestampFormat::DayFirst);
    let prepared = prepare_rows(input(json!("04/05/2026"), json!("05/05/2026")), &mapping).unwrap();
    assert_eq!(prepared.clocks[0].1, 86_400_000);
    mapping.timestamp_format = Some(ForecastTimestampFormat::UnixSeconds);
    assert_eq!(
        prepare_rows(input(json!("10.125"), json!("11.125")), &mapping)
            .unwrap()
            .clocks,
        [(11125, 1000)]
    );
    mapping.timestamp_format = None;
    assert_eq!(
        prepare_rows(input(json!(10), json!(20)), &mapping)
            .unwrap()
            .clocks,
        [(20, 10)]
    );
    mapping.timestamp_format = Some(ForecastTimestampFormat::Auto);
    let units = BTreeMap::from([("time".into(), TemporalUnit::Milliseconds)]);
    assert_eq!(
        prepare(
            input(json!(10), json!(20)),
            &units,
            None,
            &mapping,
            2,
            vec![],
            &TableBudget::default()
        )
        .unwrap()
        .clocks,
        [(20, 10)]
    );
}
