use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json, Value};
use super::chart_data_utils::{extract_from_csv_table, parse_csv_text, parse_column_ref};

/// Converts CSV data or CSVTable (from DataFusion) to Nivo Calendar chart format.
///
/// **Output Format:** `[{ day: "2024-01-15", value: 50 }, ...]`
///
/// **Documentation:** https://nivo.rocks/calendar/
///
/// **Accepts:**
/// - Raw CSV text with headers
/// - CSVTable struct from DataFusion SQL queries
#[crate::register_node]
#[derive(Default)]
pub struct CsvToCalendarData;

impl CsvToCalendarData {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for CsvToCalendarData {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_csv_to_calendar_data",
            "CSV to Calendar Data",
            "Converts CSV or DataFusion CSVTable to Nivo Calendar heatmap format. Docs: https://nivo.rocks/calendar/",
            "A2UI/Elements/Charts/Calendar",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin(
            "csv",
            "CSV",
            "CSV with date and value columns. Dates must be YYYY-MM-DD format",
            VariableType::String,
        )
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_input_pin(
            "table",
            "Table",
            "Alternative: CSVTable from DataFusion query",
            VariableType::Struct,
        )
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_input_pin(
            "date_column",
            "Date Column",
            "Column name or 0-based index for dates (default: 0)",
            VariableType::String,
        )
        .set_default_value(Some(json!("0")));

        node.add_input_pin(
            "value_column",
            "Value Column",
            "Column name or 0-based index for values (default: 1)",
            VariableType::String,
        )
        .set_default_value(Some(json!("1")));

        node.add_input_pin(
            "delimiter",
            "Delimiter",
            "Column delimiter for CSV text (default: comma)",
            VariableType::String,
        )
        .set_default_value(Some(json!(",")));

        node.add_output_pin("data", "Data", "Calendar chart data array", VariableType::Generic);
        node.add_output_pin("from_date", "From Date", "Earliest date in the data (YYYY-MM-DD)", VariableType::String);
        node.add_output_pin("to_date", "To Date", "Latest date in the data (YYYY-MM-DD)", VariableType::String);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let date_col: String = context.evaluate_pin("date_column").await?;
        let value_col: String = context.evaluate_pin("value_column").await?;
        let delimiter: String = context.evaluate_pin("delimiter").await?;

        let (headers, rows) = if let Ok(table_value) = context.evaluate_pin::<Value>("table").await {
            if !table_value.is_null() {
                extract_from_csv_table(&table_value)?
            } else {
                let csv_text: String = context.evaluate_pin("csv").await?;
                parse_csv_text(&csv_text, delimiter.chars().next().unwrap_or(','))?
            }
        } else {
            let csv_text: String = context.evaluate_pin("csv").await?;
            parse_csv_text(&csv_text, delimiter.chars().next().unwrap_or(','))?
        };

        if headers.is_empty() || rows.is_empty() {
            context.set_pin_value("data", json!([])).await?;
            context.set_pin_value("from_date", json!("")).await?;
            context.set_pin_value("to_date", json!("")).await?;
            return Ok(());
        }

        let date_idx = parse_column_ref(&date_col, &headers);
        let value_idx = parse_column_ref(&value_col, &headers);

        let mut min_date: Option<String> = None;
        let mut max_date: Option<String> = None;

        let data: Vec<Value> = rows.iter().filter_map(|row| {
            let day = row.get(date_idx)?.clone();
            let value: f64 = row.get(value_idx).and_then(|s| s.parse().ok()).unwrap_or(0.0);

            // Validate date format (basic check) - support YYYY-MM-DD or ISO datetime
            let day_str = if day.len() >= 10 && day.chars().nth(4) == Some('-') && day.chars().nth(7) == Some('-') {
                day[..10].to_string()
            } else {
                return None;
            };

            if min_date.is_none() || day_str < *min_date.as_ref().unwrap() {
                min_date = Some(day_str.clone());
            }
            if max_date.is_none() || day_str > *max_date.as_ref().unwrap() {
                max_date = Some(day_str.clone());
            }
            Some(json!({ "day": day_str, "value": value }))
        }).collect();

        context.set_pin_value("data", json!(data)).await?;
        context.set_pin_value("from_date", json!(min_date.unwrap_or_default())).await?;
        context.set_pin_value("to_date", json!(max_date.unwrap_or_default())).await?;

        Ok(())
    }
}
