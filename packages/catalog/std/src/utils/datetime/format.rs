use chrono::{DateTime, Utc};
use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{Value, async_trait, json::json};
use serde::{Deserialize, Serialize};

#[crate::register_node]
#[derive(Default)]
pub struct DateTimeFormatNode {}

impl DateTimeFormatNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SystemTime {
    secs_since_epoch: i64,
    nanos_since_epoch: u32,
}

#[async_trait]
impl NodeLogic for DateTimeFormatNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "utils_datetime_format",
            "Format DateTime",
            "Converts a DateTime to a formatted string",
            "Utils/DateTime",
        );

        node.add_input_pin("date", "Date", "Date to format", VariableType::Date);
        node.add_input_pin(
            "format",
            "Format",
            "Format string (e.g., '%Y-%m-%d %H:%M:%S', '%Y-%m-%d', 'rfc3339', 'rfc2822')",
            VariableType::String,
        )
        .set_default_value(Some(json!("rfc3339")));

        node.add_output_pin(
            "formatted",
            "Formatted",
            "Formatted string",
            VariableType::String,
        );
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let dt: DateTime<Utc> = context.evaluate_pin("date").await?;
        let format_str: String = context.evaluate_pin("format").await?;

        let formatted = match format_str.to_lowercase().as_str() {
            "rfc3339" => dt.to_rfc3339(),
            "rfc2822" => dt.to_rfc2822(),
            _ => match dt.format(&format_str).to_string() {
                s => s,
            },
        };

        context.set_pin_value("formatted", json!(formatted)).await?;

        Ok(())
    }
}
