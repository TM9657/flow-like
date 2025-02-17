use object_store::path::Path;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs::File,
    sync::{Arc, Weak},
};
use tokio::sync::{Mutex, RwLock};

use super::{
    internal_pin::InternalPin, log::LogMessage, trace::Trace, Cacheable, InternalNode, LogLevel,
    Run,
};
use crate::{
    flow::{
        board::ExecutionStage,
        node::{Node, NodeState},
        pin::PinType,
        utils::evaluate_pin_value,
        variable::{Variable, VariableType},
    },
    profile::Profile,
    state::{FlowLikeEvent, FlowLikeState, ToastEvent, ToastLevel},
};

#[derive(Clone)]
pub struct ExecutionContextCache {
    pub user_store: Arc<dyn object_store::ObjectStore>,
    pub user_board_cache: Path,
    pub user_node_cache: Path,
    pub project_store: Arc<dyn object_store::ObjectStore>,
    pub board_cache: Path,
    pub node_cache: Path,
}

impl ExecutionContextCache {
    pub async fn new(
        run: &Weak<Mutex<Run>>,
        state: &Arc<Mutex<FlowLikeState>>,
        node_id: &str,
    ) -> Option<Self> {
        let (board_dir, board_id) = match run.upgrade() {
            Some(run) => {
                let board = &run.lock().await.board;
                (board.board_dir.clone(), board.id.clone())
            }
            None => return None,
        };

        let user_store = state.lock().await.config.read().await.user_store.clone();
        let project_store = state.lock().await.config.read().await.project_store.clone();

        let user_board_cache = Path::from("boards").child(board_id);
        let user_node_cache = user_board_cache.child(node_id);
        let board_cache = board_dir.child("cache");
        let node_cache = board_cache.child(node_id);

        Some(ExecutionContextCache {
            project_store,
            user_store,
            user_board_cache,
            user_node_cache,
            board_cache,
            node_cache,
        })
    }

    pub fn get_tmp_dir(&self) -> anyhow::Result<tempfile::TempDir> {
        let dir = tempfile::tempdir()?;
        Ok(dir)
    }

    pub fn get_tmp_file(&self) -> anyhow::Result<File> {
        let file = tempfile::tempfile()?;
        Ok(file)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
enum RunUpdateEventMethod {
    Add,
    Remove,
    Update,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RunUpdateEvent {
    run_id: String,
    node_ids: Vec<String>,
    method: RunUpdateEventMethod,
}

#[derive(Clone)]
pub struct ExecutionContext {
    pub id: String,
    pub run: Weak<Mutex<Run>>,
    pub profile: Arc<Profile>,
    pub node: Arc<Mutex<InternalNode>>,
    pub sub_traces: Vec<Trace>,
    pub app_state: Arc<Mutex<FlowLikeState>>,
    pub variables: Arc<Mutex<HashMap<String, Variable>>>,
    pub cache: Arc<RwLock<HashMap<String, Arc<dyn Cacheable>>>>,
    pub stage: ExecutionStage,
    pub log_level: LogLevel,
    pub trace: Trace,
    pub execution_cache: Option<ExecutionContextCache>,
    run_id: String,
    state: NodeState,
    sender: tokio::sync::mpsc::Sender<FlowLikeEvent>,
}

impl ExecutionContext {
    pub async fn new(
        run: &Weak<Mutex<Run>>,
        state: &Arc<Mutex<FlowLikeState>>,
        node: &Arc<Mutex<InternalNode>>,
        variables: &Arc<Mutex<HashMap<String, Variable>>>,
        cache: &Arc<RwLock<HashMap<String, Arc<dyn Cacheable>>>>,
        log_level: LogLevel,
        stage: ExecutionStage,
        profile: Arc<Profile>,
        sender: tokio::sync::mpsc::Sender<FlowLikeEvent>,
    ) -> Self {
        let (id, execution_cache) = {
            let guard = node.lock().await;
            let node = guard.node.lock().await;
            let execution_cache = ExecutionContextCache::new(run, state, &node.id).await;
            (node.id.clone(), execution_cache)
        };

        let mut trace = Trace::new(&id);
        if log_level == LogLevel::Debug {
            trace.snapshot_variables(variables).await;
        }

        let run_id = match run.upgrade() {
            Some(run) => {
                let run = run.lock().await;
                run.id.clone()
            }
            None => "".to_string(),
        };

        ExecutionContext {
            id,
            run_id,
            run: run.clone(),
            app_state: state.clone(),
            node: node.clone(),
            variables: variables.clone(),
            cache: cache.clone(),
            log_level,
            stage,
            sub_traces: vec![],
            trace,
            profile,
            sender,
            execution_cache,
            state: NodeState::Idle,
        }
    }

    /// Get directory for storing long term state for the node
    /// e.g if you want to keep track of a directory you can use this directory safely to store the state of the directory.
    pub async fn get_node_dir(&self) -> anyhow::Result<Path> {
        let dir = self
            .execution_cache
            .clone()
            .ok_or(anyhow::anyhow!("Execution cache not found"))?
            .node_cache;
        Ok(dir)
    }

    pub async fn create_sub_context(&self, node: &Arc<Mutex<InternalNode>>) -> ExecutionContext {
        ExecutionContext::new(
            &self.run,
            &self.app_state,
            node,
            &self.variables,
            &self.cache,
            self.log_level.clone(),
            self.stage.clone(),
            self.profile.clone(),
            self.sender.clone(),
        )
        .await
    }

    pub async fn get_variable(&self, variable_id: &str) -> anyhow::Result<Variable> {
        let variables = self.variables.lock().await;
        if let Some(variable) = variables.get(variable_id) {
            return Ok(variable.clone());
        }

        Err(anyhow::anyhow!("Variable not found"))
    }

    pub async fn set_variable(&self, variable: Variable) {
        let mut variables = self.variables.lock().await;
        variables.insert(variable.id.clone(), variable);
    }

    pub async fn set_variable_value(&self, variable_id: &str, value: Value) -> anyhow::Result<()> {
        let mut variables = self.variables.lock().await;
        if let Some(variable) = variables.get_mut(variable_id) {
            let mut guard = variable.value.lock().await;
            *guard = value;
            return Ok(());
        }

        Err(anyhow::anyhow!("Variable not found"))
    }

    pub async fn get_cache(&self, key: &str) -> Option<Arc<dyn Cacheable>> {
        let cache = self.cache.read().await;
        if let Some(value) = cache.get(key) {
            return Some(value.clone());
        }

        None
    }

    pub async fn set_cache(&self, key: &str, value: Arc<dyn Cacheable>) {
        let mut cache = self.cache.write().await;
        cache.insert(key.to_string(), value);
    }

    pub fn log(&mut self, log: LogMessage) {
        if log.log_level < self.log_level {
            return;
        }

        self.trace.logs.push(log);
    }

    pub fn log_message(&mut self, message: &str, log_level: LogLevel) {
        if log_level < self.log_level {
            return;
        }

        let log = LogMessage::new(message, log_level, None);
        self.trace.logs.push(log);
    }

    pub async fn set_state(&mut self, state: NodeState) {
        self.state = state;

        let method = match self.state {
            NodeState::Running => RunUpdateEventMethod::Add,
            _ => RunUpdateEventMethod::Remove,
        };

        let update_event = RunUpdateEvent {
            run_id: self.run_id.clone(),
            node_ids: vec![self.id.clone()],
            method,
        };

        let event = FlowLikeEvent::new(&format!("run:{}", self.run_id), update_event);
        self.sender.send(event).await.unwrap();
    }

    pub fn get_state(&self) -> NodeState {
        self.state.clone()
    }

    pub async fn get_pin_by_name(&self, name: &str) -> anyhow::Result<Arc<Mutex<InternalPin>>> {
        let node = self.node.lock().await;
        let pin = node.get_pin_by_name(name).await?;
        Ok(pin)
    }

    pub async fn evaluate_pin<T: DeserializeOwned>(&self, name: &str) -> anyhow::Result<T> {
        let pin = self.get_pin_by_name(name).await?;
        let value = evaluate_pin_value(pin).await?;
        let value = serde_json::from_value(value)?;
        Ok(value)
    }

    pub async fn get_pins_by_name(
        &self,
        name: &str,
    ) -> anyhow::Result<Vec<Arc<Mutex<InternalPin>>>> {
        let node = self.node.lock().await;
        let pins = node.get_pins_by_name(name).await?;
        Ok(pins)
    }

    pub async fn get_pin_by_id(&self, id: &str) -> anyhow::Result<Arc<Mutex<InternalPin>>> {
        let node = self.node.lock().await;
        let pin = node.get_pin_by_id(id)?;
        Ok(pin)
    }

    pub async fn set_pin_ref_value(
        &self,
        pin: &Arc<Mutex<InternalPin>>,
        value: Value,
    ) -> anyhow::Result<()> {
        let pin = pin.lock().await;
        pin.set_value(value).await;
        Ok(())
    }

    pub async fn set_pin_value(&self, pin: &str, value: Value) -> anyhow::Result<()> {
        let pin = self.get_pin_by_name(pin).await?;
        self.set_pin_ref_value(&pin, value).await
    }

    pub async fn activate_exec_pin(&self, pin: &str) -> anyhow::Result<()> {
        let pin = self.get_pin_by_name(pin).await?;
        self.activate_exec_pin_ref(&pin).await
    }

    pub async fn activate_exec_pin_ref(&self, pin: &Arc<Mutex<InternalPin>>) -> anyhow::Result<()> {
        let pin_guard = pin.lock().await;
        let pin = pin_guard.pin.lock().await;
        if pin.data_type != VariableType::Execution {
            return Err(anyhow::anyhow!("Pin is not of type Execution"));
        }

        if pin.pin_type != PinType::Output {
            return Err(anyhow::anyhow!("Pin is not of type Output"));
        }

        drop(pin);
        pin_guard.set_value(serde_json::json!(true)).await;

        Ok(())
    }

    pub async fn deactivate_exec_pin(&self, pin: &str) -> anyhow::Result<()> {
        let pin = self.get_pin_by_name(pin).await?;
        self.deactivate_exec_pin_ref(&pin).await
    }

    pub async fn deactivate_exec_pin_ref(
        &self,
        pin: &Arc<Mutex<InternalPin>>,
    ) -> anyhow::Result<()> {
        let pin_guard = pin.lock().await;
        let pin = pin_guard.pin.lock().await;
        if pin.data_type != VariableType::Execution {
            return Err(anyhow::anyhow!("Pin is not of type Execution"));
        }

        if pin.pin_type != PinType::Output {
            return Err(anyhow::anyhow!("Pin is not of type Output"));
        }

        drop(pin);
        pin_guard.set_value(serde_json::json!(false)).await;

        Ok(())
    }

    pub fn push_sub_context(&mut self, context: ExecutionContext) {
        let sub_traces = context.get_traces();
        self.sub_traces.extend(sub_traces);
    }

    pub fn end_trace(&mut self) {
        self.trace.finish();
    }

    pub fn get_traces(&self) -> Vec<Trace> {
        let mut traces = self.sub_traces.clone();
        traces.push(self.trace.clone());
        traces.sort_by(|a, b| a.start.cmp(&b.start));
        traces
    }

    pub fn try_get_run(&self) -> anyhow::Result<Arc<Mutex<Run>>> {
        if let Some(run) = self.run.upgrade() {
            return Ok(run);
        }

        Err(anyhow::anyhow!("Run not found"))
    }

    pub async fn read_node(&mut self) -> Node {
        let node_guard = self.node.lock().await;
        let node = node_guard.node.lock().await;

        node.clone()
    }

    pub async fn toast_message(&self, message: &str, level: ToastLevel) -> anyhow::Result<()> {
        let event = FlowLikeEvent::new("toast", ToastEvent::new(message, level));
        self.app_state
            .lock()
            .await
            .event_sender
            .lock()
            .await
            .send(event)
            .await?;
        Ok(())
    }
}
