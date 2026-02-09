use flow_like::flow::{
    board::Board,
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json};
use std::sync::Arc;

/// Widget Action Event - Entry point for widget action triggers.
///
/// This node acts as an entry point when a widget action is triggered in the UI.
/// The action context (data provided by the widget) is passed through the payload
/// and can be accessed via output pins.
///
/// Use this instead of Simple Event when you need context from widget actions.
#[crate::register_node]
#[derive(Default)]
pub struct WidgetActionEvent;

impl WidgetActionEvent {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for WidgetActionEvent {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "events_widget_action",
            "Widget Action Event",
            "Entry point triggered when a widget action is invoked. Provides action context data.",
            "Events",
        );
        node.add_icon("/flow/icons/event.svg");
        node.set_start(true);
        node.set_can_be_referenced_by_fns(true);

        node.add_input_pin(
            "action_id",
            "Action ID",
            "The action identifier that triggers this event (e.g., 'clicked_delete', 'clicked_open')",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_output_pin(
            "exec_out",
            "Output",
            "Triggered when the widget action is invoked",
            VariableType::Execution,
        );

        node.add_output_pin(
            "widget_instance_id",
            "Widget Instance ID",
            "The unique ID of the widget instance that triggered the action",
            VariableType::String,
        );

        node.add_output_pin(
            "action_context",
            "Action Context",
            "The context data passed from the widget action (JSON object with field values)",
            VariableType::Generic,
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let payload = context.get_payload().await?;

        // Extract widget action context from payload
        let widget_instance_id = payload
            .payload
            .as_ref()
            .and_then(|p| p.get("_widget_instance_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let action_context = payload
            .payload
            .as_ref()
            .and_then(|p| p.get("_action_context"))
            .cloned()
            .unwrap_or(json!({}));

        // Set output pins
        context
            .get_pin_by_name("widget_instance_id")
            .await?
            .set_value(json!(widget_instance_id))
            .await;

        context
            .get_pin_by_name("action_context")
            .await?
            .set_value(action_context)
            .await;

        // Activate execution flow
        let exec_out_pin = context.get_pin_by_name("exec_out").await?;
        context.activate_exec_pin_ref(&exec_out_pin).await?;

        Ok(())
    }

    async fn on_update(&self, node: &mut Node, _board: Arc<Board>) {
        node.error = None;

        // Validate action_id is not empty when provided
        let action_id = node
            .get_pin_by_name("action_id")
            .and_then(|pin| pin.default_value.as_ref())
            .and_then(|v| {
                let parsed: flow_like_types::Value = flow_like_types::json::from_slice(v).ok()?;
                parsed.as_str().map(String::from)
            })
            .unwrap_or_default();

        if action_id.is_empty() {
            node.error = Some("Action ID should be specified to identify this event".to_string());
        }
    }
}
