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

pub struct SetVariable {}

impl SetVariable {
    pub fn new() -> Self {
        SetVariable {}
    }

    pub fn push_registry(registry: &mut HashMap<&'static str, Arc<Mutex<dyn NodeLogic>>>) {
        let node = SetVariable::new();
        let node = Arc::new(Mutex::new(node));
        registry.insert("variable_set", node);
    }
}

#[async_trait]
impl NodeLogic for SetVariable {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "variable_set",
            "Set Variable",
            "Set Variable Value",
            "Variable",
        );

        node.add_icon("/flow/icons/variable.svg");

        node.add_input_pin("exec_in", "Input", "Trigger Pin", VariableType::Execution);

        node.add_input_pin(
            "var_ref",
            "Variable Reference",
            "The reference to the variable",
            VariableType::String,
        );

        node.add_input_pin(
            "value_ref",
            "Value",
            "The value of the variable",
            VariableType::Generic,
        );

        node.add_output_pin(
            "exec_out",
            "Output",
            "Triggering once the variable value was set",
            VariableType::Execution,
        );

        node.add_output_pin(
            "new_value",
            "New Value",
            "The newly set value",
            VariableType::Generic,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let var_ref: String = context.evaluate_pin("var_ref").await?;
        let value = context.evaluate_pin::<Value>("value_ref").await?;

        context.set_variable_value(&var_ref, value).await?;

        context.activate_exec_pin("exec_out").await?;
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

        node.friendly_name = format!("Set {}", &var_ref_variable.name);
        let mut_value = match node.get_pin_mut_by_name("value_ref") {
            Some(val) => val,
            None => {
                node.error = Some("Value pin not found!".to_string());
                return;
            }
        };
        mut_value.data_type = var_ref_variable.data_type.clone();
        mut_value.value_type = var_ref_variable.value_type.clone();
        if !mut_value.depends_on.is_empty() {
            let mut dependencies = mut_value.depends_on.clone();
            dependencies.retain(|deps| {
                board.get_pin_by_id(deps).is_some_and(|pin| {
                    pin.data_type == mut_value.data_type && pin.value_type == mut_value.value_type
                })
            });

            mut_value.depends_on = dependencies;
        }

        let mut_new_value = match node.get_pin_mut_by_name("new_value") {
            Some(val) => val,
            None => {
                node.error = Some("New Value pin not found!".to_string());
                return;
            }
        };

        mut_new_value.data_type = var_ref_variable.data_type.clone();
        mut_new_value.value_type = var_ref_variable.value_type.clone();

        if !var_ref.connected_to.is_empty() {
            let mut connected = var_ref.connected_to.clone();

            connected.retain(|conn| {
                board.get_pin_by_id(conn).is_some_and(|pin| {
                    pin.data_type == mut_new_value.data_type
                        && pin.value_type == mut_new_value.value_type
                })
            });

            mut_new_value.connected_to = connected;
        }
    }
}
