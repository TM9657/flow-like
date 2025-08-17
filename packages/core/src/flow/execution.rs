use super::board::ExecutionStage;
use super::event::Event;
use super::{board::Board, node::NodeState, variable::Variable};
use crate::credentials::SharedCredentials;
use crate::profile::Profile;
use crate::state::FlowLikeState;
use ahash::AHasher;
use context::ExecutionContext;
use flow_like_storage::arrow_array::{RecordBatch, RecordBatchIterator};
use flow_like_storage::arrow_schema::FieldRef;
use flow_like_storage::files::store::FlowLikeStore;
use flow_like_storage::lancedb::Connection;
use flow_like_storage::lancedb::index::scalar::BitmapIndexBuilder;
use flow_like_storage::serde_arrow::schema::{SchemaLike, TracingOptions};
use flow_like_storage::{Path, serde_arrow};
use flow_like_types::Value;
use flow_like_types::intercom::InterComCallback;
use flow_like_types::json::to_vec;
use flow_like_types::sync::{DashMap, Mutex, RwLock};
use flow_like_types::{Cacheable, anyhow, create_id};
use futures::StreamExt;
use futures::future::BoxFuture;
use internal_node::InternalNode;
use internal_pin::InternalPin;
use log::LogMessage;
use num_cpus;
use once_cell::sync::Lazy;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    sync::{Arc, Weak},
    time::SystemTime,
};
use trace::Trace;

pub mod context;
pub mod internal_node;
pub mod internal_pin;
pub mod log;
pub mod trace;

const USE_DEPENDENCY_GRAPH: bool = false;
static STORED_META_FIELDS: Lazy<Vec<FieldRef>> = Lazy::new(|| {
    Vec::<FieldRef>::from_type::<LogMeta>(
        TracingOptions::default()
            .allow_null_fields(true)
            .strings_as_large_utf8(false),
    )
    .expect("derive FieldRef for StoredLogMessage")
});

#[derive(
    Serialize, Deserialize, JsonSchema, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord,
)]
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
    Fatal = 4,
}

impl LogLevel {
    pub fn from_u32(value: u32) -> Self {
        match value {
            0 => LogLevel::Debug,
            1 => LogLevel::Info,
            2 => LogLevel::Warn,
            3 => LogLevel::Error,
            4 => LogLevel::Fatal,
            _ => LogLevel::Debug,
        }
    }

    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => LogLevel::Debug,
            1 => LogLevel::Info,
            2 => LogLevel::Warn,
            3 => LogLevel::Error,
            4 => LogLevel::Fatal,
            _ => LogLevel::Debug,
        }
    }

    pub fn to_u32(self) -> u32 {
        match self {
            LogLevel::Debug => 0,
            LogLevel::Info => 1,
            LogLevel::Warn => 2,
            LogLevel::Error => 3,
            LogLevel::Fatal => 4,
        }
    }

    pub fn to_u8(self) -> u8 {
        match self {
            LogLevel::Debug => 0,
            LogLevel::Info => 1,
            LogLevel::Warn => 2,
            LogLevel::Error => 3,
            LogLevel::Fatal => 4,
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct LogMeta {
    pub app_id: String,
    pub run_id: String,
    pub board_id: String,
    pub start: u64,
    pub end: u64,
    pub log_level: u8,
    pub version: String,
    pub nodes: Option<Vec<(String, u8)>>,
    pub logs: Option<u64>,
    pub node_id: String,
    pub event_version: Option<String>,
    pub event_id: String,
    pub payload: Vec<u8>,
}

impl LogMeta {
    fn into_arrow(&self) -> flow_like_types::Result<RecordBatch> {
        let fields = &*STORED_META_FIELDS;
        let batch = serde_arrow::to_record_batch(fields, &vec![self])?;
        Ok(batch)
    }

    pub fn into_duckdb_types() -> String {
        let fields = &*STORED_META_FIELDS;
        let mut types = vec![];

        for field in fields {
            let field_type = match field.data_type() {
                flow_like_storage::arrow_schema::DataType::Utf8 => "TEXT",
                flow_like_storage::arrow_schema::DataType::UInt64 => "INTEGER",
                flow_like_storage::arrow_schema::DataType::Int64 => "INTEGER",
                flow_like_storage::arrow_schema::DataType::Boolean => "BOOLEAN",
                _ => "TEXT",
            };
            types.push(format!("{} {}", field.name(), field_type));
        }

        types.join(", ")
    }

    pub async fn flush(&self, db: Connection) -> flow_like_types::Result<()> {
        let arrow_batch = self.into_arrow()?;
        let schema = arrow_batch.schema();

        let table = db.open_table("runs").execute().await;

        if let Err(_err) = table {
            let table = db
                .create_empty_table("runs", schema.clone())
                .execute()
                .await?;
            let iter = RecordBatchIterator::new(vec![arrow_batch].into_iter().map(Ok), schema);
            table.add(iter).execute().await?;
            table
                .create_index(
                    &["event_id"],
                    flow_like_storage::lancedb::index::Index::Bitmap(BitmapIndexBuilder {}),
                )
                .execute()
                .await?;
            table
                .create_index(
                    &["node_id"],
                    flow_like_storage::lancedb::index::Index::Bitmap(BitmapIndexBuilder {}),
                )
                .execute()
                .await?;
            table
                .create_index(
                    &["log_level"],
                    flow_like_storage::lancedb::index::Index::Bitmap(BitmapIndexBuilder {}),
                )
                .execute()
                .await?;
            table
                .create_index(
                    &["start"],
                    flow_like_storage::lancedb::index::Index::BTree(
                        flow_like_storage::lancedb::index::scalar::BTreeIndexBuilder {},
                    ),
                )
                .execute()
                .await?;
            return Ok(());
        }
        let table = table?;
        let iter = RecordBatchIterator::new(vec![arrow_batch].into_iter().map(Ok), schema);
        table.add(iter).execute().await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct Run {
    pub id: String,
    pub app_id: String,
    pub traces: Vec<Trace>,
    pub status: RunStatus,
    pub start: SystemTime,
    pub end: SystemTime,
    pub board: Arc<Board>,
    pub log_level: LogLevel,
    pub payload: Arc<RunPayload>,
    pub sub: String,
    pub highest_log_level: LogLevel,
    pub log_initialized: bool,
    pub logs: u64,
    pub stream_state: bool,

    pub event_id: Option<String>,
    pub event_version: Option<String>,

    pub visited_nodes: HashMap<String, LogLevel>,
    pub log_store: Option<FlowLikeStore>,
    pub log_db: Option<
        Arc<dyn Fn(Path) -> flow_like_storage::lancedb::connection::ConnectBuilder + Send + Sync>,
    >,
}

impl Run {
    pub async fn flush_logs(&mut self, finalize: bool) -> flow_like_types::Result<Option<LogMeta>> {
        let db_fn = self
            .log_db
            .as_ref()
            .ok_or_else(|| anyhow!("No log database configured"))?;
        let base_path = Path::from("runs")
            .child(self.app_id.clone())
            .child(self.board.id.clone());
        println!("Flushing logs to {}", base_path);
        let db = db_fn(base_path.clone()).execute().await?;

        // 1) pre‑count total logs, reserve once, and find highest level in one pass
        let total = self.traces.iter().map(|t| t.logs.len()).sum();
        let mut logs = Vec::with_capacity(total);
        let mut highest = self.highest_log_level;
        for trace in self.traces.drain(..) {
            let node_level = self
                .visited_nodes
                .entry(trace.node_id.clone())
                .or_insert(LogLevel::Debug);

            for log in trace.logs {
                let lvl = log.log_level;

                if lvl > highest {
                    highest = lvl;
                }

                if lvl > *node_level {
                    *node_level = lvl;
                }

                logs.push(log);
            }
        }
        self.logs = self.logs.saturating_add(logs.len() as u64);
        self.highest_log_level = highest;

        // 2) write arrow batches
        let arrow_batch = LogMessage::into_arrow(logs)?;
        let schema = arrow_batch.schema();
        let table = if self.log_initialized {
            db.open_table(&self.id).execute().await?
        } else {
            let t = db
                .create_empty_table(&self.id, schema.clone())
                .execute()
                .await?;
            self.log_initialized = true;
            t
        };
        let iter = RecordBatchIterator::new(vec![arrow_batch].into_iter().map(Ok), schema);
        table.add(iter).execute().await?;

        if !finalize {
            return Ok(None);
        }

        // 3) write meta file
        let vs = &self.board.version;
        let version_string = format!("v{}-{}-{}", vs.0, vs.1, vs.2);
        let start_micros = self
            .start
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_micros()
            .try_into()
            .map_err(|_| anyhow!("start timestamp overflowed u64"))?;
        let end_micros = self
            .end
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_micros()
            .try_into()
            .map_err(|_| anyhow!("end timestamp overflowed u64"))?;
        let payload =
            to_vec(&self.payload.payload.clone().unwrap_or(Value::Null)).unwrap_or_default();
        let visited_nodes = self
            .visited_nodes
            .drain()
            .map(|(k, v)| (k, v.to_u8()))
            .collect::<Vec<(String, u8)>>();

        let content = LogMeta {
            app_id: self.app_id.clone(),
            run_id: self.id.clone(),
            board_id: self.board.id.clone(),
            start: start_micros,
            end: end_micros,
            log_level: self.highest_log_level.to_u8(),
            version: version_string,
            nodes: Some(visited_nodes),
            logs: Some(self.logs),
            node_id: self.payload.id.clone(),
            event_id: self.event_id.clone().unwrap_or("".to_string()),
            event_version: self.event_version.clone(),
            payload,
        };

        Ok(Some(content))
    }
}

#[derive(Clone)]
struct RunStack {
    stack: Vec<Arc<InternalNode>>,
    deduplication: HashSet<u64>,
    hash: u64,
}

impl RunStack {
    fn with_capacity(capacity: usize) -> Self {
        RunStack {
            stack: Vec::with_capacity(capacity),
            deduplication: HashSet::with_capacity(capacity),
            hash: 0u64,
        }
    }

    fn push(&mut self, node_id: &str, node: Arc<InternalNode>) {
        let mut hasher = AHasher::default();
        node_id.hash(&mut hasher);
        let hash = hasher.finish();

        if self.deduplication.contains(&hash) {
            return;
        }

        self.deduplication.insert(hash);
        self.hash ^= hash;
        self.stack.push(node);
    }

    #[inline]
    fn hash(&self) -> u64 {
        self.hash
    }

    #[inline]
    fn len(&self) -> usize {
        self.stack.len()
    }
}

pub type EventTrigger =
    Arc<dyn Fn(&InternalRun) -> BoxFuture<'_, flow_like_types::Result<()>> + Send + Sync>;

#[derive(Clone)]
pub struct InternalRun {
    pub run: Arc<Mutex<Run>>,
    pub nodes: Arc<HashMap<String, Arc<InternalNode>>>,
    pub dependencies: HashMap<String, Vec<Arc<InternalNode>>>,
    pub pins: HashMap<String, Arc<Mutex<InternalPin>>>,
    pub variables: Arc<Mutex<HashMap<String, Variable>>>,
    pub cache: Arc<RwLock<HashMap<String, Arc<dyn Cacheable>>>>,
    pub profile: Arc<Profile>,
    pub callback: InterComCallback,
    pub credentials: Option<Arc<SharedCredentials>>,

    stack: Arc<RunStack>,
    concurrency_limit: u64,
    concurrency_map: Arc<DashMap<String, u64>>,
    cpus: usize,
    log_level: LogLevel,
    completion_callbacks: Arc<RwLock<Vec<EventTrigger>>>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct RunPayload {
    pub id: String,
    pub payload: Option<Value>,
}

impl InternalRun {
    pub async fn new(
        app_id: &str,
        board: Arc<Board>,
        event: Option<Event>,
        handler: &Arc<Mutex<FlowLikeState>>,
        profile: &Profile,
        payload: &RunPayload,
        sub: Option<String>,
        stream_state: bool,
        callback: InterComCallback,
        credentials: Option<SharedCredentials>,
    ) -> flow_like_types::Result<Self> {
        let before = Instant::now();
        let run_id = create_id();

        let (log_store, db) = {
            let state = handler.lock().await;
            let guard = state.config.read().await;
            let log_store = guard.stores.log_store.clone();
            let db = guard.callbacks.build_logs_database.clone();
            (log_store, db)
        };

        let run = Run {
            id: run_id.clone(),
            app_id: app_id.to_string(),
            traces: vec![],
            status: RunStatus::Running,
            start: SystemTime::now(),
            end: SystemTime::now(),
            log_level: board.log_level,
            board: board.clone(),
            payload: Arc::new(payload.clone()),
            sub: sub.unwrap_or_else(|| "local".to_string()),
            highest_log_level: LogLevel::Debug,
            log_initialized: false,
            logs: 0,
            stream_state,

            event_id: event.as_ref().map(|e| e.id.clone()),
            event_version: event.as_ref().map(|e| {
                let (major, minor, patch) = e.event_version;
                format!("{}.{}.{}", major, minor, patch)
            }),

            visited_nodes: HashMap::with_capacity(board.nodes.len()),
            log_store,
            log_db: db,
        };

        let run = Arc::new(Mutex::new(run));

        let mut dependencies = HashMap::with_capacity(board.nodes.len());

        let event_variables = event
            .as_ref()
            .map(|e| e.variables.clone())
            .unwrap_or_default();

        let variables = Arc::new(Mutex::new({
            let mut map = HashMap::with_capacity(board.variables.len());
            for (variable_id, board_variable) in &board.variables {
                let variable = if board_variable.exposed {
                    event_variables.get(variable_id).unwrap_or(board_variable)
                } else {
                    board_variable
                };

                let value = match &variable.default_value {
                    Some(bytes) => {
                        flow_like_types::json::from_slice::<Value>(bytes).unwrap_or(Value::Null)
                    }
                    None => Value::Null,
                };

                let mut var = variable.clone();
                var.value = Arc::new(Mutex::new(value));
                map.insert(variable_id.clone(), var);
            }
            map
        }));

        let mut pin_to_node = HashMap::with_capacity(board.nodes.len() * 3);
        let mut pins = HashMap::with_capacity(board.nodes.len() * 3);

        for (node_id, node) in &board.nodes {
            for (pin_id, pin) in &node.pins {
                let internal_pin = InternalPin {
                    pin: Arc::new(Mutex::new(pin.clone())),
                    node: Some(Weak::new()),
                    connected_to: vec![],
                    depends_on: vec![],
                    layer_pin: false,
                };

                pin_to_node.insert(pin_id, (node_id, node.is_pure()));
                pins.insert(pin.id.clone(), Arc::new(Mutex::new(internal_pin)));
            }
        }

        for layer in board.layers.values() {
            for (pin_id, pin) in &layer.pins {
                if pins.contains_key(pin_id) {
                    // this is the old layer format, where we just relayed the connected pin to the layers pin.
                    continue;
                }

                let internal_pin = InternalPin {
                    pin: Arc::new(Mutex::new(pin.clone())),
                    node: None,
                    connected_to: vec![],
                    depends_on: vec![],
                    layer_pin: true,
                };

                pins.insert(pin.id.clone(), Arc::new(Mutex::new(internal_pin)));
            }
        }

        for pin_arc in pins.values() {
            let mut internal_pin = pin_arc.lock().await;
            let (connected_to, depends_on) = {
                let inner = internal_pin.pin.lock().await;
                let connected_to = inner.connected_to.clone();
                let depends_on = inner.depends_on.clone();
                (connected_to, depends_on)
            };

            for connected_pin_id in connected_to {
                if let Some(connected_pin) = pins.get(&connected_pin_id) {
                    let connected = Arc::downgrade(connected_pin);
                    internal_pin.connected_to.push(connected);
                }
            }

            for depends_on_pin_id in depends_on {
                if let Some(depends_on_pin) = pins.get(&depends_on_pin_id) {
                    let depends_on = Arc::downgrade(depends_on_pin);
                    internal_pin.depends_on.push(depends_on);
                }
            }
        }

        let mut dependency_map = HashMap::with_capacity(board.nodes.len());
        let mut nodes = HashMap::with_capacity(board.nodes.len());
        let mut stack = RunStack::with_capacity(1);

        let registry = handler
            .lock()
            .await
            .node_registry
            .read()
            .await
            .node_registry
            .clone();
        for (node_id, node) in &board.nodes {
            let logic = registry.instantiate(node)?;
            let mut node_pins = HashMap::new();
            let mut pin_cache = HashMap::new();

            for pin in node.pins.values() {
                if let Some(internal_pin) = pins.get(&pin.id) {
                    node_pins.insert(pin.id.clone(), internal_pin.clone());
                    let cached_array = pin_cache.entry(pin.name.clone()).or_insert(vec![]);
                    cached_array.push(internal_pin.clone());
                }

                if USE_DEPENDENCY_GRAPH {
                    for dependency_pin_id in &pin.depends_on {
                        if let Some((dependency_node_id, is_pure)) =
                            pin_to_node.get(dependency_pin_id)
                        {
                            dependency_map
                                .entry(node_id)
                                .or_insert(vec![])
                                .push((*dependency_node_id, is_pure));
                        }
                    }
                }
            }

            let internal_node = Arc::new(InternalNode::new(
                node.clone(),
                node_pins.clone(),
                logic,
                pin_cache.clone(),
            ));

            for internal_pin in node_pins.values() {
                let mut pin_guard = internal_pin.lock().await;
                pin_guard.node = Some(Arc::downgrade(&internal_node));
            }

            if payload.id == node.id {
                stack.push(&node.id, internal_node.clone());
            }

            nodes.insert(node_id.clone(), internal_node);
        }

        if USE_DEPENDENCY_GRAPH {
            for node_id in board.nodes.keys() {
                let deps = recursive_get_deps(
                    node_id.to_string(),
                    &dependency_map,
                    &nodes,
                    &mut HashSet::new(),
                );
                dependencies.insert(node_id.clone(), deps);
            }
        }

        if board.log_level <= LogLevel::Info {
            println!(
                "InternalRun::new took {:?} on {} nodes and {} pins",
                before.elapsed(),
                nodes.len(),
                pins.len()
            );
        }

        Ok(InternalRun {
            run,
            nodes: Arc::new(nodes),
            pins,
            variables,
            cache: Arc::new(RwLock::new(HashMap::new())),
            stack: Arc::new(stack),
            concurrency_limit: 10,
            concurrency_map: Arc::new(DashMap::with_capacity(board.nodes.len())),
            cpus: num_cpus::get(),
            callback,
            credentials: credentials.map(Arc::new),
            dependencies,
            log_level: board.log_level,
            profile: Arc::new(profile.clone()),
            completion_callbacks: Arc::new(RwLock::new(vec![])),
        })
    }

    // Reuse the same run, but reset the states
    pub async fn fork(&mut self) -> flow_like_types::Result<()> {
        if self.stack.len() != 0 {
            return Err(flow_like_types::anyhow!(
                "Cannot fork a run that is not finished"
            ));
        }

        self.cache.write().await.clear();
        self.stack = Arc::new(RunStack::with_capacity(self.stack.len()));
        self.concurrency_limit = 128_000;
        self.run.lock().await.status = RunStatus::Running;
        self.run.lock().await.traces.clear();
        self.run.lock().await.start = SystemTime::now();
        self.run.lock().await.end = SystemTime::now();
        for node in self.nodes.values() {
            for (_, pin) in node.pins.iter() {
                pin.lock().await.reset().await;
            }
        }
        for variable in self.variables.lock().await.values_mut() {
            let default = variable.default_value.as_ref();
            let value = default.map_or(Value::Null, |v| {
                flow_like_types::json::from_slice(v).unwrap()
            });
            *variable.value.lock().await = value;
        }

        Ok(())
    }

    async fn step_parallel(
        &mut self,
        stack: Arc<RunStack>,
        handler: &Arc<Mutex<FlowLikeState>>,
        log_level: LogLevel,
        stage: ExecutionStage,
    ) {
        let variables = &self.variables;
        let cache = &self.cache;
        let dependencies = self.dependencies.clone();
        let run = self.run.clone();
        let profile = self.profile.clone();
        let concurrency_limit = self.concurrency_limit;
        let callback = self.callback.clone();

        let new_stack = futures::stream::iter(stack.stack.clone())
            .map(|node| {
                // Clone per iteration as needed
                let dependencies = dependencies.clone();
                let handler = handler.clone();
                let run = run.clone();
                let profile = profile.clone();
                let callback = callback.clone();
                let stage = stage.clone();
                let log_level = log_level;
                let concurrency_map = self.concurrency_map.clone();
                let completion_callbacks = self.completion_callbacks.clone();
                let credentials = self.credentials.clone();
                let nodes = self.nodes.clone();

                async move {
                    step_core(
                        nodes,
                        &node,
                        concurrency_limit,
                        &handler,
                        &run,
                        variables,
                        cache,
                        log_level,
                        stage,
                        &dependencies,
                        &profile,
                        &callback,
                        concurrency_map,
                        &completion_callbacks,
                        credentials,
                    )
                    .await
                }
            })
            .buffer_unordered(self.cpus * 3)
            .fold(
                RunStack::with_capacity(stack.stack.len()),
                |mut acc: RunStack, result| async move {
                    if let Ok(inner_iter) = result {
                        for (key, node) in inner_iter {
                            acc.push(&key, node);
                        }
                    }
                    acc
                },
            )
            .await;

        self.stack = Arc::new(new_stack);
    }

    async fn step_single(
        &mut self,
        stack: Arc<RunStack>,
        handler: &Arc<Mutex<FlowLikeState>>,
        log_level: LogLevel,
        stage: ExecutionStage,
    ) {
        let variables = &self.variables;
        let cache = &self.cache;
        let concurrency_limit = self.concurrency_limit;

        let node = stack.stack.first().unwrap();
        let connected_nodes = step_core(
            self.nodes.clone(),
            node,
            concurrency_limit,
            handler,
            &self.run,
            variables,
            cache,
            log_level,
            stage.clone(),
            &self.dependencies,
            &self.profile,
            &self.callback,
            self.concurrency_map.clone(),
            &self.completion_callbacks,
            self.credentials.clone(),
        )
        .await;

        let mut new_stack = RunStack::with_capacity(stack.len());
        if let Ok(nodes) = connected_nodes {
            for (key, node) in nodes {
                new_stack.push(&key, node);
            }
        }

        self.stack = Arc::new(new_stack);
    }

    async fn step(&mut self, handler: Arc<Mutex<FlowLikeState>>) {
        let start = Instant::now();

        let (stage, log_level, stack) = {
            let run = self.run.lock().await;
            (run.board.stage.clone(), run.log_level, self.stack.clone())
        };

        match stack.len() {
            1 => self.step_single(stack, &handler, log_level, stage).await,
            _ => self.step_parallel(stack, &handler, log_level, stage).await,
        };

        if self.log_level <= LogLevel::Debug {
            println!("InternalRun::step took {:?}", start.elapsed());
        }
    }

    pub async fn execute(&mut self, handler: Arc<Mutex<FlowLikeState>>) -> Option<LogMeta> {
        let start = Instant::now();

        {
            let mut run = self.run.lock().await;
            run.start = SystemTime::now();
        }

        let mut stack_hash = self.stack.hash();
        let mut current_stack_len = self.stack.len();
        let mut errored = false;
        let mut iter = 0;

        while current_stack_len > 0 {
            self.step(handler.clone()).await;
            iter += 1;

            if iter % 20 == 0 {
                let mut run = self.run.lock().await;
                if let Err(err) = run.flush_logs(false).await {
                    eprintln!("[Error] flushing logs: {:?}", err);
                }
            }

            current_stack_len = self.stack.len();
            let new_stack_hash = self.stack.hash();
            if new_stack_hash == stack_hash {
                errored = true;
                println!("End Reason: Stack did not change");
                break;
            }
            stack_hash = new_stack_hash;
        }

        self.trigger_completion_callbacks().await;

        let meta = {
            let mut run = self.run.lock().await;
            run.end = SystemTime::now();
            run.status = if errored {
                RunStatus::Failed
            } else {
                RunStatus::Success
            };
            match run.flush_logs(true).await {
                Ok(Some(meta)) => Some(meta),
                Ok(None) => None,
                Err(err) => {
                    eprintln!("[Error] flushing logs (final): {:?}", err);
                    None
                }
            }
        };

        if self.log_level == LogLevel::Info {
            println!("InternalRun::execute took {:?}", start.elapsed());
        }

        meta
    }

    pub async fn debug_step(&mut self, handler: Arc<Mutex<FlowLikeState>>) -> bool {
        let stack_hash = self.stack.hash();
        if self.stack.len() == 0 {
            let mut run = self.run.lock().await;
            run.end = SystemTime::now();
            run.status = RunStatus::Success;
            return false;
        }

        self.step(handler.clone()).await;

        if self.stack.len() == 0 {
            let mut run = self.run.lock().await;
            run.end = SystemTime::now();
            run.status = RunStatus::Success;
            return false;
        }

        let new_stack_hash = self.stack.hash();
        if new_stack_hash == stack_hash {
            let mut run = self.run.lock().await;
            run.end = SystemTime::now();
            run.status = RunStatus::Failed;
            return false;
        }

        true
    }

    pub async fn get_run(&self) -> Run {
        self.run.lock().await.clone()
    }

    pub async fn get_traces(&self) -> Vec<Trace> {
        self.run.lock().await.traces.clone()
    }

    pub async fn get_status(&self) -> RunStatus {
        self.run.lock().await.status.clone()
    }

    async fn trigger_completion_callbacks(&self) {
        let callbacks = self.completion_callbacks.read().await;
        for callback in callbacks.iter() {
            if let Err(err) = callback(self).await {
                eprintln!("[Error] executing completion callback: {:?}", err);
            }
        }
    }

    // ONLY CALL THIS IF WE ARE BEING CANCELLED
    pub async fn flush_logs_cancelled(&mut self) -> flow_like_types::Result<Option<LogMeta>> {
        let mut run = self.run.lock().await;
        run.highest_log_level = LogLevel::Fatal;
        run.status = RunStatus::Stopped;
        run.end = SystemTime::now();

        let cancel_log = LogMessage::new("Run cancelled", LogLevel::Fatal, None);
        if let Some(trace) = run.traces.last_mut() {
            trace.logs.push(cancel_log);
        } else {
            // Create a system trace if no traces exist
            let mut system_trace = Trace::new(
                run.visited_nodes
                    .keys()
                    .next()
                    .unwrap_or(&"system".to_string()),
            );
            system_trace.logs.push(cancel_log);
            run.traces.push(system_trace);
        }

        run.flush_logs(true).await
    }
}

fn recursive_get_deps(
    node_id: String,
    dependencies: &HashMap<&String, Vec<(&String, &bool)>>,
    lookup: &HashMap<String, Arc<InternalNode>>,
    recursion_filter: &mut HashSet<String>,
) -> Vec<Arc<InternalNode>> {
    if recursion_filter.contains(&node_id) {
        return vec![];
    }

    recursion_filter.insert(node_id.clone());

    if !dependencies.contains_key(&node_id) {
        return vec![];
    }

    let deps = dependencies.get(&node_id).unwrap();
    let mut found_dependencies = Vec::with_capacity(deps.len());

    for (dep_id, is_pure) in deps {
        if !**is_pure {
            continue;
        }

        if let Some(dep) = lookup.get(*dep_id) {
            found_dependencies.push(dep.clone());
        }

        found_dependencies.extend(recursive_get_deps(
            dep_id.to_string(),
            dependencies,
            lookup,
            recursion_filter,
        ));
    }

    found_dependencies
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub enum RunStatus {
    Running,
    Success,
    Failed,
    Stopped,
}

async fn step_core(
    nodes: Arc<HashMap<String, Arc<InternalNode>>>,
    node: &Arc<InternalNode>,
    concurrency_limit: u64,
    handler: &Arc<Mutex<FlowLikeState>>,
    run: &Arc<Mutex<Run>>,
    variables: &Arc<Mutex<HashMap<String, Variable>>>,
    cache: &Arc<RwLock<HashMap<String, Arc<dyn Cacheable>>>>,
    log_level: LogLevel,
    stage: ExecutionStage,
    dependencies: &HashMap<String, Vec<Arc<InternalNode>>>,
    profile: &Arc<Profile>,
    callback: &InterComCallback,
    concurrency_map: Arc<DashMap<String, u64>>,
    completion_callbacks: &Arc<RwLock<Vec<EventTrigger>>>,
    credentials: Option<Arc<SharedCredentials>>,
) -> flow_like_types::Result<Vec<(String, Arc<InternalNode>)>> {
    // Check Node State and Validate Execution Count (to stop infinite loops)
    {
        let mut limit = concurrency_map
            .entry(node.node.lock().await.id.clone())
            .or_insert(0);
        if *limit >= concurrency_limit {
            return Err(anyhow!("Concurrency limit reached"));
        }

        *limit += 1;
    }

    let weak_run = Arc::downgrade(run);
    let mut context = ExecutionContext::new(
        nodes,
        &weak_run,
        handler,
        node,
        variables,
        cache,
        log_level,
        stage.clone(),
        profile.clone(),
        callback.clone(),
        completion_callbacks.clone(),
        credentials,
    )
    .await;

    if USE_DEPENDENCY_GRAPH {
        if let Err(err) =
            InternalNode::trigger_with_dependencies(&mut context, &mut None, false, dependencies)
                .await
        {
            eprintln!("[Error] executing node: {:?}", err);
        }
    } else if let Err(err) = InternalNode::trigger(&mut context, &mut None, false).await {
        eprintln!("[Error] executing node: {:?}", err);
    }

    {
        let mut run_locked = run.lock().await;
        run_locked.traces.extend(context.get_traces());
    }

    let state = context.get_state();

    drop(context);

    if state == NodeState::Success {
        let connected = node.get_connected_exec(true).await.unwrap();
        let mut connected_nodes = Vec::with_capacity(connected.len());
        for connected_node in connected {
            let id = connected_node.node.clone().lock().await.id.clone();

            connected_nodes.push((id, connected_node.clone()));
        }
        return Ok(connected_nodes);
    }

    Err(anyhow!("Node failed"))
}
