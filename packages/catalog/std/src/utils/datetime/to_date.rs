use chrono::{DateTime, Datelike, Utc};
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
pub struct DateTimeToDateNode {}

impl DateTimeToDateNode {
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
impl NodeLogic for DateTimeToDateNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "utils_datetime_to_date",
            "To Date",
            "Extracts date components from a DateTime",
            "Utils/DateTime",
        );

        node.add_input_pin(
            "date",
            "Date",
            "DateTime to extract from",
            VariableType::Date,
        );

        node.add_output_pin("year", "Year", "Year", VariableType::Integer);
        node.add_output_pin("month", "Month", "Month (1-12)", VariableType::Integer);
        node.add_output_pin("day", "Day", "Day of month (1-31)", VariableType::Integer);
        node.add_output_pin(
            "weekday",
            "Weekday",
            "Day of week (0=Monday, 6=Sunday)",
            VariableType::Integer,
        );
        node.add_output_pin(
            "day_of_year",
            "Day of Year",
            "Day of year (1-366)",
            VariableType::Integer,
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("error").await?;

        let dt: DateTime<Utc> = context.evaluate_pin("date").await?;

        context
            .set_pin_value("year", json!(dt.year() as i64))
            .await?;
        context
            .set_pin_value("month", json!(dt.month() as i64))
            .await?;
        context.set_pin_value("day", json!(dt.day() as i64)).await?;
        context
            .set_pin_value("weekday", json!(dt.weekday().num_days_from_monday() as i64))
            .await?;
        context
            .set_pin_value("day_of_year", json!(dt.ordinal() as i64))
            .await?;

        Ok(())
    }
}
