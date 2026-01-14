use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json, Value};
use super::chart_data_utils::{extract_from_csv_table, parse_csv_text, parse_column_ref, clean_field_name};

/// Converts CSV data or CSVTable (from DataFusion) to Nivo Line chart format.
///
/// **Output Format:** `[{ id: "Series1", data: [{ x: "Jan", y: 10 }, ...] }, ...]`
///
/// **Documentation:** https://nivo.rocks/line/
///
/// **Accepts:**
/// - Raw CSV text with headers
/// - CSVTable struct from DataFusion SQL queries
#[crate::register_node]
#[derive(Default)]
pub struct CsvToLineData;

impl CsvToLineData {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for CsvToLineData {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_csv_to_line_data",
            "CSV to Line Data",
            "Converts CSV or DataFusion CSVTable to Nivo Line chart format. Docs: https://nivo.rocks/line/",
            "A2UI/Elements/Charts/Line",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin(
            "csv",
            "CSV",
            "CSV with headers. First column = X values, other columns = Y series",
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
            "x_column",
            "X Column",
            "Column name or 0-based index for X axis values (default: 0)",
            VariableType::String,
        )
        .set_default_value(Some(json!("0")));

        node.add_input_pin(
            "y_columns",
            "Y Columns",
            "Comma-separated column names/indices for Y series. Empty = all except X",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "delimiter",
            "Delimiter",
            "Column delimiter for CSV text (default: comma)",
            VariableType::String,
        )
        .set_default_value(Some(json!(",")));

        node.add_output_pin("data", "Data", "Line chart data array with series", VariableType::Generic);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let x_col: String = context.evaluate_pin("x_column").await?;
        let y_columns: String = context.evaluate_pin("y_columns").await?;
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
            return Ok(());
        }

        let x_idx = parse_column_ref(&x_col, &headers);

        let y_indices: Vec<usize> = if y_columns.is_empty() {
            (0..headers.len()).filter(|&i| i != x_idx).collect()
        } else {
            y_columns.split(',').map(|s| parse_column_ref(s.trim(), &headers)).filter(|&i| i != x_idx && i < headers.len()).collect()
        };

        let series: Vec<Value> = y_indices.iter().map(|&y_idx| {
            let series_name = clean_field_name(headers.get(y_idx).map(|s| s.as_str()).unwrap_or(&format!("Series{}", y_idx)));
            let points: Vec<Value> = rows.iter().map(|row| {
                let x = row.get(x_idx).cloned().unwrap_or_default();
                let y: f64 = row.get(y_idx).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                json!({ "x": x, "y": y })
            }).collect();
            json!({ "id": series_name, "data": points })
        }).collect();

        context.set_pin_value("data", json!(series)).await?;

        Ok(())
    }
}
