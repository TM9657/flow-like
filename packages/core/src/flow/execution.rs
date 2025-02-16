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
use std::collections::BTreeMap;
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
use crate::vault::vector::VectorStore;

use super::board::ExecutionStage;
use super::{board::Board, catalog::node_to_dyn, node::NodeState, variable::Variable};

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
    pub board: Board,
    pub log_level: LogLevel,
}

#[derive(Clone)]
pub enum CacheValue {
    Value(Value),
    VectorDB(Arc<Mutex<dyn VectorStore>>),
}

#[derive(Clone)]
pub struct InternalRun {
    pub run: Arc<Mutex<Run>>,
    pub nodes: HashMap<String, Arc<Mutex<InternalNode>>>,
    pub dependencies: HashMap<String, Vec<Arc<Mutex<InternalNode>>>>,
    pub pins: HashMap<String, Arc<Mutex<InternalPin>>>,
    pub variables: Arc<Mutex<HashMap<String, Variable>>>,
    pub cache: Arc<RwLock<HashMap<String, CacheValue>>>,
    pub profile: Arc<Profile>,
    pub sender: tokio::sync::mpsc::Sender<FlowLikeEvent>,

    // starting with the leaf nodes, executing these in parallel
    stack: Arc<Mutex<BTreeMap<String, Arc<Mutex<InternalNode>>>>>,
    concurrency_limit: u64,
    cpus: usize,
    log_level: LogLevel,
}

impl InternalRun {
    pub async fn new(
        board: &Board,
        handler: &Arc<Mutex<FlowLikeState>>,
        profile: &Profile,
        start_ids: Vec<String>,
        log_level: LogLevel,
    ) -> anyhow::Result<Self> {
        let before = Instant::now();
        let start_ids: HashSet<String> = HashSet::from_iter(start_ids);
        let sender = handler.lock().await.event_sender.lock().await.clone();
        let run_id = cuid2::create_id();
        let run = Run {
            id: run_id.clone(),
            traces: vec![],
            status: RunStatus::Running,
            start: SystemTime::now(),
            end: SystemTime::now(),
            board: board.clone(),
            log_level: log_level.clone(),
        };

        let run = Arc::new(Mutex::new(run));

        let mut pin_to_node = HashMap::new();
        let mut dependencies = HashMap::new();
        let variables = Arc::new(Mutex::new(HashMap::with_capacity(board.variables.len())));
        let mut stack = BTreeMap::new();

        for (variable_id, variable) in &board.variables {
            // initialize variable value
            let value = match &variable.default_value {
                Some(default_value) => serde_json::from_slice::<Value>(default_value).unwrap(),
                None => Value::Null,
            };
            let value = Arc::new(Mutex::new(value));
            let mut variable = variable.clone();
            variable.value = value.clone();
            variables.lock().await.insert(variable_id.clone(), variable);
        }

        let mut nodes = HashMap::with_capacity(board.nodes.len());
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

                let internal_pin = Arc::new(Mutex::new(internal_pin));
                pins.insert(pin.id.clone(), internal_pin);
            }
        }

        for pin in pins.values() {
            let mut pin_guard = pin.lock().await;
            let connected_to = pin_guard.pin.lock().await.connected_to.clone();
            let depends_on = pin_guard.pin.lock().await.depends_on.clone();

            for connected_pin_id in connected_to {
                if let Some(connected_pin) = pins.get(&connected_pin_id) {
                    pin_guard.connected_to.push(connected_pin.clone());
                }
            }

            for depends_on_pin_id in depends_on {
                if let Some(depends_on_pin) = pins.get(&depends_on_pin_id) {
                    pin_guard.depends_on.push(depends_on_pin.clone());
                }
            }
        }

        let mut dependency_map = HashMap::with_capacity(board.nodes.len());

        let handler = handler.lock().await;
        for (node_id, node) in &board.nodes {
            let logic = node_to_dyn(&handler, node).await?;
            let mut node_pins = HashMap::new();
            let mut pin_cache = HashMap::new();

            for pin in node.pins.values() {
                if let Some(internal_pin) = pins.get(&pin.id) {
                    node_pins.insert(pin.id.clone(), internal_pin.clone());
                    let cached_array = pin_cache.entry(pin.name.clone()).or_insert(vec![]);
                    cached_array.push(internal_pin.clone());
                }

                if !USE_DEPENDENCY_GRAPH {
                    continue;
                }

                for dependency_pin_id in &pin.depends_on {
                    let (dependency_node_id, is_pure) = pin_to_node.get(dependency_pin_id).unwrap();

                    dependency_map
                        .entry(node_id)
                        .or_insert(vec![])
                        .push((*dependency_node_id, is_pure));
                }
            }

            let internal_node =
                InternalNode::new(node.clone(), node_pins.clone(), logic, pin_cache.clone());
            let internal_node = Arc::new(Mutex::new(internal_node));

            for pin in node_pins.values() {
                let mut pin_guard = pin.lock().await;
                pin_guard.node = Arc::downgrade(&internal_node);
            }

            if start_ids.contains(&node.id) {
                stack.insert(node.id.clone(), internal_node.clone());
            }

            nodes.insert(node_id.clone(), internal_node);
        }

        if USE_DEPENDENCY_GRAPH {
            dependency_map.iter().for_each(|(node_id, _)| {
                let deps = recursive_get_deps(
                    node_id.to_string(),
                    &dependency_map,
                    &nodes,
                    &mut HashSet::new(),
                );
                dependencies.insert(node_id.to_string(), deps);
            });
        }

        if log_level <= LogLevel::Info {
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
            stack: Arc::new(Mutex::new(stack)),
            concurrency_limit: 128_000,
            cpus: num_cpus::get(),
            sender,
            dependencies,
            log_level,
            profile: Arc::new(profile.clone()),
        })
    }

    async fn step_parallel(
        &mut self,
        stack: BTreeMap<String, Arc<Mutex<InternalNode>>>,
        handler: &Arc<Mutex<FlowLikeState>>,
        log_level: LogLevel,
        stage: ExecutionStage,
    ) {
        let variables = &self.variables;
        let cache = &self.cache;
        let dependencies = &self.dependencies;
        let concurrency_limit = self.concurrency_limit;

        let futures = futures::stream::iter(stack)
            .map(|(_, node)| {
                let dependencies = dependencies.clone();
                let handler = handler.clone();
                let log_level = log_level.clone();
                let stage = stage.clone();
                let run = self.run.clone();
                let profile = self.profile.clone();
                let sender = self.sender.clone();

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
            .buffer_unordered(self.cpus * 3);

        let intermediate_stack: BTreeMap<_, _> = futures
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .flatten()
            .flatten()
            .collect();

        self.stack = Arc::new(Mutex::new(intermediate_stack));
    }

    async fn step_single(
        &mut self,
        stack: BTreeMap<String, Arc<Mutex<InternalNode>>>,
        handler: &Arc<Mutex<FlowLikeState>>,
        log_level: LogLevel,
        stage: ExecutionStage,
    ) {
        let variables = &self.variables;
        let cache = &self.cache;
        let concurrency_limit = self.concurrency_limit;

        let (_, node) = stack.iter().next().unwrap();
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

        let intermediate_stack = match connected_nodes {
            Some(connected_nodes) => connected_nodes.into_iter().collect(),
            None => BTreeMap::new(),
        };

        self.stack = Arc::new(Mutex::new(intermediate_stack));
    }

    async fn step(&mut self, handler: Arc<Mutex<FlowLikeState>>) {
        let start = Instant::now();

        let (stage, log_level, stack) = {
            let run = self.run.lock().await;
            (
                run.board.stage.clone(),
                run.log_level.clone(),
                self.stack.lock().await.clone(),
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

    async fn stack_hash(&self) -> u64 {
        let start = Instant::now();
        let stack: Arc<Mutex<BTreeMap<String, Arc<Mutex<InternalNode>>>>> = self.stack.clone();
        let stack = stack.lock().await;
        let mut hasher = AHasher::default();

        for (node_id, _node) in stack.iter() {
            node_id.hash(&mut hasher);
        }

        let result = hasher.finish();

        if self.log_level <= LogLevel::Debug {
            println!("InternalRun::stack_hash took {:?}", start.elapsed());
        }

        result
    }

    pub async fn execute(&mut self, handler: Arc<Mutex<FlowLikeState>>) {
        let start = Instant::now();
        self.run.lock().await.start = SystemTime::now();
        let mut stack_hash = self.stack_hash().await;
        let mut current_stack_len = self.stack.lock().await.len();

        let mut errored = false;
        while current_stack_len > 0 {
            self.step(handler.clone()).await;
            current_stack_len = self.stack.lock().await.len();
            let new_stack_hash = self.stack_hash().await;
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
        let stack_hash = self.stack_hash().await;
        if self.stack.lock().await.len() == 0 {
            let mut run = self.run.lock().await;
            run.end = SystemTime::now();
            run.status = RunStatus::Success;
            return false;
        }

        self.step(handler.clone()).await;

        if self.stack.lock().await.len() == 0 {
            let mut run = self.run.lock().await;
            run.end = SystemTime::now();
            run.status = RunStatus::Success;
            return false;
        }

        let new_stack_hash = self.stack_hash().await;
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
    cache: &Arc<RwLock<HashMap<String, CacheValue>>>,
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
            let id = {
                let connected_node = connected_node.lock().await;
                let node = connected_node.node.lock().await;
                node.id.clone()
            };

            connected_nodes.push((id, connected_node.clone()));
        }
        return Some(connected_nodes);
    }

    None
}
