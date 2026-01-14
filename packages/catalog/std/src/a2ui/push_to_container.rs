use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Push To Container - Dynamically adds a widget or component to a container.
///
/// This node allows workflows to dynamically insert elements into container
/// components (rows, columns, stacks, etc.) at runtime.
#[crate::register_node]
#[derive(Default)]
pub struct PushToContainer;

impl PushToContainer {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for PushToContainer {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_push_to_container",
            "Push To Container",
            "Dynamically adds an element to a container's children list",
            "A2UI/Container",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin("exec_in", "▶", "Execution input", VariableType::Execution);

        node.add_input_pin(
            "container_id",
            "Container ID",
            "ID of the container element to add to (e.g., 'my-row', 'main-column')",
            VariableType::String,
        );

        node.add_input_pin(
            "element_id",
            "Element ID",
            "Unique ID for the new element being added",
            VariableType::String,
        );

        node.add_input_pin(
            "element_type",
            "Element Type",
            "Type of element to add (e.g., 'text', 'button', 'image', or 'widget')",
            VariableType::String,
        )
        .set_default_value(Some(json!("text")));

        node.add_input_pin(
            "element_props",
            "Element Props",
            "Properties for the new element (JSON object)",
            VariableType::Generic,
        )
        .set_default_value(Some(json!({})));

        node.add_input_pin(
            "position",
            "Position",
            "Position to insert: -1 for end, 0 for start, or specific index",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(-1)));

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);

        node.add_output_pin(
            "success",
            "Success",
            "Whether the element was successfully added",
            VariableType::Boolean,
        );

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let container_id: String = context.evaluate_pin("container_id").await?;
        let element_id: String = context.evaluate_pin("element_id").await?;
        let element_type: String = context.evaluate_pin("element_type").await?;
        let element_props: Value = context.evaluate_pin("element_props").await?;
        let position: i64 = context.evaluate_pin("position").await?;

        // Build the command payload for the frontend
        let command = json!({
            "type": "push_to_container",
            "container_id": container_id,
            "element": {
                "id": element_id,
                "type": element_type,
                "props": element_props
            },
            "position": position
        });

        // Use upsert_element to send command to frontend (special command element)
        context.upsert_element(&format!("_cmd/{}", container_id), command).await?;

        context
            .get_pin_by_name("success")
            .await?
            .set_value(json!(true))
            .await;

        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
