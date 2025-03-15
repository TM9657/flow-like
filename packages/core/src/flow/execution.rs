use ahash::AHasher;
use context::ExecutionContext;
use cuid2;
use futures::StreamExt;
use internal_node::InternalNode;
use internal_pin::InternalPin;
use num_cpus;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::Any;
use std::time::Instant;
use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    sync::{Arc, Weak},
    time::SystemTime,
};
use tokio::sync::{Mutex, RwLock};
use trace::Trace;
pub mod context;
pub mod internal_node;
pub mod internal_pin;
pub mod log;
pub mod trace;

use crate::profile::Profile;
use crate::state::{FlowLikeEvent, FlowLikeState};

use super::board::ExecutionStage;
use super::catalog::load_catalog;
use super::{board::Board, node::NodeState, variable::Variable};

const USE_DEPENDENCY_GRAPH: bool = false;

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Eq, PartialOrd)]
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
    Fatal = 4,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub struct Run {
    pub id: String,
    pub traces: Vec<Trace>,
    pub status: RunStatus,
    pub start: SystemTime,
    pub end: SystemTime,
    pub board: Arc<Board>,
    pub log_level: LogLevel,
    pub sub: String,
}

pub trait Cacheable: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl dyn Cacheable {
    pub fn downcast_ref<T: Cacheable>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }

    pub fn downcast_mut<T: Cacheable>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut::<T>()
    }
}

#[derive(Clone)]
struct RunStack {
    stack: Vec<Arc<Mutex<InternalNode>>>,
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

    fn push(&mut self, node_id: &str, node: Arc<Mutex<InternalNode>>) {
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

#[derive(Clone)]
pub struct InternalRun {
    pub run: Arc<Mutex<Run>>,
    pub nodes: HashMap<String, Arc<Mutex<InternalNode>>>,
    pub dependencies: HashMap<String, Vec<Arc<Mutex<InternalNode>>>>,
    pub pins: HashMap<String, Arc<Mutex<InternalPin>>>,
    pub variables: Arc<Mutex<HashMap<String, Variable>>>,
    pub cache: Arc<RwLock<HashMap<String, Arc<dyn Cacheable>>>>,
    pub profile: Arc<Profile>,
    pub sender: tokio::sync::mpsc::Sender<FlowLikeEvent>,

    stack: Arc<RunStack>,
    concurrency_limit: u64,
    cpus: usize,
    log_level: LogLevel,
}

impl InternalRun {
    pub async fn new(
        board: Board,
        handler: &Arc<Mutex<FlowLikeState>>,
        profile: &Profile,
        start_ids: Vec<String>,
        sub: Option<String>,
    ) -> anyhow::Result<Self> {
        let before = Instant::now();
        let start_ids_set: HashSet<String> = start_ids.into_iter().collect();
        let sender = handler.lock().await.event_sender.lock().await.clone();
        let run_id = cuid2::create_id();

        let board = Arc::new(board);

        let run = Run {
            id: run_id.clone(),
            traces: vec![],
            status: RunStatus::Running,
            start: SystemTime::now(),
            end: SystemTime::now(),
            log_level: board.log_level.clone(),
            board: board.clone(),
            sub: sub.unwrap_or_else(|| "local".to_string()),
        };

        let run = Arc::new(Mutex::new(run));

        let mut dependencies = HashMap::with_capacity(board.nodes.len());

        let variables = Arc::new(Mutex::new({
            let mut map = HashMap::with_capacity(board.variables.len());
            for (variable_id, variable) in &board.variables {
                let value = variable
                    .default_value
                    .as_ref()
                    .map_or(Value::Null, |v| serde_json::from_slice::<Value>(v).unwrap());
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
                    node: Weak::new(),
                    connected_to: vec![],
                    depends_on: vec![],
                };

                pin_to_node.insert(pin_id, (node_id, node.is_pure()));
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
                    internal_pin.connected_to.push(connected_pin.clone());
                }
            }

            for depends_on_pin_id in depends_on {
                if let Some(depends_on_pin) = pins.get(&depends_on_pin_id) {
                    internal_pin.depends_on.push(depends_on_pin.clone());
                }
            }
        }

        let mut dependency_map = HashMap::with_capacity(board.nodes.len());
        let mut nodes = HashMap::with_capacity(board.nodes.len());
        let mut stack = RunStack::with_capacity(start_ids_set.len());

        if !handler.lock().await.node_registry.read().await.initialized {
            load_catalog(handler.clone()).await;
        }

        let registry = handler.lock().await.node_registry.read().await.node_registry.clone();
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


            let internal_node = Arc::new(Mutex::new(InternalNode::new(
                node.clone(),
                node_pins.clone(),
                logic,
                pin_cache.clone(),
            )));

            for internal_pin in node_pins.values() {
                let mut pin_guard = internal_pin.lock().await;
                pin_guard.node = Arc::downgrade(&internal_node);
            }

            if start_ids_set.contains(&node.id) {
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
            nodes,
            pins,
            variables,
            cache: Arc::new(RwLock::new(HashMap::new())),
            stack: Arc::new(stack),
            concurrency_limit: 128_000,
            cpus: num_cpus::get(),
            sender,
            dependencies,
            log_level: board.log_level.clone(),
            profile: Arc::new(profile.clone()),
        })
    }

    // Reuse the same run, but reset the states
    pub async fn fork(&mut self) -> anyhow::Result<()> {
        if self.stack.len() != 0 {
            return Err(anyhow::anyhow!("Cannot fork a run that is not finished"));
        }

        self.cache.write().await.clear();
        self.stack = Arc::new(RunStack::with_capacity(self.stack.len()));
        self.concurrency_limit = 128_000;
        self.run.lock().await.status = RunStatus::Running;
        self.run.lock().await.traces.clear();
        self.run.lock().await.start = SystemTime::now();
        self.run.lock().await.end = SystemTime::now();
        for node in self.nodes.values() {
            let mut guard = node.lock().await;
            guard.reset().await;
            for (_, pin) in guard.pins.iter_mut() {
                pin.lock().await.reset().await;
            }
        }
        for variable in self.variables.lock().await.values_mut() {
            let default = variable.default_value.as_ref();
            let value = default.map_or(Value::Null, |v| serde_json::from_slice(v).unwrap());
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
        let sender = self.sender.clone();
        let concurrency_limit = self.concurrency_limit;

        let new_stack = futures::stream::iter(stack.stack.clone())
            .map(|node| {
                // Clone per iteration as needed
                let dependencies = dependencies.clone();
                let handler = handler.clone();
                let run = run.clone();
                let profile = profile.clone();
                let sender = sender.clone();
                let stage = stage.clone();
                let log_level = log_level.clone();

                async move {
                    step_core(
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
                        &sender,
                    )
                    .await
                }
            })
            .buffer_unordered(self.cpus * 3)
            .fold(RunStack::with_capacity(stack.stack.len()), |mut acc, result| async move {
                if let Some(inner_iter) = result {
                    for (key, node) in inner_iter {
                        acc.push(&key, node);
                    }
                }
                acc
            })
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
            node,
            concurrency_limit,
            handler,
            &self.run,
            variables,
            cache,
            log_level.clone(),
            stage.clone(),
            &self.dependencies,
            &self.profile,
            &self.sender,
        )
        .await;

        let mut new_stack = RunStack::with_capacity(stack.len());
        if let Some(nodes) = connected_nodes {
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
            (
                run.board.stage.clone(),
                run.log_level.clone(),
                self.stack.clone(),
            )
        };

        match stack.len() {
            1 => self.step_single(stack, &handler, log_level, stage).await,
            _ => self.step_parallel(stack, &handler, log_level, stage).await,
        };

        if self.log_level <= LogLevel::Debug {
            println!("InternalRun::step took {:?}", start.elapsed());
        }
    }

    pub async fn execute(&mut self, handler: Arc<Mutex<FlowLikeState>>) {
        let start = Instant::now();
        self.run.lock().await.start = SystemTime::now();
        let mut stack_hash = self.stack.hash();
        let mut current_stack_len = self.stack.len();

        let mut errored = false;
        while current_stack_len > 0 {
            self.step(handler.clone()).await;
            current_stack_len = self.stack.len();
            let new_stack_hash = self.stack.hash();
            if new_stack_hash == stack_hash {
                errored = true;
                println!("End Reason: Stack did not change");
                break;
            }
            stack_hash = new_stack_hash;
        }

        self.run.lock().await.end = SystemTime::now();
        self.run.lock().await.status = if errored {
            RunStatus::Failed
        } else {
            RunStatus::Success
        };

        if self.log_level == LogLevel::Info {
            println!("InternalRun::execute took {:?}", start.elapsed());
        }
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
}

fn recursive_get_deps(
    node_id: String,
    dependencies: &HashMap<&String, Vec<(&String, &bool)>>,
    lookup: &HashMap<String, Arc<Mutex<InternalNode>>>,
    recursion_filter: &mut HashSet<String>,
) -> Vec<Arc<Mutex<InternalNode>>> {
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
    node: &Arc<Mutex<InternalNode>>,
    concurrency_limit: u64,
    handler: &Arc<Mutex<FlowLikeState>>,
    run: &Arc<Mutex<Run>>,
    variables: &Arc<Mutex<HashMap<String, Variable>>>,
    cache: &Arc<RwLock<HashMap<String, Arc<dyn Cacheable>>>>,
    log_level: LogLevel,
    stage: ExecutionStage,
    dependencies: &HashMap<String, Vec<Arc<Mutex<InternalNode>>>>,
    profile: &Arc<Profile>,
    sender: &tokio::sync::mpsc::Sender<FlowLikeEvent>,
) -> Option<Vec<(String, Arc<Mutex<InternalNode>>)>> {
    // Check Node State and Validate Execution Count (to stop infinite loops)
    {
        let mut internal_node = node.lock().await;

        if internal_node.get_execution_count() >= concurrency_limit
            || internal_node.orphaned().await
        {
            return None;
        }

        internal_node.increment_execution_count();
    }

    let weak_run = Arc::downgrade(run);
    let mut context = ExecutionContext::new(
        &weak_run,
        handler,
        node,
        variables,
        cache,
        log_level.clone(),
        stage.clone(),
        profile.clone(),
        sender.clone(),
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
        let connected = node.lock().await.get_connected_exec(true).await;
        let mut connected_nodes = Vec::with_capacity(connected.len());
        for connected_node in connected {
            let id = connected_node
                .lock()
                .await
                .node
                .clone()
                .lock()
                .await
                .id
                .clone();

            connected_nodes.push((id, connected_node.clone()));
        }
        return Some(connected_nodes);
    }

    None
}
