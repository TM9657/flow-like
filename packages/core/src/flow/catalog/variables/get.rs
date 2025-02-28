use crate::{
    flow::{
        board::Board,
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use async_trait::async_trait;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

#[derive(Default)]

pub struct GetVariable {}

impl GetVariable {
    pub fn new() -> Self {
        GetVariable {}
    }

    pub fn push_registry(registry: &mut HashMap<&'static str, Arc<Mutex<dyn NodeLogic>>>) {
        let node = GetVariable::new();
        let node = Arc::new(Mutex::new(node));
        registry.insert("variable_get", node);
    }
}

#[async_trait]
impl NodeLogic for GetVariable {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "variable_get",
            "Get Variable",
            "Get Variable Value",
            "Variable",
        );

        node.add_icon("/flow/icons/variable.svg");

        node.add_input_pin(
            "var_ref",
            "Variable Reference",
            "The reference to the variable",
            VariableType::String,
        );

        node.add_output_pin(
            "value_ref",
            "Value",
            "The value of the variable",
            VariableType::Generic,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let var_ref: String = context.evaluate_pin("var_ref").await?;
        let variable: crate::flow::variable::Variable = context.get_variable(&var_ref).await?;

        let value_pin = context.get_pin_by_name("value_ref").await?;
        let value = variable.get_value();

        context.log_message(
            &format!("Got Value: {}", value.lock().await),
            crate::flow::execution::LogLevel::Debug,
        );

        value_pin.lock().await.set_pin_by_ref(value).await;
        Ok(())
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        node.error = None;

        let read_only_node = node.clone();
        let var_ref = match read_only_node.get_pin_by_name("var_ref") {
            Some(pin) => pin,
            None => {
                node.error = Some("Variable not found!".to_string());
                return;
            }
        };

        let var_ref_value = match var_ref.default_value.as_ref().and_then(|v| {
            let parsed: Value = serde_json::from_slice(v).unwrap();
            parsed.as_str().map(String::from)
        }) {
            Some(val) => val,
            None => {
                node.error = Some("Variable reference not found!".to_string());
                return;
            }
        };

        let var_ref_variable = match board.get_variable(&var_ref_value) {
            Some(var) => var,
            None => {
                node.error = Some("Variable not found!".to_string());
                return;
            }
        };

        node.friendly_name = format!("Get {}", &var_ref_variable.name);
        let mut_value = match node.get_pin_mut_by_name("value_ref") {
            Some(val) => val,
            None => {
                node.error = Some("Value pin not found!".to_string());
                return;
            }
        };
        let immutable_value = mut_value.clone();

        mut_value.data_type = var_ref_variable.data_type.clone();
        mut_value.value_type = var_ref_variable.value_type.clone();

        if immutable_value.connected_to.is_empty() {
            return;
        }

        let mut connected = immutable_value.connected_to.clone();

        connected.retain(|conn| {
            board.get_pin_by_id(conn).map_or(false, |pin| {
                pin.data_type == mut_value.data_type && pin.value_type == mut_value.value_type
            })
        });

        mut_value.connected_to = connected;
    }
}
