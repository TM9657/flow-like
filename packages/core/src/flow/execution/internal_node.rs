use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::Mutex;

use crate::flow::{
    node::{Node, NodeLogic, NodeState},
    pin::PinType,
    utils::evaluate_pin_value,
    variable::VariableType,
};

use super::{context::ExecutionContext, internal_pin::InternalPin, log::LogMessage, LogLevel};

#[derive(Debug)]
pub enum InternalNodeError {
    DependencyFailed(String),
    ExecutionFailed(String),
    PinNotReady(String),
}

pub struct InternalNode {
    pub node: Arc<Mutex<Node>>,
    pub pins: HashMap<String, Arc<Mutex<InternalPin>>>,
    pub logic: Arc<dyn NodeLogic>,
    pin_name_cache: Mutex<HashMap<String, Vec<Arc<Mutex<InternalPin>>>>>,
}

impl InternalNode {
    pub fn new(
        node: Node,
        pins: HashMap<String, Arc<Mutex<InternalPin>>>,
        logic: Arc<dyn NodeLogic>,
        name_cache: HashMap<String, Vec<Arc<Mutex<InternalPin>>>>,
    ) -> Self {
        InternalNode {
            node: Arc::new(Mutex::new(node)),
            pins,
            logic,
            pin_name_cache: Mutex::new(name_cache),
        }
    }

    pub async fn ensure_cache(&self, name: &str) {
        {
            let cache = self.pin_name_cache.lock().await;
            if cache.contains_key(name) {
                return;
            }
        }

        let mut pins_by_name = HashMap::new();
        for pin_ref in self.pins.values() {
            let pin_name = {
                let pin_guard = pin_ref.lock().await;
                let pin = pin_guard.pin.lock().await;
                pin.name.clone()
            };

            pins_by_name
                .entry(pin_name)
                .or_insert_with(Vec::new)
                .push(pin_ref.clone());
        }

        let mut cache = self.pin_name_cache.lock().await;
        for (pin_name, pins) in pins_by_name {
            cache.entry(pin_name).or_insert(pins);
        }
    }

    pub async fn get_pin_by_name(&self, name: &str) -> anyhow::Result<Arc<Mutex<InternalPin>>> {
        self.ensure_cache(name).await;

        let pin = {
            let cache = self.pin_name_cache.lock().await;
            cache
                .get(name)
                .and_then(|pins_ref| pins_ref.first().cloned())
        };

        let pin = pin.ok_or(anyhow::anyhow!("Pin {} not found", name))?;
        Ok(pin)
    }

    pub async fn get_pins_by_name(
        &self,
        name: &str,
    ) -> anyhow::Result<Vec<Arc<Mutex<InternalPin>>>> {
        self.ensure_cache(name).await;
        let cache = self.pin_name_cache.lock().await;
        if let Some(pins_ref) = cache.get(name) {
            return Ok(pins_ref.clone());
        }

        Err(anyhow::anyhow!("Pin {} not found", name))
    }

    pub fn get_pin_by_id(&self, id: &str) -> anyhow::Result<Arc<Mutex<InternalPin>>> {
        if let Some(pin) = self.pins.get(id) {
            return Ok(pin.clone());
        }

        Err(anyhow::anyhow!("Pin {} not found", id))
    }

    pub async fn orphaned(&self) -> bool {
        for pin in self.pins.values() {
            let pin_guard = pin.lock().await.pin.clone();
            let pin = pin_guard.lock().await;

            if pin.pin_type != PinType::Input {
                continue;
            }

            if pin.depends_on.is_empty() && pin.default_value.is_none() {
                return true;
            }
        }

        false
    }

    pub async fn is_ready(&self) -> bool {
        for pin in self.pins.values() {
            let pin_guard = pin.lock().await;
            let pin = pin_guard.pin.lock().await;

            if pin.pin_type != PinType::Input {
                continue;
            }

            if pin.depends_on.is_empty() && pin.default_value.is_none() {
                return false;
            }

            // execution pins can have multiple inputs for different paths leading to it. We only need to make sure that one of them is valid!
            let is_execution = pin.data_type == VariableType::Execution;
            let mut execution_valid = false;
            let depends_on = pin_guard.depends_on.clone();
            drop(pin);
            drop(pin_guard);

            for depends_on_pin in depends_on {
                let depends_on_pin_guard = depends_on_pin.lock().await;
                let depends_on_pin = depends_on_pin_guard.pin.lock().await;

                // non execution pins need all inputs to be valid
                if depends_on_pin.value.is_none() && !is_execution {
                    return false;
                }

                if depends_on_pin.value.is_some() {
                    execution_valid = true;
                }
            }

            if is_execution && !execution_valid {
                return false;
            }
        }

        true
    }

    pub async fn get_connected(&self) -> Vec<Arc<InternalNode>> {
        let mut connected = Vec::with_capacity(self.pins.len());

        for pin in self.pins.values() {
            let pin_guard = pin.lock().await;
            let pin = pin_guard.pin.lock().await;

            if pin.pin_type != PinType::Output {
                continue;
            }

            drop(pin);

            let connected_pins = pin_guard.connected_to.clone();

            for connected_pin in &connected_pins {
                let connected_pin_guard = connected_pin.lock().await;
                let connected_node = connected_pin_guard.node.upgrade();

                if let Some(connected_node) = connected_node {
                    connected.push(connected_node);
                }
            }
        }

        connected
    }

    pub async fn get_connected_exec(&self, filter_valid: bool) -> Vec<Arc<InternalNode>> {
        let mut connected = vec![];

        for pin in self.pins.values() {
            let value = evaluate_pin_value(pin.clone()).await;

            if filter_valid && value.is_err() {
                continue;
            }

            let bool_val = match value.unwrap() {
                serde_json::Value::Bool(b) => b,
                _ => false,
            };

            if filter_valid && !bool_val {
                continue;
            }

            let pin_guard = pin.lock().await;
            let pin = pin_guard.pin.lock().await;

            if pin.pin_type != PinType::Output {
                continue;
            }

            if pin.data_type != VariableType::Execution {
                continue;
            }

            drop(pin);

            let connected_pins = pin_guard.connected_to.clone();

            for connected_pin in &connected_pins {
                let connected_pin_guard = connected_pin.lock().await;
                let connected_node = connected_pin_guard.node.upgrade();

                if let Some(connected_node) = connected_node {
                    connected.push(connected_node);
                }
            }
        }

        connected
    }

    pub async fn get_dependencies(&self) -> Vec<Arc<InternalNode>> {
        let mut dependencies = Vec::with_capacity(self.pins.len());

        for pin in self.pins.values() {
            let pin_guard = pin.lock().await;
            let pin = pin_guard.pin.lock().await;

            if pin.pin_type != PinType::Input {
                continue;
            }

            drop(pin);

            let dependency_pins = pin_guard.depends_on.clone();

            for connected_pin in &dependency_pins {
                let dependency_pin_guard = connected_pin.lock().await;
                let dependency_pin = dependency_pin_guard.node.upgrade();

                if let Some(dependency) = dependency_pin {
                    dependencies.push(dependency);
                }
            }
        }

        dependencies
    }

    pub async fn is_pure(&self) -> bool {
        let node = self.node.lock().await;
        let pins = node
            .pins
            .values()
            .find(|pin| pin.data_type == VariableType::Execution);
        pins.is_none()
    }

    async fn trigger_missing_dependencies(
        context: &mut ExecutionContext,
        recursion_guard: &mut Option<HashSet<String>>,
        with_successors: bool,
    ) -> bool {
        let pins = context.node.pins.clone();

        // TODO: optimize this for parallel execution
        for pin in pins.values() {
            let dependencies = {
                let pin_guard = pin.lock().await;
                let pin = pin_guard.pin.lock().await;

                if pin.pin_type != PinType::Input {
                    continue;
                }

                // we only trigger pure dependencies
                if pin.data_type == VariableType::Execution {
                    continue;
                }

                pin_guard.depends_on.clone()
            };

            // TODO: optimize this for parallel execution
            for dependency in &dependencies {
                let parent = {
                    let dependency_guard = dependency.lock().await;

                    let parent = dependency_guard.node.upgrade();

                    if parent.is_none() {
                        continue;
                    }

                    parent.unwrap().clone()
                };

                let (node_id, node_name) = {
                    // We only run pure nodes
                    if !parent.is_pure().await {
                        continue;
                    }
                    let parent_node = parent.node.lock().await;
                    (parent_node.id.clone(), parent_node.friendly_name.clone())
                };

                if let Some(recursion_guard) = recursion_guard {
                    if recursion_guard.contains(&node_id) {
                        context.log_message(
                            &format!("Recursion detected for: {}, skipping execution", &node_id),
                            LogLevel::Debug,
                        );
                        continue;
                    }
                }

                let mut sub_context = context.create_sub_context(&parent).await;

                let mut log_message = LogMessage::new(
                    &format!("Triggering missing dependency: {}", &node_name),
                    LogLevel::Debug,
                    None,
                );
                let success = Box::pin(InternalNode::trigger(
                    &mut sub_context,
                    recursion_guard,
                    with_successors,
                ))
                .await;
                log_message.end();
                context.log(log_message);
                sub_context.end_trace();
                context.push_sub_context(sub_context);

                if success.is_err() {
                    context.log_message(
                        &format!("Failed to trigger dependency: {}", &node_name),
                        LogLevel::Error,
                    );
                    return false;
                }
            }
        }
        true
    }

    pub async fn trigger(
        context: &mut ExecutionContext,
        recursion_guard: &mut Option<HashSet<String>>,
        with_successors: bool,
    ) -> Result<(), InternalNodeError> {
        context.set_state(NodeState::Running).await;
        let node = context.read_node().await;

        // create recursion guard if not present
        if recursion_guard.is_none() {
            *recursion_guard = Some(HashSet::new());
        }

        // check recursion guard
        if let Some(recursion_guard) = recursion_guard {
            if recursion_guard.contains(&node.id) {
                context.log_message(
                    &format!("Recursion detected for: {}", &node.id),
                    LogLevel::Debug,
                );
                context.end_trace();
                return Ok(());
            }

            recursion_guard.insert(node.id.clone());
        }

        let success =
            InternalNode::trigger_missing_dependencies(context, recursion_guard, with_successors)
                .await;
        if !success {
            context.log_message("Failed to trigger missing dependencies", LogLevel::Error);
            context.end_trace();
            return Err(InternalNodeError::DependencyFailed(node.id));
        }

        let logic = context.node.logic.clone();

        let mut log_message = LogMessage::new(
            &format!("Starting Node Execution: {} [{}]", &node.name, &node.id),
            LogLevel::Debug,
            None,
        );
        let result = logic.run(context).await;
        drop(logic);
        if result.is_err() {
            context.log_message(
                &format!("Failed to execute node: {:?}", result.err()),
                LogLevel::Error,
            );
            log_message.end();
            context.log(log_message);
            context.end_trace();
            context.set_state(NodeState::Error).await;
            return Err(InternalNodeError::ExecutionFailed(node.id));
        }
        context.set_state(NodeState::Success).await;
        log_message.end();
        context.log(log_message);
        context.end_trace();

        if with_successors {
            let successors = context.node.get_connected_exec(true).await;
            // TODO: optimize this for parallel execution
            for successor in successors {
                let mut sub_context = context.create_sub_context(&successor).await;
                let result = Box::pin(InternalNode::trigger(
                    &mut sub_context,
                    &mut None,
                    with_successors,
                ))
                .await;
                sub_context.end_trace();
                context.push_sub_context(sub_context);
                if result.is_err() {
                    return Err(InternalNodeError::ExecutionFailed(node.id));
                }
            }
        }

        Ok(())
    }

    pub async fn trigger_with_dependencies(
        context: &mut ExecutionContext,
        recursion_guard: &mut Option<HashSet<String>>,
        with_successors: bool,
        dependencies: &HashMap<String, Vec<Arc<InternalNode>>>,
    ) -> Result<(), InternalNodeError> {
        context.set_state(NodeState::Running).await;

        let node = context.read_node().await;

        if recursion_guard.is_none() {
            *recursion_guard = Some(HashSet::new());
        }

        if let Some(recursion_guard) = recursion_guard {
            if recursion_guard.contains(&node.id) {
                context.log_message(
                    &format!("Recursion detected for: {}", &node.id),
                    LogLevel::Debug,
                );
                context.end_trace();
                return Ok(());
            }

            recursion_guard.insert(node.id.clone());
        }

        let mut executed_dependencies = HashSet::new();

        // TODO: add the depth of the dependency, sort by depth and execute all nodes of the same depth in parallel (e.g parallel AI calls)
        if let Some(dep) = dependencies.get(&node.id) {
            for node_ref in dep.iter().rev() {
                let mut sub_context = context.create_sub_context(node_ref).await;
                let node_id = sub_context.trace.node_id.clone();
                if executed_dependencies.contains(&node_id) {
                    continue;
                }
                executed_dependencies.insert(node_id);
                let result = Box::pin(InternalNode::trigger_with_dependencies(
                    &mut sub_context,
                    recursion_guard,
                    false,
                    &HashMap::new(),
                ))
                .await;
                sub_context.end_trace();
                context.push_sub_context(sub_context);
                if result.is_err() {
                    return Err(InternalNodeError::DependencyFailed(node.id));
                }
            }
        }

        let logic = context.node.logic.clone();

        let mut log_message = LogMessage::new(
            &format!("Starting Node Execution: {} [{}]", &node.name, &node.id),
            LogLevel::Debug,
            None,
        );
        let result = logic.run(context).await;

        if result.is_err() {
            context.log_message(
                &format!("Failed to execute node: {:?}", result.err()),
                LogLevel::Error,
            );
            log_message.end();
            context.log(log_message);
            context.end_trace();
            context.set_state(NodeState::Error).await;
            return Err(InternalNodeError::ExecutionFailed(node.id));
        }
        context.set_state(NodeState::Success).await;
        log_message.end();
        context.log(log_message);
        context.end_trace();

        if with_successors {
            let successors = context.node.get_connected_exec(true).await;
            // TODO: optimize this for parallel execution
            for successor in successors {
                let mut sub_context = context.create_sub_context(&successor).await;
                let result = Box::pin(InternalNode::trigger_with_dependencies(
                    &mut sub_context,
                    &mut None,
                    with_successors,
                    dependencies,
                ))
                .await;
                sub_context.end_trace();
                context.push_sub_context(sub_context);
                if result.is_err() {
                    return Err(InternalNodeError::ExecutionFailed(node.id));
                }
            }
        }

        Ok(())
    }
}
