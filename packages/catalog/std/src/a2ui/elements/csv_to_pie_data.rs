use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json, Value};
use super::chart_data_utils::{extract_from_csv_table, parse_csv_text, parse_column_ref};

/// Converts CSV data or CSVTable (from DataFusion) to Nivo Pie chart format.
///
/// **Output Format:** `[{ id: "A", label: "Category A", value: 35 }, ...]`
///
/// **Documentation:** https://nivo.rocks/pie/
///
/// **Accepts:**
/// - Raw CSV text with headers
/// - CSVTable struct from DataFusion SQL queries
#[crate::register_node]
#[derive(Default)]
pub struct CsvToPieData;

impl CsvToPieData {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for CsvToPieData {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_csv_to_pie_data",
            "CSV to Pie Data",
            "Converts CSV or DataFusion CSVTable to Nivo Pie/Donut chart format. Docs: https://nivo.rocks/pie/",
            "A2UI/Elements/Charts/Pie",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin(
            "csv",
            "CSV",
            "CSV with headers. Expected: label column and value column",
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
            "label_column",
            "Label Column",
            "Column name or 0-based index for slice labels (default: 0)",
            VariableType::String,
        )
        .set_default_value(Some(json!("0")));

        node.add_input_pin(
            "value_column",
            "Value Column",
            "Column name or 0-based index for slice values (default: 1)",
            VariableType::String,
        )
        .set_default_value(Some(json!("1")));

        node.add_input_pin(
            "color_column",
            "Color Column",
            "Optional: Column with hex colors for each slice",
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

        node.add_output_pin("data", "Data", "Pie chart data array", VariableType::Generic);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let label_col: String = context.evaluate_pin("label_column").await?;
        let value_col: String = context.evaluate_pin("value_column").await?;
        let color_col: String = context.evaluate_pin("color_column").await?;
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

        let label_idx = parse_column_ref(&label_col, &headers);
        let value_idx = parse_column_ref(&value_col, &headers);
        let color_idx = if color_col.is_empty() { None } else { Some(parse_column_ref(&color_col, &headers)) };

        let data: Vec<Value> = rows.iter().map(|row| {
            let label = row.get(label_idx).cloned().unwrap_or_default();
            let id = label.to_lowercase().replace(' ', "_");
            let value: f64 = row.get(value_idx).and_then(|s| s.parse().ok()).unwrap_or(0.0);

            let mut slice = json!({
                "id": id,
                "label": label,
                "value": value
            });

            if let Some(c_idx) = color_idx {
                if let Some(color) = row.get(c_idx) {
                    if !color.is_empty() {
                        slice["color"] = json!(color);
                    }
                }
            }

            slice
        }).collect();

        context.set_pin_value("data", json!(data)).await?;

        Ok(())
    }
}
