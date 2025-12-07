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
        registry
            .push_nodes(catalog)
            .await
            .expect("register catalog");
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
    // Disable debug logging for benchmarks
    board.log_level = LogLevel::Error;
    board
}

fn create_intercom() -> Arc<BufferedInterComHandler> {
    BufferedInterComHandler::new(
        Arc::new(move |_event| Box::pin(async move { Ok(()) })),
        Some(100),
        Some(400),
        Some(false), // Disable background check to avoid spawning tasks outside runtime
    )
}

async fn run_once(
    board: Arc<Board>,
    state: Arc<FlowLikeState>,
    profile: &Profile,
    start: &str,
    intercom: Arc<BufferedInterComHandler>,
) {
    let payload = RunPayload {
        id: start.to_string(),
        payload: None,
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
    let intercom = create_intercom();

    // Warmup
    rt.block_on(async {
        for _ in 0..100 {
            run_once(
                board.clone(),
                state.clone(),
                &profile,
                &start,
                intercom.clone(),
            )
            .await;
        }
    });

    // Use consistent group name for baseline comparison
    let mut group = c.benchmark_group("allocator_comparison");
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    // Single-threaded throughput (pure execution speed, no contention)
    group.throughput(Throughput::Elements(1));
    group.bench_function(format!("{}/single_exec", allocator_name()), |b| {
        let intercom = intercom.clone();
        b.to_async(&rt).iter(|| {
            let st = state.clone();
            let brd = board.clone();
            let pr = profile.clone();
            let sid = start.clone();
            let ic = intercom.clone();
            async move {
                run_once(brd, st, &pr, &sid, ic).await;
            }
        });
    });

    // Concurrent throughput - measures contention impact
    // Note: Low scaling indicates mutex contention in FlowLikeState
    let optimal_concurrency = num_cpus::get() * 2;
    group.throughput(Throughput::Elements(optimal_concurrency as u64));
    group.bench_with_input(
        BenchmarkId::new(
            format!("{}/concurrent_exec", allocator_name()),
            optimal_concurrency,
        ),
        &optimal_concurrency,
        |b, &conc| {
            let intercom = intercom.clone();
            b.to_async(&rt).iter_custom(|iters| {
                let state = state.clone();
                let board = board.clone();
                let profile = profile.clone();
                let start = start.clone();
                let intercom = intercom.clone();

                async move {
                    let begin = Instant::now();
                    for _ in 0..iters {
                        let mut tasks = Vec::with_capacity(conc);
                        for _ in 0..conc {
                            let st = state.clone();
                            let brd = board.clone();
                            let pr = profile.clone();
                            let sid = start.clone();
                            let ic = intercom.clone();
                            tasks.push(tokio::spawn(async move {
                                run_once(brd, st, &pr, &sid, ic).await;
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

    // Parallel independent states - true parallelism without shared state contention
    let parallel_count = num_cpus::get();

    // Pre-create states and boards outside the benchmark closure
    let parallel_states: Vec<_> = rt.block_on(async {
        let mut states = Vec::with_capacity(parallel_count);
        for _ in 0..parallel_count {
            states.push(default_state().await);
        }
        states
    });
    let parallel_boards: Vec<_> = rt.block_on(async {
        let mut boards = Vec::with_capacity(parallel_count);
        for s in &parallel_states {
            boards.push(Arc::new(open_board(&board_id(), s.clone()).await));
        }
        boards
    });
    let parallel_intercoms: Vec<_> = (0..parallel_count).map(|_| create_intercom()).collect();

    group.throughput(Throughput::Elements(parallel_count as u64));
    group.bench_with_input(
        BenchmarkId::new(
            format!("{}/parallel_independent", allocator_name()),
            parallel_count,
        ),
        &parallel_count,
        |b, &count| {
            b.to_async(&rt).iter_custom(|iters| {
                let states = parallel_states.clone();
                let boards = parallel_boards.clone();
                let profile = profile.clone();
                let start = start.clone();
                let intercoms = parallel_intercoms.clone();

                async move {
                    let begin = Instant::now();
                    for _ in 0..iters {
                        let mut tasks = Vec::with_capacity(count);
                        for i in 0..count {
                            let st = states[i].clone();
                            let brd = boards[i].clone();
                            let pr = profile.clone();
                            let sid = start.clone();
                            let ic = intercoms[i].clone();
                            tasks.push(tokio::spawn(async move {
                                run_once(brd, st, &pr, &sid, ic).await;
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

    // Max parallelism test - 2x CPU cores with independent states
    let max_parallel = num_cpus::get() * 2;

    let max_states: Vec<_> = rt.block_on(async {
        let mut states = Vec::with_capacity(max_parallel);
        for _ in 0..max_parallel {
            states.push(default_state().await);
        }
        states
    });
    let max_boards: Vec<_> = rt.block_on(async {
        let mut boards = Vec::with_capacity(max_parallel);
        for s in &max_states {
            boards.push(Arc::new(open_board(&board_id(), s.clone()).await));
        }
        boards
    });
    let max_intercoms: Vec<_> = (0..max_parallel).map(|_| create_intercom()).collect();

    group.throughput(Throughput::Elements(max_parallel as u64));
    group.bench_with_input(
        BenchmarkId::new(format!("{}/max_parallel", allocator_name()), max_parallel),
        &max_parallel,
        |b, &count| {
            b.to_async(&rt).iter_custom(|iters| {
                let states = max_states.clone();
                let boards = max_boards.clone();
                let profile = profile.clone();
                let start = start.clone();
                let intercoms = max_intercoms.clone();

                async move {
                    let begin = Instant::now();
                    for _ in 0..iters {
                        let mut tasks = Vec::with_capacity(count);
                        for i in 0..count {
                            let st = states[i].clone();
                            let brd = boards[i].clone();
                            let pr = profile.clone();
                            let sid = start.clone();
                            let ic = intercoms[i].clone();
                            tasks.push(tokio::spawn(async move {
                                run_once(brd, st, &pr, &sid, ic).await;
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

    group.finish();

    eprintln!(
        "\n[{}] Benchmark complete. Workers: {}, Max parallel: {}",
        allocator_name(),
        worker_threads,
        max_parallel
    );
}

criterion_group!(benches, allocator_throughput_bench);
criterion_main!(benches);
