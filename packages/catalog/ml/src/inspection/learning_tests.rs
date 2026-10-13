use super::super::lifecycle::{
    LEDGER_SCOPE_KEY, LedgerScope, TRAINING_UNAVAILABLE, is_training_unavailable,
};
use super::super::published::{
    ChampionPointer, ModelStorage, POINTER_FORMAT_VERSION, experiment_pointer_path,
    learning_pointer_path,
};
use super::*;
use flow_like_catalog_core::{FlowPath, NodeDBConnection};
use flow_like_types::{Value, tokio_util::sync::CancellationToken};

#[test]
fn project_history_keeps_operator_dataset_exclusions() {
    use super::super::auto_training_tables::DatasetSelection;
    let mut selected = DatasetSelection {
        excluded_ids: vec!["operator".into()],
        excluded_groups: vec!["operator-group".into()],
        ineligible_test_ids: vec!["old-training".into()],
        ineligible_test_groups: vec!["old-group".into()],
    };
    merge_dataset_exclusions(
        &mut selected,
        LearningDatasetExclusions {
            sealed_audit_sample_ids: vec!["audit".into(), "operator".into()],
            sealed_audit_group_ids: vec!["audit-group".into()],
            previously_trained_sample_ids: vec!["trained".into()],
            previously_trained_group_ids: vec!["trained-group".into()],
        },
    );
    assert_eq!(selected.excluded_ids, ["audit", "operator"]);
    assert_eq!(selected.excluded_groups, ["audit-group", "operator-group"]);
    assert_eq!(selected.ineligible_test_ids, ["old-training", "trained"]);
    assert_eq!(
        selected.ineligible_test_groups,
        ["old-group", "trained-group"]
    );
}

#[test]
fn rolling_temporal_boundaries_advance_without_changing_embargo() {
    use super::super::auto_training_tables::TableSplitPolicy;
    let rolling = RollingTimeSplit {
        validation_duration_ms: 200,
        test_duration_ms: 100,
    };
    let mut split = TableSplitPolicy::Time {
        train_end_ms: 0,
        validation_end_ms: 0,
        embargo_ms: 25,
    };
    for (at_ms, expected_train, expected_validation) in [(1000, 700, 900), (1500, 1200, 1400)] {
        rolling.apply(&mut split, at_ms).unwrap();
        let TableSplitPolicy::Time {
            train_end_ms,
            validation_end_ms,
            embargo_ms,
        } = split
        else {
            panic!("temporal split changed kind")
        };
        assert_eq!(
            (train_end_ms, validation_end_ms, embargo_ms),
            (expected_train, expected_validation, 25)
        );
    }
    assert!(rolling.apply(&mut split, i64::MIN).is_err());
    let mut groups = TableSplitPolicy::Group {
        train_fraction: 0.6,
        validation_fraction: 0.2,
        seed: 42,
    };
    assert!(rolling.apply(&mut groups, 1000).is_err());
    assert!(
        RollingTimeSplit {
            validation_duration_ms: 0,
            test_duration_ms: 100
        }
        .apply(&mut split, 1000)
        .is_err()
    );
}

#[test]
fn only_stopped_experiments_settle_after_a_failed_resume() {
    use flow_like_ml_runtime::{Error, ExperimentStatus};
    let locked =
        flow_like_types::Error::from(Error::Conflict("training operation is locked".into()));
    let storage =
        flow_like_types::Error::from(Error::Io(std::io::Error::other("disk unavailable")));
    assert!(settles_after_failed_resume(
        ExperimentStatus::Cancelled,
        &locked
    ));
    assert!(settles_after_failed_resume(
        ExperimentStatus::CancelRequested,
        &locked
    ));
    assert!(!settles_after_failed_resume(
        ExperimentStatus::Cancelled,
        &storage
    ));
    assert!(!settles_after_failed_resume(
        ExperimentStatus::Running,
        &locked
    ));
}

#[test]
fn audit_replay_budget_counts_from_the_first_recorded_prediction() {
    let pause = || std::thread::sleep(std::time::Duration::from_millis(2));
    let cancel = CancelFlag::default();
    let mut budget = ReplayBudget::new("Champion audit replay", &cancel, 0);
    pause();
    budget.check().unwrap();
    budget.recorded();
    pause();
    let error = budget.check().unwrap_err().to_string();
    assert!(error.contains("exceeded its 0 ms step budget"), "{error}");
    let unlimited = ReplayBudget::new("Canary replay", &cancel, u64::MAX);
    cancel.cancel();
    let error = unlimited.check().unwrap_err().to_string();
    assert!(error.contains("Canary replay was cancelled"), "{error}");
}

#[tokio::test]
async fn dropping_a_cancellable_step_stops_its_blocking_work() {
    let stopped = CancelFlag::default();
    let flag = stopped.clone();
    let (started, wait_started) = std::sync::mpsc::channel();
    let (release, wait_release) = std::sync::mpsc::channel::<()>();
    let mut work = Box::pin(run_cancellable(
        None,
        move || flag.cancel(),
        "cancelled",
        move || {
            let _ = started.send(());
            let _ = wait_release.recv();
            Ok(())
        },
    ));
    let running = tokio::task::spawn_blocking(move || wait_started.recv());
    tokio::select! {
        _ = &mut work => panic!("blocking work finished before it was released"),
        ready = running => ready.unwrap().unwrap(),
    }
    assert!(!stopped.is_cancelled());
    // The executor drops a cancelled run's node future without polling it again.
    drop(work);
    assert!(stopped.is_cancelled());
    let _ = release.send(());
}

#[tokio::test]
async fn a_cancelled_run_token_stops_cancellable_work() {
    let stopped = CancelFlag::default();
    let flag = stopped.clone();
    let token = CancellationToken::new();
    token.cancel();
    let (release, wait_release) = std::sync::mpsc::channel::<()>();
    let error = run_cancellable(
        Some(token),
        move || flag.cancel(),
        "audit cancelled",
        move || {
            let _ = wait_release.recv();
            Ok(())
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error.to_string(), "audit cancelled");
    assert!(stopped.is_cancelled());
    let _ = release.send(());
}

#[tokio::test]
async fn finished_cancellable_work_keeps_its_result_and_disarms_the_guard() {
    let stopped = CancelFlag::default();
    let flag = stopped.clone();
    let value = run_cancellable(
        Some(CancellationToken::new()),
        move || flag.cancel(),
        "cancelled",
        || Ok(7),
    )
    .await
    .unwrap();
    assert_eq!(value, 7);
    assert!(!stopped.is_cancelled());
}

struct Directory(std::path::PathBuf);
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Harness {
    context: ExecutionContext,
    source: NodeDBConnection,
    _directory: Directory,
}

fn source_rows(range: std::ops::Range<usize>) -> Vec<Value> {
    range.map(|i|json!({"id":i.to_string(),"group":(i/2).to_string(),"time":i as i64+1,"measurement":(i%2) as f64*100.+i as f64/1000.,"target":if i%2==0{"pass"}else{"fail"}})).collect()
}

/// A board context whose project database holds `rows` source rows of two-row groups.
async fn learning_harness(rows: usize) -> Harness {
    learning_harness_in(rows, false).await
}

/// A board context of user "tester" with local app storage, a local user store, and project and
/// user databases; the source rows live in the user's database when `private` is set.
async fn learning_harness_in(rows: usize, private: bool) -> Harness {
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
        object_store::path::Path,
    };
    use flow_like_types::sync::{Mutex, RwLock};
    use std::sync::{Arc, Weak};
    let directory = Directory(std::env::temp_dir().join(format!(
        "flow-like-learning-project-{}",
        uuid::Uuid::new_v4()
    )));
    let user_store = directory.0.join("user-store");
    std::fs::create_dir_all(&user_store).unwrap();
    let project_location = directory.0.join("database").to_string_lossy().into_owned();
    let user_location = directory
        .0
        .join("user-database")
        .to_string_lossy()
        .into_owned();
    let mut config = FlowLikeConfig::new();
    let reopen = project_location.clone();
    config.register_build_project_database(Arc::new(move |_| {
        flow_like_storage::lancedb::connect(&reopen)
    }));
    let reopen = user_location.clone();
    config.register_build_user_database(Arc::new(move |_| {
        flow_like_storage::lancedb::connect(&reopen)
    }));
    let db_location = if private {
        user_location
    } else {
        project_location
    };
    let state = Arc::new(FlowLikeState::new(
        config,
        HTTPClient::new_without_refetch(),
    ));
    let logic: Arc<dyn NodeLogic> = Arc::new(CreateLearningProjectNode);
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
        LocalObjectStore::new(directory.0.clone()).unwrap(),
    )));
    stores.user_store = Some(FlowLikeStore::Local(Arc::new(
        LocalObjectStore::new(user_store).unwrap(),
    )));
    context.execution_cache = Some(ExecutionContextCache {
        stores,
        app_id: "learning-test".into(),
        model_usage_app_id: None,
        board_dir: Path::from("board"),
        board_id: "board".into(),
        node_id: node.shared_node_id(),
        sub: "tester".into(),
        shadow: false,
    });
    let connection = flow_like_storage::lancedb::connect(&db_location)
        .execute()
        .await
        .unwrap();
    let mut store = LanceDBVectorStore::from_connection(connection, "measurements".into()).await;
    store.insert(source_rows(0..rows)).await.unwrap();
    let source = NodeDBConnection {
        cache_key: "learning-source".into(),
    };
    context.cache.write().await.insert(
        source.cache_key.clone(),
        Arc::new(CachedDB {
            db: Arc::new(RwLock::new(BufferedVectorStore::new(store, 0))),
        }),
    );
    Harness {
        context,
        source,
        _directory: directory,
    }
}

/// Automatic promotion with a canary over a measured pass/fail table; tests edit fields first.
fn project_request(source: &NodeDBConnection) -> Value {
    let recipe = super::super::auto_training_legacy::LegacyRecipe {
        algorithm: super::super::auto_training_legacy::LegacyAlgorithm::GaussianNaiveBayes,
        labels: vec!["pass".into(), "fail".into()],
    };
    json!({
        "training":{
            "dataset":{"source":source,"stream_id":"quality","spec":{"id":"quality","description":"Predict measured quality","task":"sensor_classification","labels":["pass","fail"],"input_shape":[1],"prediction_horizon_ms":null,"minimum_examples":2,"minimum_examples_per_class":1},
                "mapping":{"row_id":"id","group_id":"group","timestamp_ms":"time","features":{"kind":"tabular","options":{"numeric_columns":["measurement"],"categorical_columns":[]}},"target":{"kind":"column","column":"target"},"provenance":{"kind":"measured","source":"gauge"}},
                "split":{"kind":"group","train_fraction":0.6,"validation_fraction":0.2,"seed":42}},
            "compute":{"backend":"cpu"},"search":{"maximum_candidates":1},"budget":{"maximum_trials":1},
            "feature_plan":{"pipeline":{"steps":[{"kind":"derive_columns","columns":[{"name":"energy","expression":{"kind":"unary","op":"square","value":{"kind":"column","name":"measurement"}}}]}]},"numeric_columns":["measurement","energy"],"categorical_columns":[],"standardize":true},
            "initial_candidates":[{"engine":"legacy_classical","recipe":recipe,"compute":{"backend":"cpu"}}],
            "goals":{"task":{"kind":"classification"},"primary_metric":"accuracy","direction":"maximize","minimum_audited_samples":4,"bounds":[{"name":"accuracy","minimum":0.9,"maximum":null}]}
        },"policy":{"minimum_new_reviews":1,"minimum_audit_samples":4,"cooldown_ms":0,"canary_fraction":0.5},"automatic_promotion":true
    })
}

async fn create_project(harness: &mut Harness, request: Value) -> Result<LearningProjectResult> {
    create(
        &mut harness.context,
        flow_like_types::json::from_value(request)?,
    )
    .await
}

/// The champion pointer published for project `id` in the harness's app storage.
fn champion_pointer(directory: &Directory, id: &str) -> ChampionPointer {
    let path = directory
        .0
        .join("board")
        .join("storage")
        .join(learning_pointer_path(id));
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    flow_like_types::json::from_slice(&bytes).unwrap()
}

fn experiment_published(directory: &Directory, id: &str) -> bool {
    directory
        .0
        .join("board")
        .join("storage")
        .join(experiment_pointer_path(id))
        .exists()
}

/// A champion pointer of `order`; without `artifact_id` the teacher decides.
fn champion(
    project_id: &str,
    artifact_id: Option<&str>,
    paused: bool,
    order: (i64, i64),
) -> ChampionPointer {
    ChampionPointer {
        format_version: POINTER_FORMAT_VERSION,
        project_id: project_id.into(),
        deployment_id: format!("deployment:{project_id}"),
        deployment_generation: order.0,
        project_generation: order.1,
        artifact_id: artifact_id.map(str::to_owned),
        sha256: artifact_id.map(|_| "a".repeat(64)),
        paused,
        cycle_id: None,
        retained: Vec::new(),
        storage: ModelStorage::App,
        published_at_ms: 0,
    }
}

/// Writes `pointer` where the app storage keeps its project's champion and drops the run's
/// cached pointer probes, so the next route reads it.
async fn publish_pointer(context: &mut ExecutionContext, pointer: &ChampionPointer) {
    let root = FlowPath::from_storage_dir(context, false).await.unwrap();
    FlowPath {
        path: format!("{}/{}", root.path, learning_pointer_path(&pointer.project_id)),
        store_ref: root.store_ref,
        cache_store_ref: None,
    }
    .put(context, flow_like_types::json::to_vec(pointer).unwrap(), true)
    .await
    .unwrap();
    context.cache.write().await.clear();
}

#[tokio::test]
async fn learning_projects_on_user_tables_live_in_the_private_ledger() {
    let mut harness = learning_harness_in(240, true).await;
    let request = project_request(&harness.source);
    let created = create_project(&mut harness, request).await.unwrap();
    let id = created.project.id.clone();
    let stream = &created.project.request.stream;
    assert!(
        stream.stream_id.starts_with("user:learning:"),
        "{}",
        stream.stream_id
    );
    assert_eq!(stream.project_id, "learning-test:board:user");
    assert_eq!(
        created.project.request.deployment_id,
        format!("deployment:{}", stream.stream_id)
    );
    assert!(
        created
            .project
            .request
            .deployment_id
            .starts_with("deployment:user:learning:")
    );
    assert_eq!(created.project.request.source[LEDGER_SCOPE_KEY], "user");
    let project_id = || LearningProjectId {
        project_id: id.clone(),
    };
    let read = get(&mut harness.context, project_id()).await.unwrap();
    assert_eq!(read.project.id, id);
    let stepped = step(&mut harness.context, project_id()).await.unwrap();
    assert_eq!(stepped.project.usage.cycles_started, 1);
    let (repo, scope) = project_ledger(&harness.context, &id).unwrap();
    assert_eq!(scope, LedgerScope::User);
    let cycle = repo
        .get_experiment(&stepped.experiment.unwrap().experiment_id)
        .unwrap();
    assert_eq!(cycle.request.stream, created.project.request.stream);
    assert_eq!(cycle.request.context[LEDGER_SCOPE_KEY], "user");
    let ledgers = super::super::lifecycle::ledger_paths(&harness.context).unwrap();
    assert!(!ledgers.app.exists(), "{}", ledgers.app.display());
}

#[tokio::test]
async fn learning_projects_need_a_local_training_ledger() {
    let mut harness = learning_harness(4).await;
    harness
        .context
        .execution_cache
        .as_mut()
        .unwrap()
        .stores
        .app_storage_store = Some(flow_like_storage::files::store::FlowLikeStore::Memory(
        std::sync::Arc::new(flow_like_storage::object_store::memory::InMemory::new()),
    ));
    let mut request = project_request(&harness.source);
    request["training"]["dataset"]["source"] = json!({"cache_key":"unreadable-source"});
    let error = create_project(&mut harness, request)
        .await
        .err()
        .expect("learning projects need a local ledger");
    assert!(is_training_unavailable(&error), "{error}");
    assert_eq!(error.to_string(), TRAINING_UNAVAILABLE);
}

#[tokio::test]
async fn routing_without_a_ledger_follows_the_published_champion() {
    let mut harness = learning_harness(4).await;
    harness
        .context
        .execution_cache
        .as_mut()
        .unwrap()
        .stores
        .app_storage_store = Some(flow_like_storage::files::store::FlowLikeStore::Memory(
        std::sync::Arc::new(flow_like_storage::object_store::memory::InMemory::new()),
    ));
    let route_to = |project_id: &str| LearningRouteRequest {
        project_id: project_id.into(),
        sample_id: "part-1".into(),
    };
    let published = champion("published", Some("champion-artifact"), false, (1, 1));
    publish_pointer(&mut harness.context, &published).await;
    match route(&mut harness.context, route_to("published"))
        .await
        .unwrap()
    {
        LearningRoute::Champion { artifact_id } => assert_eq!(artifact_id, "champion-artifact"),
        other => panic!("expected the published champion, got {}", json!(other)),
    }
    let paused = champion("published", Some("champion-artifact"), true, (1, 2));
    publish_pointer(&mut harness.context, &paused).await;
    assert!(matches!(
        route(&mut harness.context, route_to("published"))
            .await
            .unwrap(),
        LearningRoute::Teacher
    ));
    publish_pointer(&mut harness.context, &champion("collecting", None, false, (0, 1))).await;
    assert!(matches!(
        route(&mut harness.context, route_to("collecting"))
            .await
            .unwrap(),
        LearningRoute::Teacher
    ));
    let error = route(&mut harness.context, route_to("unpublished"))
        .await
        .unwrap_err()
        .to_string();
    assert!(
        error.contains(&learning_pointer_path("unpublished")),
        "{error}"
    );
}

#[tokio::test]
async fn a_newer_local_champion_pointer_routes_before_the_ledger() {
    let mut harness = learning_harness(4).await;
    let mut request = project_request(&harness.source);
    request["automatic_promotion"] = json!(false);
    let created = create_project(&mut harness, request).await.unwrap();
    let id = created.project.id.clone();
    let route_request = || LearningRouteRequest {
        project_id: id.clone(),
        sample_id: "part-1".into(),
    };
    assert!(matches!(
        route(&mut harness.context, route_request()).await.unwrap(),
        LearningRoute::Teacher
    ));
    let generation = created.project.generation;
    let equal = champion(&id, Some("equal-order"), false, (0, generation));
    publish_pointer(&mut harness.context, &equal).await;
    assert!(
        matches!(
            route(&mut harness.context, route_request()).await.unwrap(),
            LearningRoute::Teacher
        ),
        "a pointer of equal order leaves the ledger in charge"
    );
    let newer = champion(&id, Some("newer-champion"), false, (1, generation));
    publish_pointer(&mut harness.context, &newer).await;
    match route(&mut harness.context, route_request()).await.unwrap() {
        LearningRoute::Champion { artifact_id } => assert_eq!(artifact_id, "newer-champion"),
        other => panic!("expected the newer published champion, got {}", json!(other)),
    }
}

#[tokio::test]
async fn learning_projects_require_measurable_quality_bounds() {
    let mut harness = learning_harness(4).await;
    let mut unbounded = project_request(&harness.source);
    unbounded["automatic_promotion"] = json!(false);
    unbounded["training"]["goals"]["bounds"] = json!([]);
    let error = create_project(&mut harness, unbounded)
        .await
        .err()
        .expect("an unbounded project must be rejected");
    assert!(
        error
            .to_string()
            .contains("Learning projects require explicit quality bounds"),
        "{error}"
    );
    let mut unmeasured = project_request(&harness.source);
    unmeasured["training"]["goals"]["bounds"] =
        json!([{"name":"rmse","minimum":null,"maximum":1.0}]);
    let error = create_project(&mut harness, unmeasured)
        .await
        .err()
        .expect("a bound validation cannot measure must be rejected");
    assert!(error.to_string().contains("rmse"), "{error}");
    let (repo, scope) = super::super::lifecycle::repository(&harness.context).unwrap();
    assert!(repo.learning_projects(&scope).unwrap().is_empty());
}

#[tokio::test]
async fn undersized_audit_split_releases_its_cycle_reservation() {
    let mut harness = learning_harness(60).await;
    let mut request = project_request(&harness.source);
    // About 12 of 60 rows reach the test split, below the audit minimum.
    request["policy"]["minimum_audit_samples"] = json!(25);
    let id = create_project(&mut harness, request)
        .await
        .unwrap()
        .project
        .id;
    let error = step(
        &mut harness.context,
        LearningProjectId {
            project_id: id.clone(),
        },
    )
    .await
    .err()
    .expect("a step without enough audit samples must fail");
    assert!(error.to_string().contains("audit"), "{error}");
    let repo = project_repository(&harness.context, &id).unwrap();
    let project = repo.get_learning_project(&id).unwrap();
    assert!(project.active_cycle_id.is_none());
    assert_eq!(project.state, LearningState::Collecting);
    assert_eq!(project.usage.cycles_aborted, 1);
    let cycles = repo.learning_cycles(&id).unwrap();
    assert_eq!(cycles.len(), 1);
    assert_eq!(cycles[0].state, LearningCycleState::Aborted);
    assert!(
        repo.find_learning_cycle_experiment(&cycles[0].id)
            .unwrap()
            .is_none()
    );
    let waiting = step(&mut harness.context, LearningProjectId { project_id: id })
        .await
        .unwrap();
    assert!(
        waiting
            .next_actions
            .blocking
            .contains(&"abort_backoff".to_string())
    );
    assert_eq!(waiting.project.usage.cycles_started, 1);
}

#[tokio::test]
async fn cancelled_cycle_past_its_wall_budget_settles_on_the_next_step() {
    let mut harness = learning_harness(240).await;
    let request = project_request(&harness.source);
    let id = create_project(&mut harness, request)
        .await
        .unwrap()
        .project
        .id;
    let prepared = prepare_learning_cycle(&mut harness.context, &id)
        .await
        .unwrap();
    let experiment_id = prepared.experiment.unwrap().experiment_id;
    let repo = project_repository(&harness.context, &id).unwrap();
    let wall_ms = repo
        .get_experiment(&experiment_id)
        .unwrap()
        .request
        .budget
        .maximum_wall_time_ms;
    let now = flow_like_ml_runtime::now_ms();
    repo.mark_experiment_active(&experiment_id, now).unwrap();
    // Paused mid-training and resumed after its whole wall budget had elapsed.
    repo.cancel_experiment(&experiment_id, now + 2 * i64::try_from(wall_ms).unwrap())
        .unwrap();
    let settled = step(
        &mut harness.context,
        LearningProjectId {
            project_id: id.clone(),
        },
    )
    .await
    .unwrap();
    assert!(settled.project.active_cycle_id.is_none());
    assert_eq!(settled.project.state, LearningState::Collecting);
    assert_eq!(settled.cycles[0].state, LearningCycleState::Settled);
    let experiment = settled.experiment.unwrap();
    assert_eq!(
        experiment.status,
        flow_like_ml_runtime::ExperimentStatus::BudgetExhausted
    );
    assert_eq!(
        experiment.stop_reason.as_deref(),
        Some("wall_time_budget_exhausted")
    );
    assert!(
        !settled
            .next_actions
            .blocking
            .contains(&"cycle_in_progress".to_string())
    );
}

#[tokio::test]
async fn stopped_cycle_settles_when_its_experiment_cannot_resume() {
    use flow_like_ml_runtime::Error;
    let mut harness = learning_harness(60).await;
    let request = project_request(&harness.source);
    let id = create_project(&mut harness, request)
        .await
        .unwrap()
        .project
        .id;
    let prepared = prepare_learning_cycle(&mut harness.context, &id)
        .await
        .unwrap();
    let experiment_id = prepared.experiment.unwrap().experiment_id;
    let repo = project_repository(&harness.context, &id).unwrap();
    repo.cancel_experiment(&experiment_id, flow_like_ml_runtime::now_ms())
        .unwrap();
    let project_id = || LearningProjectId {
        project_id: id.clone(),
    };
    let error = step_with(&mut harness.context, project_id(), |_, _| {
        Err(Error::Io(std::io::Error::other("disk unavailable")).into())
    })
    .await
    .err()
    .expect("a storage failure must reach the caller");
    assert!(error.to_string().contains("disk unavailable"), "{error}");
    assert!(
        repo.get_learning_project(&id)
            .unwrap()
            .active_cycle_id
            .is_some()
    );
    let settled = step_with(&mut harness.context, project_id(), |_, _| {
        Err(Error::Conflict("experiment must be stopped before resuming".into()).into())
    })
    .await
    .unwrap();
    assert!(settled.project.active_cycle_id.is_none());
    assert_eq!(settled.project.state, LearningState::Collecting);
    assert_eq!(settled.cycles[0].state, LearningCycleState::Settled);
}

#[tokio::test]
async fn overlapping_preparation_reports_the_project_without_reserving_a_cycle() {
    let mut harness = learning_harness(240).await;
    let request = project_request(&harness.source);
    let id = create_project(&mut harness, request)
        .await
        .unwrap()
        .project
        .id;
    let repo = project_repository(&harness.context, &id).unwrap();
    let held = try_lock_learning_project(&repo, &id).unwrap().unwrap();
    assert!(try_lock_learning_project(&repo, &id).unwrap().is_none());
    let waiting = prepare_learning_cycle(&mut harness.context, &id)
        .await
        .unwrap();
    assert_eq!(waiting.project.usage.cycles_started, 0);
    assert!(waiting.project.active_cycle_id.is_none());
    assert!(waiting.experiment.is_none());
    drop(held);
    let prepared = prepare_learning_cycle(&mut harness.context, &id)
        .await
        .unwrap();
    assert_eq!(prepared.project.usage.cycles_started, 1);
    assert!(prepared.experiment.is_some());
}

#[tokio::test]
async fn learning_project_reopens_source_retrains_and_promotes_on_fresh_audit_groups() {
    run_learning_project(CanaryReviewScenario::Unchanged).await;
}

#[tokio::test]
async fn corrected_canary_review_rejects_cached_predictions_and_retains_champion() {
    run_learning_project(CanaryReviewScenario::CorrectedAutomatic).await;
}

#[tokio::test]
async fn manual_promotion_preserves_corrected_canary_rejection() {
    run_learning_project(CanaryReviewScenario::CorrectedManual).await;
}

#[tokio::test]
async fn rejected_candidate_keeps_the_champion_and_seals_its_canary() {
    run_learning_project(CanaryReviewScenario::Rejected).await;
}

#[tokio::test]
async fn corrected_audit_review_removes_the_waiting_canary_candidate() {
    run_learning_project(CanaryReviewScenario::CorrectedAudit).await;
}

#[tokio::test]
async fn champion_replay_records_a_prediction_per_call_past_its_step_budget() {
    run_learning_project(CanaryReviewScenario::ChampionReplayBudget).await;
}

enum CanaryReviewScenario {
    Unchanged,
    CorrectedAutomatic,
    CorrectedManual,
    Rejected,
    CorrectedAudit,
    ChampionReplayBudget,
}

/// Replays the first cycle's audit for `model` under a zero step budget; every call must still
/// record a prediction until the audit is complete.
fn replay_champion_past_its_budget(
    repo: &flow_like_ml_runtime::TrainingRepository,
    waiting: &LearningProjectResult,
    model: &str,
) {
    let mut project = waiting.project.clone();
    project.champion_artifact_id = Some(model.into());
    project.request.source["training"]["budget"]["worker_limits"]["maximum_duration_ms"] = json!(0);
    let audit = &waiting.cycles[0];
    let unrecorded = || {
        let recorded: std::collections::HashSet<_> = repo
            .predictions(model)
            .unwrap()
            .into_iter()
            .map(|prediction| prediction.sample_id)
            .collect();
        audit
            .audit_sample_ids
            .iter()
            .filter(|sample| !recorded.contains(*sample))
            .count()
    };
    let mut remaining = unrecorded();
    assert!(remaining > 0);
    while remaining > 0 {
        if let Err(error) = audit_champion(repo, &project, audit, &CancelFlag::default()) {
            assert!(error.to_string().contains("0 ms step budget"), "{error}");
        }
        let left = unrecorded();
        assert!(left < remaining, "a replay call recorded no prediction");
        remaining = left;
    }
}

/// The current time once the millisecond clock has passed `at_ms`.
fn after(at_ms: i64) -> i64 {
    loop {
        let now = flow_like_ml_runtime::now_ms();
        if now > at_ms {
            return now;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

/// Observes one of the waiting candidate's sealed audit samples, other than the reviewed "240",
/// and returns a review confirming its source label that becomes available after the comparison.
fn confirming_audit_review(
    repo: &flow_like_ml_runtime::TrainingRepository,
    id: &str,
    waiting: &LearningProjectResult,
) -> LearningReview {
    let sample = waiting.cycles[1]
        .audit_sample_ids
        .iter()
        .find(|sample| *sample != "240")
        .unwrap()
        .clone();
    let index: usize = sample.parse().unwrap();
    repo.record_learning_observation(
        id,
        &LearningObservation {
            sample_id: sample.clone(),
            group_id: (index / 2).to_string(),
            captured_at_ms: index as i64 + 1,
            student: None,
            teacher: None,
            embedding: vec![],
            slices: Default::default(),
            sample: json!({}),
        },
    )
    .unwrap();
    LearningReview {
        sample_id: sample,
        annotation: json!({"kind":"class","class_id":index % 2}),
        source: flow_like_ml_runtime::LabelSource::ObservedOutcome,
        reviewer: "gauge".into(),
        available_at_ms: after(waiting.comparison.as_ref().unwrap().evaluated_at_ms),
        revision: 1,
        outcome: None,
    }
}

/// A comparison that a concurrent review aborts leaves the waiting candidate on its sealed
/// comparison until the next step.
async fn finish_while_reviews_race_compare(
    context: &mut ExecutionContext,
    id: &str,
    compared: &LearningComparison,
) {
    let raced = finish_with(context, id, |_, _, _| {
        Err(flow_like_ml_runtime::Error::Conflict(
            "learning project generation or clock changed".into(),
        ))
    })
    .await
    .unwrap();
    assert_eq!(raced.project.state, LearningState::Canary);
    assert_eq!(
        raced.comparison.unwrap().candidate_evaluation_id,
        compared.candidate_evaluation_id
    );
}

/// Reviews of a sealed audit sample that become available while the candidate waits in Canary
/// reach compare: confirming truth refreshes the comparison once, contradicting it removes the
/// candidate.
async fn review_waiting_audit_sample(
    context: &mut ExecutionContext,
    repo: &flow_like_ml_runtime::TrainingRepository,
    id: &str,
    waiting: &LearningProjectResult,
    champion: &str,
) {
    let compared = waiting.comparison.clone().unwrap();
    let mut review = confirming_audit_review(repo, id, waiting);
    repo.record_learning_review(id, &review).unwrap();
    finish_while_reviews_race_compare(context, id, &compared).await;
    let confirmed = finish_learning_cycle(context, id).await.unwrap();
    assert_eq!(confirmed.project.state, LearningState::Canary);
    let refreshed = confirmed.comparison.unwrap();
    assert!(refreshed.eligible);
    assert_ne!(
        refreshed.candidate_evaluation_id,
        compared.candidate_evaluation_id
    );
    let kept = finish_learning_cycle(context, id)
        .await
        .unwrap()
        .comparison
        .unwrap();
    assert_eq!(
        kept.candidate_evaluation_id,
        refreshed.candidate_evaluation_id
    );

    review.revision = 2;
    review.annotation["class_id"] = json!(1 - review.annotation["class_id"].as_u64().unwrap());
    review.available_at_ms = after(refreshed.evaluated_at_ms);
    repo.record_learning_review(id, &review).unwrap();
    let rejected = finish_learning_cycle(context, id).await.unwrap();
    assert_eq!(rejected.project.state, LearningState::Active);
    assert_eq!(
        rejected.project.champion_artifact_id.as_deref(),
        Some(champion)
    );
    assert!(rejected.project.candidate_artifact_id.is_none());
    let comparison = rejected.comparison.unwrap();
    assert!(!comparison.eligible);
    assert!(
        comparison
            .reasons
            .contains(&"audit_review_changed".to_string())
    );
}

async fn run_learning_project(scenario: CanaryReviewScenario) {
    let Harness {
        mut context,
        source,
        _directory,
    } = learning_harness(240).await;
    let request: CreateLearningProjectRequest =
        flow_like_types::json::from_value(project_request(&source)).unwrap();
    let project = create(&mut context, request).await.unwrap();
    let id = project.project.id;
    let teacher = champion_pointer(&_directory, &id);
    assert_eq!(teacher.project_id, id);
    assert!(teacher.artifact_id.is_none() && !teacher.paused);
    let prepared = prepare_learning_cycle(&mut context, &id).await.unwrap();
    let experiment_id = prepared.experiment.unwrap().experiment_id;
    super::super::auto_training::run_next_auto_training(&context, &experiment_id)
        .await
        .unwrap();
    let exhausted = super::super::auto_training::run_next_auto_training(&context, &experiment_id)
        .await
        .unwrap();
    assert_eq!(
        exhausted.status,
        flow_like_ml_runtime::ExperimentStatus::BudgetExhausted
    );
    let pending = project_repository(&context, &id)
        .unwrap()
        .get_experiment(&experiment_id)
        .unwrap();
    assert!(pending.best_trial_id.is_some());
    assert!(pending.final_evaluation_id.is_none());
    let first = step(
        &mut context,
        LearningProjectId {
            project_id: id.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(first.project.state, LearningState::Active);
    assert!(first.comparison.as_ref().unwrap().eligible);
    let champion = first.project.champion_artifact_id.clone().unwrap();
    assert_eq!(
        champion_pointer(&_directory, &id).artifact_id.as_deref(),
        Some(champion.as_str())
    );
    // Only the champion is published, never a learning cycle's experiment.
    assert!(!experiment_published(&_directory, &experiment_id));
    let original_audit = first.cycles[0].audit_group_ids.clone();
    let cached = source.load(&mut context).await.unwrap();
    cached
        .upsert_from(&context, source_rows(240..480), "id".into())
        .await
        .unwrap();
    cached.ensure_flushed().await.unwrap();
    let repo = project_repository(&context, &id).unwrap();
    repo.record_learning_observation(
        &id,
        &LearningObservation {
            sample_id: "240".into(),
            group_id: "120".into(),
            captured_at_ms: 241,
            student: None,
            teacher: None,
            embedding: vec![],
            slices: Default::default(),
            sample: json!({}),
        },
    )
    .unwrap();
    repo.record_learning_review(
        &id,
        &LearningReview {
            sample_id: "240".into(),
            annotation: json!({"kind":"class","class_id":0}),
            source: flow_like_ml_runtime::LabelSource::ObservedOutcome,
            reviewer: "gauge".into(),
            available_at_ms: flow_like_ml_runtime::now_ms(),
            revision: 1,
            outcome: None,
        },
    )
    .unwrap();
    context.cache.write().await.clear();
    let second = step(
        &mut context,
        LearningProjectId {
            project_id: id.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(second.project.state, LearningState::Canary);
    assert_eq!(second.project.usage.cycles_finished, 2);
    assert!(
        second.cycles[1]
            .audit_group_ids
            .iter()
            .all(|group| !original_audit.contains(group))
    );
    assert!(second.comparison.as_ref().unwrap().champion_metrics["accuracy"] >= 0.9);
    let candidate = second.project.candidate_artifact_id.clone().unwrap();

    // A waiting canary reuses its sealed comparison instead of evaluating both models again.
    let compared = second.comparison.clone().unwrap();
    let waiting = finish_learning_cycle(&mut context, &id).await.unwrap();
    assert_eq!(waiting.project.state, LearningState::Canary);
    let kept = waiting.comparison.unwrap();
    assert_eq!(
        kept.candidate_evaluation_id,
        compared.candidate_evaluation_id
    );
    assert_eq!(kept.evaluated_at_ms, compared.evaluated_at_ms);
    assert!(
        waiting
            .next_actions
            .blocking
            .contains(&"canary_requires_reviewed_traffic_and_both_model_predictions".to_string())
    );

    // The champion already predicted every sealed audit sample, so nothing is reloaded.
    let waiting_project = repo.get_learning_project(&id).unwrap();
    let mut audited = second.cycles[1].clone();
    audited.experiment_id = Some("missing-experiment".into());
    audit_champion(&repo, &waiting_project, &audited, &CancelFlag::default()).unwrap();
    audited.audit_sample_ids.push("never-recorded".into());
    let cancelled = CancelFlag::default();
    cancelled.cancel();
    let error = audit_champion(&repo, &waiting_project, &audited, &cancelled).unwrap_err();
    assert!(error.to_string().contains("cancelled"), "{error}");
    match scenario {
        CanaryReviewScenario::CorrectedAudit => {
            review_waiting_audit_sample(&mut context, &repo, &id, &second, &champion).await;
            return;
        }
        CanaryReviewScenario::ChampionReplayBudget => {
            replay_champion_past_its_budget(&repo, &second, &candidate);
            return;
        }
        _ => {}
    }

    let mut observations = Vec::new();
    let mut reviews = Vec::new();
    for (i, mut row) in (480..488).zip(source_rows(480..488)) {
        let at = flow_like_ml_runtime::now_ms();
        row["time"] = json!(at);
        observations.push(LearningObservation {
            sample_id: i.to_string(),
            group_id: (i / 2).to_string(),
            captured_at_ms: at,
            student: Some(flow_like_ml_runtime::learning::LearningPrediction {
                artifact_id: Some(candidate.clone()),
                label: Some(if i % 2 == 0 { "pass" } else { "fail" }.into()),
                confidence: 0.95,
                probabilities: vec![],
            }),
            teacher: None,
            embedding: vec![],
            slices: Default::default(),
            sample: row,
        });
        reviews.push(LearningReview {
            sample_id: i.to_string(),
            annotation: json!({"kind":"class","class_id":i%2}),
            source: flow_like_ml_runtime::LabelSource::ObservedOutcome,
            reviewer: "gauge".into(),
            available_at_ms: at,
            revision: 1,
            outcome: None,
        });
    }
    // Canary replay rebuilds rows from their raw source, so Observe refuses rows it cannot replay.
    let mut sensor_time = observations[0].clone();
    sensor_time.sample_id = "488".into();
    sensor_time.group_id = "244".into();
    sensor_time.sample = source_rows(488..489).remove(0);
    let error = observe(
        &mut context,
        LearningObservationsRequest {
            project_id: id.clone(),
            observations: vec![observations[1].clone(), sensor_time],
        },
    )
    .await
    .err()
    .expect("an observation without a valid raw row must be rejected");
    assert!(error.to_string().contains("'488'"), "{error}");
    assert!(
        repo.learning_observations(&id)
            .unwrap()
            .iter()
            .all(|observation| observation.sample_id != observations[1].sample_id)
    );
    observe(
        &mut context,
        LearningObservationsRequest {
            project_id: id.clone(),
            observations,
        },
    )
    .await
    .unwrap();
    for review in &reviews {
        repo.record_learning_review(&id, review).unwrap();
    }
    if matches!(scenario, CanaryReviewScenario::Rejected) {
        let cohort = repo
            .learning_canary_sample_ids(&id, flow_like_ml_runtime::now_ms())
            .unwrap();
        assert!(!cohort.is_empty());
        let generation = repo.get_learning_project(&id).unwrap().generation;
        assert!(
            reject(
                &mut context,
                RejectLearningCandidateRequest {
                    project_id: id.clone(),
                    expected_generation: Some(generation - 1),
                },
            )
            .await
            .is_err()
        );
        let rejected = reject(
            &mut context,
            RejectLearningCandidateRequest {
                project_id: id.clone(),
                expected_generation: Some(generation),
            },
        )
        .await
        .unwrap();
        assert_eq!(rejected.project.state, LearningState::Active);
        assert_eq!(rejected.project.champion_artifact_id, Some(champion));
        assert!(rejected.project.candidate_artifact_id.is_none());
        assert!(
            rejected
                .comparison
                .unwrap()
                .reasons
                .contains(&"candidate_rejected".to_string())
        );
        assert!(
            !rejected
                .next_actions
                .blocking
                .contains(&"candidate_requires_promotion_or_rejection".to_string())
        );
        let sealed = repo
            .learning_dataset_exclusions(&id)
            .unwrap()
            .sealed_audit_sample_ids;
        assert!(cohort.iter().all(|sample_id| sealed.contains(sample_id)));
        return;
    }
    if !matches!(scenario, CanaryReviewScenario::Unchanged) {
        let project = repo.get_learning_project(&id).unwrap();
        inference::audit_canary(&repo, &project, &CancelFlag::default()).unwrap();
        let predictions = json!(repo.predictions(&candidate).unwrap());
        let cohort = repo
            .learning_canary_sample_ids(&id, flow_like_ml_runtime::now_ms())
            .unwrap();
        assert!(!cohort.is_empty());
        let mut corrected = repo
            .learning_training_reviews(&id)
            .unwrap()
            .into_iter()
            .find(|review| review.sample_id == cohort[0])
            .unwrap();
        corrected.revision += 1;
        corrected.available_at_ms = flow_like_ml_runtime::now_ms();
        corrected.annotation["class_id"] =
            json!(1 - corrected.annotation["class_id"].as_u64().unwrap());
        repo.record_learning_review(&id, &corrected).unwrap();

        let rejected = match scenario {
            CanaryReviewScenario::CorrectedManual => promote(
                &mut context,
                LearningProjectId {
                    project_id: id.clone(),
                },
            )
            .await
            .unwrap(),
            _ => finish_learning_cycle(&mut context, &id).await.unwrap(),
        };
        // A promotion the corrected review turned down still republishes the champion.
        let pointer = champion_pointer(&_directory, &id);
        assert_eq!(pointer.artifact_id.as_deref(), Some(champion.as_str()));
        assert_eq!(pointer.project_generation, rejected.project.generation);
        assert_eq!(rejected.project.state, LearningState::Active);
        assert_eq!(rejected.project.champion_artifact_id, Some(champion));
        assert!(rejected.project.candidate_artifact_id.is_none());
        let comparison = rejected.comparison.unwrap();
        assert!(!comparison.eligible);
        assert!(comparison.reasons.contains(&"canary_review_changed".into()));
        assert_eq!(json!(repo.predictions(&candidate).unwrap()), predictions);
        return;
    }

    let promoted = step(
        &mut context,
        LearningProjectId {
            project_id: id.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(promoted.project.state, LearningState::Active);
    assert_eq!(
        promoted.project.champion_artifact_id,
        Some(candidate.clone())
    );
    assert_eq!(
        champion_pointer(&_directory, &id).artifact_id,
        Some(candidate.clone())
    );
    let deployment = repo
        .get_deployment(&second.project.request.deployment_id)
        .unwrap();
    assert_eq!(deployment.previous_artifact_id, Some(champion.clone()));
    // Boards saved before the rollback expectations send only the project.
    let legacy: RollbackLearningModelRequest =
        flow_like_types::json::from_value(json!({"project_id": id})).unwrap();
    let rolled = rollback(&mut context, legacy.clone()).await.unwrap();
    assert_eq!(rolled.project.champion_artifact_id, Some(champion.clone()));
    let pointer = champion_pointer(&_directory, &id);
    assert_eq!(pointer.artifact_id.as_deref(), Some(champion.as_str()));
    assert_eq!(pointer.retained.first(), Some(&candidate));
    // Retried rollbacks must not restore the model they rolled back.
    let retried = rollback(&mut context, legacy).await.unwrap();
    assert_eq!(retried.project.champion_artifact_id, Some(champion.clone()));
    let named = RollbackLearningModelRequest {
        project_id: id.clone(),
        expected_artifact_id: Some(candidate.clone()),
        expected_generation: None,
    };
    let retried = rollback(&mut context, named).await.unwrap();
    assert_eq!(retried.project.champion_artifact_id, Some(champion.clone()));
    assert_eq!(
        repo.get_deployment(&second.project.request.deployment_id)
            .unwrap()
            .active_artifact_id,
        champion
    );
    assert!(
        rollback(
            &mut context,
            RollbackLearningModelRequest {
                project_id: id.clone(),
                expected_artifact_id: None,
                expected_generation: Some(deployment.generation),
            },
        )
        .await
        .is_err()
    );
    assert!(
        rollback(
            &mut context,
            RollbackLearningModelRequest {
                project_id: id.clone(),
                expected_artifact_id: Some(champion.clone()),
                expected_generation: Some(deployment.generation + 1),
            },
        )
        .await
        .is_err()
    );
    let paused = pause(
        &mut context,
        LearningProjectId {
            project_id: id.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(paused.project.state, LearningState::Paused);
    assert!(matches!(
        repo.learning_route(&id, "part-1").unwrap(),
        LearningRoute::Teacher
    ));
    let pointer = champion_pointer(&_directory, &id);
    assert!(pointer.paused);
    assert_eq!(pointer.project_generation, paused.project.generation);
    let resumed = resume(
        &mut context,
        LearningProjectId {
            project_id: id.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(resumed.project.state, LearningState::Active);
    let pointer = champion_pointer(&_directory, &id);
    assert!(!pointer.paused);
    assert_eq!(pointer.artifact_id.as_deref(), Some(champion.as_str()));
}
