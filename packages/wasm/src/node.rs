//! WASM Node Logic implementation
//!
//! Bridges WASM modules to the Flow-Like NodeLogic trait.

use crate::abi::{WasmExecutionInput, WasmNodeDefinition, WasmPinDefinition};
use crate::engine::WasmEngine;
use crate::error::WasmResult;
use crate::host_functions::ExecutionMetadata;
use crate::instance::WasmInstance;
use crate::limits::WasmSecurityConfig;
use crate::module::WasmModule;
use async_trait::async_trait;
use flow_like::flow::execution::context::ExecutionContext;
use flow_like::flow::execution::{LogLevel, Run};
use flow_like::flow::node::{Node, NodeLogic, NodeScores};
use flow_like::flow::pin::{Pin, PinType, ValueType};
use flow_like::flow::variable::VariableType;
use flow_like_types::{Value, sync::Mutex, tokio::sync::RwLock};
use std::collections::BTreeSet;
use std::sync::Arc;

pub struct WasmNodeLogic {
    module: Arc<WasmModule>,
    engine: Arc<WasmEngine>,
    security: WasmSecurityConfig,
    cached_definition: RwLock<Option<WasmNodeDefinition>>,
}

impl WasmNodeLogic {
    pub fn new(module: Arc<WasmModule>, engine: Arc<WasmEngine>, security: WasmSecurityConfig) -> Self {
        Self {
            module,
            engine,
            security,
            cached_definition: RwLock::new(None),
        }
    }

    async fn get_definition(&self) -> WasmResult<WasmNodeDefinition> {
        {
            let cached = self.cached_definition.read().await;
            if let Some(def) = cached.as_ref() {
                return Ok(def.clone());
            }
        }

        let def: WasmNodeDefinition = self.module.get_node_definition(&self.engine, &self.security).await?;

        {
            let mut cache = self.cached_definition.write().await;
            *cache = Some(def.clone());
        }

        Ok(def)
    }

    fn to_flow_pin(wasm_pin: &WasmPinDefinition, index: u16) -> Pin {
        let data_type = map_wasm_data_type(&wasm_pin.data_type);
        let pin_type = match wasm_pin.pin_type.to_lowercase().as_str() {
            "output" => PinType::Output,
            _ => PinType::Input,
        };

        let value_type = wasm_pin.value_type.as_deref()
            .map(|vt| match vt.to_lowercase().as_str() {
                "array" => ValueType::Array,
                "hashmap" => ValueType::HashMap,
                "hashset" => ValueType::HashSet,
                _ => ValueType::Normal,
            })
            .unwrap_or(ValueType::Normal);

        let default_value = wasm_pin.default_value.as_ref()
            .and_then(|v| flow_like_types::json::to_vec(v).ok());

        Pin {
            id: flow_like_types::create_id(),
            name: wasm_pin.name.clone(),
            friendly_name: wasm_pin.friendly_name.clone(),
            description: wasm_pin.description.clone(),
            pin_type,
            data_type,
            schema: wasm_pin.schema.clone(),
            value_type,
            depends_on: BTreeSet::new(),
            connected_to: BTreeSet::new(),
            default_value,
            index,
            options: None,
            value: None,
        }
    }
}

fn map_wasm_data_type(wasm_type: &str) -> VariableType {
    match wasm_type.to_lowercase().as_str() {
        "string" => VariableType::String,
        "int" | "integer" | "i32" | "i64" => VariableType::Integer,
        "float" | "f32" | "f64" | "number" => VariableType::Float,
        "bool" | "boolean" => VariableType::Boolean,
        "date" | "datetime" => VariableType::Date,
        "path" | "pathbuf" => VariableType::PathBuf,
        "byte" | "bytes" | "binary" => VariableType::Byte,
        "exec" | "execution" => VariableType::Execution,
        "struct" | "object" => VariableType::Struct,
        _ => VariableType::Generic,
    }
}

#[async_trait]
impl NodeLogic for WasmNodeLogic {
    fn get_node(&self) -> Node {
        let rt = flow_like_types::tokio::runtime::Handle::try_current();

        let definition = if let Ok(handle) = rt {
            handle.block_on(async { self.get_definition().await.ok() })
        } else {
            None
        };

        let definition = definition.unwrap_or_else(|| WasmNodeDefinition {
            name: "wasm_node".to_string(),
            friendly_name: "WASM Node".to_string(),
            description: "A WebAssembly node".to_string(),
            category: "WASM".to_string(),
            pins: vec![],
            icon: None,
            scores: None,
            long_running: None,
            docs: None,
            abi_version: None,
        });

        let mut node = Node::new(
            &definition.name,
            &definition.friendly_name,
            &definition.description,
            &definition.category,
        );

        for (i, wasm_pin) in definition.pins.iter().enumerate() {
            let pin = Self::to_flow_pin(wasm_pin, i as u16);
            node.pins.insert(pin.id.clone(), pin);
        }

        if let Some(icon) = &definition.icon {
            node.icon = Some(icon.clone());
        }

        if let Some(scores) = &definition.scores {
            node.scores = Some(NodeScores {
                privacy: scores.privacy,
                security: scores.security,
                performance: scores.performance,
                governance: scores.governance,
                reliability: scores.reliability,
                cost: scores.cost,
            });
        }

        if definition.long_running.unwrap_or(false) {
            node.long_running = Some(true);
        }

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let mut instance = WasmInstance::new(&self.engine, self.module.clone(), self.security.clone())
            .await
            .map_err(|e| flow_like_types::anyhow!("Failed to create WASM instance: {}", e))?;

        let definition = self.get_definition().await
            .map_err(|e| flow_like_types::anyhow!("Failed to get node definition: {}", e))?;

        // Collect input values
        let mut inputs = serde_json::Map::new();
        for pin in &definition.pins {
            if pin.pin_type.to_lowercase() == "input" && pin.data_type.to_lowercase() != "execution" {
                if let Ok(pin_ref) = context.get_pin_by_name(&pin.name).await {
                    if let Some(val) = pin_ref.get_raw_value().await {
                        inputs.insert(pin.name.clone(), val);
                    }
                }
            }
        }

        // Set up host state
        let host_state = instance.host_state_mut();
        let inputs_for_state: std::collections::HashMap<String, Value> = inputs.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        host_state.set_inputs(inputs_for_state);

        // Build run_id
        let run_id: String = context.run.upgrade()
            .and_then(|r: Arc<Mutex<Run>>| r.try_lock().ok().map(|run| run.id.clone()))
            .unwrap_or_default();

        // Get node_id from context
        let node_id = context.id.clone();

        host_state.metadata = ExecutionMetadata {
            node_id: node_id.clone(),
            run_id: run_id.clone(),
            app_id: String::new(),
            board_id: String::new(),
            user_id: String::new(),
            stream_state: context.stream_state,
            log_level: context.log_level.clone() as u8,
        };

        // Execute
        let exec_input = WasmExecutionInput {
            inputs,
            node_id,
            run_id,
            app_id: String::new(),
            board_id: String::new(),
            user_id: String::new(),
            stream_state: context.stream_state,
            log_level: context.log_level.clone() as u8,
            node_name: definition.name.clone(),
        };

        let result = instance.call_run(&exec_input).await
            .map_err(|e| flow_like_types::anyhow!("WASM execution failed: {}", e))?;

        // Process outputs
        for (name, value) in result.outputs {
            context.set_pin_value(&name, value).await?;
        }

        // Activate exec pins
        for pin_name in &result.activate_exec {
            context.activate_exec_pin(pin_name).await?;
        }

        // Process logs
        for log in instance.host_state().get_logs() {
            let level = match log.level {
                0..=1 => LogLevel::Debug,
                2 => LogLevel::Info,
                3 => LogLevel::Warn,
                _ => LogLevel::Error,
            };
            context.log_message(&log.message, level);
        }

        // Process stream events
        for event in instance.host_state().take_stream_events() {
            if event.event_type == "text" {
                if let Some(text) = event.data.as_str() {
                    context.stream_response("wasm_text", text.to_string()).await?;
                }
            }
        }

        // Check for errors
        if let Some(error) = instance.host_state().get_error() {
            return Err(flow_like_types::anyhow!("WASM node error: {}", error));
        }

        if let Some(error) = result.error {
            return Err(flow_like_types::anyhow!("WASM execution error: {}", error));
        }

        Ok(())
    }

    async fn on_drop(&self) {}
}

impl std::fmt::Debug for WasmNodeLogic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmNodeLogic")
            .field("module_hash", &self.module.hash())
            .finish()
    }
}
