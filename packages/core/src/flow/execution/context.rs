use super::{
    EventTrigger, InternalNode, LogLevel, Run, RunPayload, internal_pin::InternalPin,
    log::LogMessage, trace::Trace,
};
use crate::{
    flow::{
        board::ExecutionStage,
        node::{Node, NodeState},
        pin::PinType,
        utils::{evaluate_pin_value, evaluate_pin_value_reference},
        variable::{Variable, VariableType},
    },
    profile::Profile,
    state::{FlowLikeState, FlowLikeStores, ToastEvent, ToastLevel},
};
use flow_like_model_provider::provider::ModelProviderConfiguration;
use flow_like_storage::object_store::path::Path;
use flow_like_types::Value;
use flow_like_types::intercom::{InterComCallback, InterComEvent};
use flow_like_types::{
    Cacheable,
    json::from_value,
    sync::{Mutex, RwLock},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};

#[derive(Clone)]
pub struct ExecutionContextCache {
    pub stores: FlowLikeStores,
    pub app_id: String,
    pub board_dir: Path,
    pub board_id: String,
    pub node_id: String,
    pub sub: String,
}

impl ExecutionContextCache {
    pub async fn new(
        run: &Weak<Mutex<Run>>,
        state: &Arc<Mutex<FlowLikeState>>,
        node_id: &str,
    ) -> Option<Self> {
        let (app_id, board_dir, board_id, sub) = match run.upgrade() {
            Some(run) => {
                let run = run.lock().await;
                let app_id = run.app_id.clone();
                let board = &run.board;
                let sub = run.sub.clone();
                (app_id, board.board_dir.clone(), board.id.clone(), sub)
            }
            None => return None,
        };

        let stores = state.lock().await.config.read().await.stores.clone();

        Some(ExecutionContextCache {
            stores,
            app_id,
            board_dir,
            board_id,
            node_id: node_id.to_string(),
            sub,
        })
    }

    pub fn get_user_dir(&self, node: bool) -> flow_like_types::Result<Path> {
        let base = Path::from("users")
            .child(self.sub.clone())
            .child("apps")
            .child(self.app_id.clone());
        if !node {
            return Ok(base);
        }

        Ok(base.child(self.node_id.clone()))
    }

    pub fn get_cache(&self, node: bool, user: bool) -> flow_like_types::Result<Path> {
        let mut base = Path::from("tmp");

        if user {
            base = base.child("user").child(self.sub.clone());
        } else {
            base = base.child("global");
        }

        base = base.child("apps").child(self.app_id.clone());

        if !node {
            return Ok(base);
        }

        Ok(base.child(self.node_id.clone()))
    }

    pub fn get_storage(&self, node: bool) -> flow_like_types::Result<Path> {
        let base = self.board_dir.child("storage");

        if !node {
            return Ok(base);
        }

        Ok(base.child(self.node_id.clone()))
    }

    pub fn get_upload_dir(&self) -> flow_like_types::Result<Path> {
        let base = self.board_dir.child("upload");
        Ok(base)
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
    pub node: Arc<InternalNode>,
    pub sub_traces: Vec<Trace>,
    pub app_state: Arc<Mutex<FlowLikeState>>,
    pub variables: Arc<Mutex<HashMap<String, Variable>>>,
    pub cache: Arc<RwLock<HashMap<String, Arc<dyn Cacheable>>>>,
    pub stage: ExecutionStage,
    pub log_level: LogLevel,
    pub trace: Trace,
    pub execution_cache: Option<ExecutionContextCache>,
    pub completion_callbacks: Arc<RwLock<Vec<EventTrigger>>>,
    pub stream_state: bool,
    run_id: String,
    state: NodeState,
    callback: InterComCallback,
}

impl ExecutionContext {
    pub async fn new(
        run: &Weak<Mutex<Run>>,
        state: &Arc<Mutex<FlowLikeState>>,
        node: &Arc<InternalNode>,
        variables: &Arc<Mutex<HashMap<String, Variable>>>,
        cache: &Arc<RwLock<HashMap<String, Arc<dyn Cacheable>>>>,
        log_level: LogLevel,
        stage: ExecutionStage,
        profile: Arc<Profile>,
        callback: InterComCallback,
        completion_callbacks: Arc<RwLock<Vec<EventTrigger>>>,
    ) -> Self {
        let (id, execution_cache) = {
            let node_id = node.node.lock().await.id.clone();
            let execution_cache = ExecutionContextCache::new(run, state, &node_id).await;
            (node_id, execution_cache)
        };

        let mut trace = Trace::new(&id);
        if log_level == LogLevel::Debug {
            trace.snapshot_variables(variables).await;
        }

        let (run_id, stream_state) = match run.upgrade() {
            Some(run) => {
                let run = run.lock().await;
                (run.id.clone(), run.stream_state)
            }
            None => ("".to_string(), false),
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
            callback,
            execution_cache,
            stream_state,
            state: NodeState::Idle,
            completion_callbacks,
        }
    }

    pub async fn create_sub_context(&self, node: &Arc<InternalNode>) -> ExecutionContext {
        ExecutionContext::new(
            &self.run,
            &self.app_state,
            node,
            &self.variables,
            &self.cache,
            self.log_level,
            self.stage.clone(),
            self.profile.clone(),
            self.callback.clone(),
            self.completion_callbacks.clone(),
        )
        .await
    }

    pub async fn get_variable(&self, variable_id: &str) -> flow_like_types::Result<Variable> {
        if let Some(variable) = self.variables.lock().await.get(variable_id).cloned() {
            return Ok(variable);
        }

        Err(flow_like_types::anyhow!("Variable not found"))
    }

    pub async fn get_payload(&self) -> flow_like_types::Result<Arc<RunPayload>> {
        let payload = self
            .run
            .upgrade()
            .ok_or_else(|| flow_like_types::anyhow!("Run not found"))?
            .lock()
            .await
            .payload
            .clone();

        if payload.id == self.id {
            return Ok(payload);
        }
        Err(flow_like_types::anyhow!("Payload not found"))
    }

    pub async fn hook_completion_event(&mut self, cb: EventTrigger) {
        let mut callbacks = self.completion_callbacks.write().await;
        callbacks.push(cb);
    }

    pub async fn set_variable(&self, variable: Variable) {
        let mut variables = self.variables.lock().await;
        variables.insert(variable.id.clone(), variable);
    }

    pub async fn set_variable_value(
        &self,
        variable_id: &str,
        value: Value,
    ) -> flow_like_types::Result<()> {
        let value_ref = self
            .variables
            .lock()
            .await
            .get(variable_id)
            .ok_or(flow_like_types::anyhow!("Variable not found"))?
            .value
            .clone();
        let mut guard = value_ref.lock().await;
        *guard = value;
        Ok(())
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

        let mut log = log;
        log.node_id = Some(self.trace.node_id.clone());
        self.trace.logs.push(log);
    }

    pub fn log_message(&mut self, message: &str, log_level: LogLevel) {
        if log_level < self.log_level {
            return;
        }

        let mut log = LogMessage::new(message, log_level, None);
        log.node_id = Some(self.trace.node_id.clone());
        self.trace.logs.push(log);
    }

    pub async fn set_state(&mut self, state: NodeState) {
        self.state = state;

        let method = match self.state {
            NodeState::Running => RunUpdateEventMethod::Add,
            _ => RunUpdateEventMethod::Remove,
        };

        if !self.stream_state {
            return;
        }

        let update_event = RunUpdateEvent {
            run_id: self.run_id.clone(),
            node_ids: vec![self.id.clone()],
            method,
        };

        let event = InterComEvent::with_type(format!("run:{}", self.run_id), update_event);

        if let Err(err) = event.call(&self.callback).await {
            self.log_message(
                &format!("Failed to send run update event: {}", err),
                LogLevel::Error,
            );
        }
    }

    pub fn get_state(&self) -> NodeState {
        self.state.clone()
    }

    pub async fn get_pin_by_name(
        &self,
        name: &str,
    ) -> flow_like_types::Result<Arc<Mutex<InternalPin>>> {
        let pin = self.node.get_pin_by_name(name).await?;
        Ok(pin)
    }

    pub async fn get_model_config(
        &self,
    ) -> flow_like_types::Result<Arc<ModelProviderConfiguration>> {
        let config = self.app_state.lock().await.model_provider_config.clone();
        Ok(config)
    }

    pub async fn evaluate_pin<T: DeserializeOwned>(
        &self,
        name: &str,
    ) -> flow_like_types::Result<T> {
        let pin = self.get_pin_by_name(name).await?;
        let value = evaluate_pin_value(pin).await?;
        let value = from_value(value)?;
        Ok(value)
    }

    pub async fn evaluate_pin_to_ref(
        &self,
        name: &str,
    ) -> flow_like_types::Result<Arc<Mutex<Value>>> {
        let pin = self.get_pin_by_name(name).await?;
        let value = evaluate_pin_value_reference(pin).await?;
        Ok(value)
    }

    pub async fn evaluate_pin_ref<T: DeserializeOwned>(
        &self,
        reference: Arc<Mutex<InternalPin>>,
    ) -> flow_like_types::Result<T> {
        let value = evaluate_pin_value(reference).await?;
        let value = from_value(value)?;
        Ok(value)
    }

    pub async fn get_pins_by_name(
        &self,
        name: &str,
    ) -> flow_like_types::Result<Vec<Arc<Mutex<InternalPin>>>> {
        let pins = self.node.get_pins_by_name(name).await?;
        Ok(pins)
    }

    pub async fn get_pin_by_id(
        &self,
        id: &str,
    ) -> flow_like_types::Result<Arc<Mutex<InternalPin>>> {
        let pin = self.node.get_pin_by_id(id)?;
        Ok(pin)
    }

    pub async fn set_pin_ref_value(
        &self,
        pin: &Arc<Mutex<InternalPin>>,
        value: Value,
    ) -> flow_like_types::Result<()> {
        let pin = pin.lock().await;
        pin.set_value(value).await;
        Ok(())
    }

    pub async fn set_pin_value(&self, pin: &str, value: Value) -> flow_like_types::Result<()> {
        let pin = self.get_pin_by_name(pin).await?;
        self.set_pin_ref_value(&pin, value).await
    }

    pub async fn activate_exec_pin(&self, pin: &str) -> flow_like_types::Result<()> {
        let pin = self.get_pin_by_name(pin).await?;
        self.activate_exec_pin_ref(&pin).await
    }

    pub async fn activate_exec_pin_ref(
        &self,
        pin: &Arc<Mutex<InternalPin>>,
    ) -> flow_like_types::Result<()> {
        let pin_guard = pin.lock().await;
        let pin = pin_guard.pin.lock().await;
        if pin.data_type != VariableType::Execution {
            return Err(flow_like_types::anyhow!("Pin is not of type Execution"));
        }

        if pin.pin_type != PinType::Output {
            return Err(flow_like_types::anyhow!("Pin is not of type Output"));
        }

        drop(pin);
        pin_guard
            .set_value(flow_like_types::json::json!(true))
            .await;

        Ok(())
    }

    pub async fn deactivate_exec_pin(&self, pin: &str) -> flow_like_types::Result<()> {
        let pin = self.get_pin_by_name(pin).await?;
        self.deactivate_exec_pin_ref(&pin).await
    }

    pub async fn deactivate_exec_pin_ref(
        &self,
        pin: &Arc<Mutex<InternalPin>>,
    ) -> flow_like_types::Result<()> {
        let pin_guard = pin.lock().await;
        let pin = pin_guard.pin.lock().await;
        if pin.data_type != VariableType::Execution {
            return Err(flow_like_types::anyhow!("Pin is not of type Execution"));
        }

        if pin.pin_type != PinType::Output {
            return Err(flow_like_types::anyhow!("Pin is not of type Output"));
        }

        drop(pin);
        pin_guard
            .set_value(flow_like_types::json::json!(false))
            .await;

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

    pub fn try_get_run(&self) -> flow_like_types::Result<Arc<Mutex<Run>>> {
        if let Some(run) = self.run.upgrade() {
            return Ok(run);
        }

        Err(flow_like_types::anyhow!("Run not found"))
    }

    pub async fn read_node(&self) -> Node {
        let node = self.node.node.lock().await;

        node.clone()
    }

    pub async fn toast_message(
        &mut self,
        message: &str,
        level: ToastLevel,
    ) -> flow_like_types::Result<()> {
        let event = InterComEvent::with_type("toast", ToastEvent::new(message, level));
        if let Err(err) = event.call(&self.callback).await {
            self.log_message(
                &format!("Failed to send toast event: {}", err),
                LogLevel::Error,
            );
        }
        Ok(())
    }

    pub async fn stream_response<T>(
        &mut self,
        event_type: &str,
        event: T,
    ) -> flow_like_types::Result<()>
    where
        T: Serialize + DeserializeOwned,
    {
        let event = InterComEvent::with_type(event_type, event);
        if let Err(err) = event.call(&self.callback).await {
            self.log_message(&format!("Failed to send event: {}", err), LogLevel::Error);
        }
        Ok(())
    }
}
