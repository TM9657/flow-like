//! Allocator comparison benchmark for workflow execution throughput.
//!
//! This benchmark compares mimalloc vs the default system allocator to determine
//! which provides better performance for workflow executions.
//!
//! Run with:
//!   cargo bench --bench allocator_bench --features mimalloc
//!   cargo bench --bench allocator_bench  # (default allocator)
//!
//! Or compare both:
//!   cargo bench --bench allocator_bench -- --save-baseline default
//!   cargo bench --bench allocator_bench --features mimalloc -- --baseline default

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use flow_like::{
    flow::{
        board::Board,
        execution::{InternalRun, LogLevel, RunPayload},
    },
    num_cpus,
    profile::Profile,
    state::{FlowLikeConfig, FlowLikeState},
    utils::http::HTTPClient,
};
use flow_like_storage::{
    Path,
    files::store::{FlowLikeStore, local_store::LocalObjectStore},
};
use flow_like_types::{intercom::BufferedInterComHandler, tokio};
use std::collections::HashMap;
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

fn env_or<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_or_string(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn board_id() -> String {
    env_or_string("FL_BOARD_ID", "o4wqrpzkx1cp4svxe91yordw")
}
fn start_id() -> String {
    env_or_string("FL_START_ID", "ek4tee4s3nufw3drfnwd20hw")
}
fn app_id() -> String {
    env_or_string("FL_APP_ID", "q99s8hb4z56mpwz8dscz7qmz")
}
fn tests_dir() -> PathBuf {
    PathBuf::from(env_or_string("FL_TESTS_DIR", "../../tests"))
}

async fn default_state() -> Arc<FlowLikeState> {
    let mut config: FlowLikeConfig = FlowLikeConfig::new();
    let store = LocalObjectStore::new(tests_dir()).expect("LocalObjectStore");
    let store = FlowLikeStore::Local(Arc::new(store));
    config.register_bits_store(store.clone());
    config.register_user_store(store.clone());
    config.register_app_storage_store(store.clone());
    config.register_app_meta_store(store);

    let (http_client, _refetch_rx) = HTTPClient::new();
    let state = FlowLikeState::new(config, http_client);
    let state_ref = Arc::new(state);
    let weak_ref = Arc::downgrade(&state_ref);
    let catalog = flow_like_catalog::get_catalog();

    {
        let registry_guard = state_ref.node_registry.clone();
        let mut registry = registry_guard.write().await;
        registry.initialize(weak_ref);
        registry.push_nodes(catalog);
    }
    state_ref
}

fn construct_profile() -> Profile {
    Profile::default()
}

async fn open_board(id: &str, state: Arc<FlowLikeState>) -> Board {
    let path = Path::from("flow").child(&*app_id());
    let mut board = Board::load(path, id, state, None)
        .await
        .expect("load board");
    // Suppress ALL internal logging during benchmarks for maximum performance
    board.log_level = LogLevel::Fatal;
    board
}

fn create_intercom() -> Arc<BufferedInterComHandler> {
    BufferedInterComHandler::new(
        Arc::new(move |_event| Box::pin(async move { Ok(()) })),
        Some(100),
        Some(400),
        Some(true), // Enable background flush for consistent behavior
    )
}

async fn run_once(board: Arc<Board>, state: Arc<FlowLikeState>, profile: &Profile, start: &str) {
    let intercom = create_intercom();
    let payload = RunPayload {
        id: start.to_string(),
        payload: None,
        runtime_variables: None,
        filter_secrets: Some(true)
    };
    let mut run = InternalRun::new(
        "bench",
        board,
        None,
        &state,
        profile,
        &payload,
        false,
        intercom.into_callback(),
        None,
        None,
        HashMap::new(),
    )
    .await
    .expect("InternalRun::new");
    run.execute(state).await;
}

fn allocator_name() -> &'static str {
    #[cfg(feature = "mimalloc")]
    {
        "mimalloc"
    }
    #[cfg(not(feature = "mimalloc"))]
    {
        "system"
    }
}

fn allocator_throughput_bench(c: &mut Criterion) {
    let worker_threads = env_or("FL_WORKER_THREADS", num_cpus::get());
    let max_blocking = env_or("FL_MAX_BLOCKING_THREADS", worker_threads * 4);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .max_blocking_threads(max_blocking)
        .enable_all()
        .build()
        .expect("tokio runtime");

    let state = rt.block_on(default_state());
    let board = Arc::new(rt.block_on(open_board(&board_id(), state.clone())));
    let profile = construct_profile();
    let start = start_id();

    // Warmup
    rt.block_on(async {
        for _ in 0..100 {
            run_once(board.clone(), state.clone(), &profile, &start).await;
        }
    });

    // Use consistent group name for baseline comparison
    let mut group = c.benchmark_group("allocator_comparison");
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(20);

    // Single-threaded throughput (pure execution speed, no contention)
    group.throughput(Throughput::Elements(1));
    group.bench_with_input(
        BenchmarkId::new(format!("{}/single_exec", allocator_name()), 1),
        &1,
        |b, _| {
            b.to_async(&rt).iter_custom(|iters| {
                let state = state.clone();
                let board = board.clone();
                let profile = profile.clone();
                let start = start.clone();

                async move {
                    let begin = Instant::now();
                    for _ in 0..iters {
                        run_once(board.clone(), state.clone(), &profile, &start).await;
                    }
                    begin.elapsed()
                }
            });
        },
    );

    // Concurrent throughput - shared state (like production use)
    // Tests high concurrency levels for peak throughput
    for &conc in &[256usize, 512, 1024] {
        group.throughput(Throughput::Elements(conc as u64));
        group.bench_with_input(
            BenchmarkId::new(format!("{}/concurrent", allocator_name()), conc),
            &conc,
            |b, &conc| {
                b.to_async(&rt).iter_custom(|iters| {
                    let state = state.clone();
                    let board = board.clone();
                    let profile = profile.clone();
                    let start = start.clone();

                    async move {
                        let begin = Instant::now();
                        for _ in 0..iters {
                            let mut tasks = Vec::with_capacity(conc);
                            for _ in 0..conc {
                                let st = state.clone();
                                let brd = board.clone();
                                let pr = profile.clone();
                                let sid = start.clone();
                                tasks.push(tokio::spawn(async move {
                                    run_once(brd, st, &pr, &sid).await;
                                }));
                            }
                            for t in tasks {
                                let _ = t.await;
                            }
                        }
                        begin.elapsed()
                    }
                });
            },
        );
    }

    group.finish();

    eprintln!(
        "\n[{}] Benchmark complete. Workers: {}",
        allocator_name(),
        worker_threads
    );
}

criterion_group!(benches, allocator_throughput_bench);
criterion_main!(benches);
