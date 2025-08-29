use criterion::{
    criterion_group, criterion_main, BenchmarkId, Criterion, Throughput,
};
use flow_like::{
    flow::{
        board::Board,
        execution::{InternalRun, RunPayload},
    }, num_cpus, profile::Profile, state::{FlowLikeConfig, FlowLikeState}, utils::http::HTTPClient
};
use flow_like_storage::{
    Path,
    files::store::{FlowLikeStore, local_store::LocalObjectStore},
};
use flow_like_types::{sync::Mutex, tokio};
use std::{path::PathBuf, sync::Arc, time::{Duration, Instant}};

/// -------- CONFIG (override via env) -----------------------------------------
fn env_or<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}
fn env_or_string(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn concurrency_list() -> Vec<usize> {
    // e.g. FL_CONCURRENCY_LIST=1,2,4,8,16,32 or leave unset to sweep powers of 2
    if let Ok(raw) = std::env::var("FL_CONCURRENCY_LIST") {
        return raw
            .split(',')
            .filter_map(|s| s.trim().parse::<usize>().ok())
            .filter(|&n| n > 0)
            .collect();
    }
    let max = env_or("FL_MAX_CONCURRENCY", num_cpus::get() * 8);
    let mut v = Vec::new();
    let mut n = 1;
    while n <= max { v.push(n); n <<= 1; }
    v
}

/// -------- Your IDs; override with env if needed -----------------------------
fn board_id() -> String { env_or_string("FL_BOARD_ID", "o4wqrpzkx1cp4svxe91yordw") }
fn start_id() -> String { env_or_string("FL_START_ID", "ek4tee4s3nufw3drfnwd20hw") }
fn app_id() -> String { env_or_string("FL_APP_ID", "q99s8hb4z56mpwz8dscz7qmz") }
fn tests_dir() -> PathBuf { PathBuf::from(env_or_string("FL_TESTS_DIR", "../../tests")) }

/// -------- Engine bootstrap (reused across runs) -----------------------------
async fn default_state() -> Arc<Mutex<FlowLikeState>> {
    let mut config: FlowLikeConfig = FlowLikeConfig::new();
    let store = LocalObjectStore::new(tests_dir()).expect("LocalObjectStore");
    let store = FlowLikeStore::Local(Arc::new(store));
    config.register_bits_store(store.clone());
    config.register_user_store(store.clone());
    config.register_app_storage_store(store.clone());
    config.register_app_meta_store(store);

    let (http_client, _refetch_rx) = HTTPClient::new();
    let state = FlowLikeState::new(config, http_client);
    let state_ref = Arc::new(Mutex::new(state));
    let weak_ref = Arc::downgrade(&state_ref);
    let catalog = flow_like_catalog::get_catalog().await;

    {
        let state = state_ref.lock().await;
        let registry_guard = state.node_registry.clone();
        drop(state);
        let mut registry = registry_guard.write().await;
        registry.initialize(weak_ref);
        registry.push_nodes(catalog).await.expect("register catalog");
    }
    state_ref
}

fn construct_profile() -> Profile {
    Profile { ..Default::default() }
}

async fn open_board(id: &str, state: Arc<Mutex<FlowLikeState>>) -> Board {
    let path = Path::from("flow").child(&*app_id());
    Board::load(path, id, state, None).await.expect("load board")
}

/// Run a single execution (from a Start node) on an already-open board/state.
async fn run_once(
    board: Arc<Board>,
    state: Arc<Mutex<FlowLikeState>>,
    profile: &Profile,
    start: &str,
) {
    let payload = RunPayload { id: start.to_string(), payload: None };
    let mut run = InternalRun::new(
        "bench",
        board,
        None,
        &state,
        profile,
        &payload,
        None,
        false,
        None,
        None,
    ).await.expect("InternalRun::new");
    run.execute(state).await;
}

/// -------- Throughput benchmark ---------------------------------------------
fn throughput_bench(c: &mut Criterion) {
    // Multi-thread runtime pinned to CPU count (tweak via FL_WORKER_THREADS)
    let worker_threads = env_or("FL_WORKER_THREADS", num_cpus::get());
    let max_blocking = env_or("FL_MAX_BLOCKING_THREADS", worker_threads * 4);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .max_blocking_threads(max_blocking)
        .enable_all()
        .build()
        .expect("tokio runtime");

    // Preload state/board/profile ONCE to measure pure execution throughput.
    let state = rt.block_on(default_state());
    let board = Arc::new(rt.block_on(open_board(&board_id(), state.clone())));
    let profile = construct_profile();
    let start = start_id();

    // Warm up the engine/JIT/caches a bit outside of measurement.
    rt.block_on(async {
        for _ in 0..64 {
            run_once(board.clone(), state.clone(), &profile, &start).await;
        }
    });

    let mut group = c.benchmark_group("flow_like_throughput");
    group.warm_up_time(Duration::from_secs(env_or("FL_WARMUP_SECS", 3)));
    group.measurement_time(Duration::from_secs(env_or("FL_MEASURE_SECS", 15)));
    group.sample_size(env_or("FL_SAMPLE_SIZE", 12u64) as usize);

    let concs = concurrency_list();
    for &concurrency in &concs {
        group.throughput(Throughput::Elements(concurrency as u64));
        group.bench_with_input(
            BenchmarkId::new("exec_from_start", concurrency),
            &concurrency,
            |b, &conc| {
                b.to_async(&rt).iter_custom(|iters| {
                    let state = state.clone();
                    let board = board.clone();
                    let profile = profile.clone();
                    let start = start.clone();

                    async move {
                        // Each "iteration" launches `conc` executions concurrently.
                        // Total ops = iters * conc; Criterion uses throughput to show ops/sec.
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
                                // If your engine never panics, ignoring JoinError is ok; else unwrap.
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

    // Optional: print the tested concurrencies so you can spot your peak easily in output
    eprintln!("[flow_like_throughput] tested concurrencies: {:?}", concs);
}

criterion_group!(benches, throughput_bench);
criterion_main!(benches);
