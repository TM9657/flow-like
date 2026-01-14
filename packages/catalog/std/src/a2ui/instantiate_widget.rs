use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Instantiate Widget - Creates a new instance of a widget for dynamic insertion.
///
/// This node creates a widget instance that can be pushed to containers.
/// The widget is resolved from its reference (app_id + widget_id) and the
/// customization values are applied.
#[crate::register_node]
#[derive(Default)]
pub struct InstantiateWidget;

impl InstantiateWidget {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for InstantiateWidget {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_instantiate_widget",
            "Instantiate Widget",
            "Creates a new widget instance for dynamic insertion into containers",
            "A2UI/Container",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin("exec_in", "▶", "Execution input", VariableType::Execution);

        node.add_input_pin(
            "app_id",
            "App ID",
            "The app ID where the widget is defined",
            VariableType::String,
        );

        node.add_input_pin(
            "widget_id",
            "Widget ID",
            "The unique identifier of the widget template",
            VariableType::String,
        );

        node.add_input_pin(
            "instance_id",
            "Instance ID",
            "Unique ID for this widget instance",
            VariableType::String,
        );

        node.add_input_pin(
            "customization_values",
            "Customizations",
            "Customization values to override defaults (JSON object)",
            VariableType::Generic,
        )
        .set_default_value(Some(json!({})));

        node.add_input_pin(
            "action_bindings",
            "Action Bindings",
            "Mapping of action IDs to workflow references (JSON object)",
            VariableType::Generic,
        )
        .set_default_value(Some(json!({})));

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);

        node.add_output_pin(
            "widget_instance",
            "Widget Instance",
            "The created widget instance data",
            VariableType::Generic,
        );

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let app_id: String = context.evaluate_pin("app_id").await?;
        let widget_id: String = context.evaluate_pin("widget_id").await?;
        let instance_id: String = context.evaluate_pin("instance_id").await?;
        let customization_values: Value = context.evaluate_pin("customization_values").await?;
        let action_bindings: Value = context.evaluate_pin("action_bindings").await?;

        // Build the widget instance structure
        let widget_instance = json!({
            "widgetId": widget_id,
            "instanceId": instance_id,
            "customizationValues": customization_values,
            "actionBindings": action_bindings,
            "widgetRef": {
                "appId": app_id,
                "widgetId": widget_id
            }
        });

        context
            .get_pin_by_name("widget_instance")
            .await?
            .set_value(widget_instance)
            .await;

        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
