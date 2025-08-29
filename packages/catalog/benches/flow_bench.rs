use criterion::{Criterion, criterion_group, criterion_main};
use flow_like::{
    flow::{
        board::Board,
        execution::{InternalRun, RunPayload},
    },
    profile::Profile,
    state::{FlowLikeConfig, FlowLikeState},
    utils::http::HTTPClient,
};
use flow_like_storage::{
    Path,
    files::store::{FlowLikeStore, local_store::LocalObjectStore},
};
use flow_like_types::{sync::Mutex, tokio};
use std::{path::PathBuf, sync::Arc};

const BOARD_1: &str = "o4wqrpzkx1cp4svxe91yordw";
const BOARD_2: &str = "o4wqrpzkx1cp4svxe91yordw";
const START_1: &str = "ek4tee4s3nufw3drfnwd20hw";
const START_2: &str = "ek4tee4s3nufw3drfnwd20hw";

async fn default_state() -> Arc<Mutex<FlowLikeState>> {
    let mut config: FlowLikeConfig = FlowLikeConfig::new();
    let store = LocalObjectStore::new(PathBuf::from("../../tests")).unwrap();
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
        registry.push_nodes(catalog).await.unwrap();
    }
    state_ref
}

fn construct_profile() -> Profile {
    Profile {
        ..Default::default()
    }
}

async fn open_board(id: &str, state: Arc<Mutex<FlowLikeState>>) -> Board {
    let path = Path::from("flow").child("q99s8hb4z56mpwz8dscz7qmz");
    Board::load(path, id, state, None).await.unwrap()
}

async fn run_board(id: &str, start_ids: Vec<String>) {
    let state = default_state().await;
    let board = Arc::new(open_board(id, state.clone()).await);
    let profile = construct_profile();

    for start_id in &start_ids {
        let payload = RunPayload {
            id: start_id.clone(),
            payload: None,
        };
        let mut run = InternalRun::new(
            "bench",
            board.clone(),
            None,
            &state,
            &profile,
            &payload,
            None,
            false,
            None,
            None,
        )
        .await
        .unwrap();
        run.execute(state.clone()).await;
    }
}

async fn run_shared_board(
    board: Arc<Board>,
    state: Arc<Mutex<FlowLikeState>>,
    profile: Profile,
    start_ids: Vec<String>,
) {
    for start_id in &start_ids {
        let payload = RunPayload {
            id: start_id.clone(),
            payload: None,
        };
        let mut run = InternalRun::new(
            "bench",
            board.clone(),
            None,
            &state,
            &profile,
            &payload,
            None,
            false,
            None,
            None,
        )
        .await
        .unwrap();
        run.execute(state.clone()).await;
    }
}

fn get_memory_usage() -> f64 {
    // This uses macOS-specific command
    let output = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .expect("Failed to execute ps command");

    let rss = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<f64>()
        .unwrap();
    rss / 1024.0 // Convert to MB
}

fn criterion_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let memory_start = get_memory_usage();
    #[allow(unused_assignments)]
    let mut memory_mid = 0.0;
    #[allow(unused_assignments)]
    let mut memory_end = 0.0;
    #[allow(unused_assignments)]
    let mut shared_memory_mid = 0.0;
    #[allow(unused_assignments)]
    let mut shared_memory_end = 0.0;

    {
        let mut group = c.benchmark_group("board_1");
        group.sample_size(1_000);
        group.bench_function("run", |b| {
            b.to_async(&rt).iter_with_large_drop(|| async {
                run_board(BOARD_1, vec![START_1.to_string()]).await
            });
        });
        memory_mid = get_memory_usage();
        group.finish();
    }

    println!("Memory difference: {} MB", memory_mid - memory_start);

    {
        let mut group = c.benchmark_group("board_2");
        group.sample_size(1_000);
        group.bench_function("run", |b| {
            b.to_async(&rt).iter_with_large_drop(|| async {
                run_board(BOARD_2, vec![START_2.to_string()]).await
            });
        });
        memory_end = get_memory_usage();
        group.finish();
    }

    println!("\n--- Shared Execution Benchmarks ---");
    let state = rt.block_on(default_state());
    let board1 = Arc::new(rt.block_on(open_board(BOARD_1, state.clone())));
    let board2 = Arc::new(rt.block_on(open_board(BOARD_2, state.clone())));
    let profile = construct_profile();

    let shared_memory_start = get_memory_usage();

    {
        let mut group = c.benchmark_group("board_shared_1");
        group.sample_size(100_000);
        group.bench_function("run_shared_board_1", |b| {
            b.to_async(&rt).iter_with_large_drop(|| async {
                run_shared_board(
                    board1.clone(),
                    state.clone(),
                    profile.clone(),
                    vec![START_1.to_string()],
                )
                .await
            });
        });
        shared_memory_mid = get_memory_usage();
        group.finish();
    }

    drop(board1);

    {
        let mut group = c.benchmark_group("board_shared_2");
        group.sample_size(100_000);
        group.bench_function("run_shared_board_2", |b| {
            b.to_async(&rt).iter_with_large_drop(|| async {
                run_shared_board(
                    board2.clone(),
                    state.clone(),
                    profile.clone(),
                    vec![START_2.to_string()],
                )
                .await
            });
        });
        shared_memory_end = get_memory_usage();
        group.finish();
    }

    println!(
        "Memory difference: {} MB (Mid: {})",
        memory_end - memory_start,
        memory_mid - memory_start
    );
    println!(
        "Shared Memory difference: {} MB (Mid: {})",
        shared_memory_end - shared_memory_start,
        shared_memory_mid - shared_memory_start
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
