use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json, Value};
use super::chart_data_utils::{extract_from_csv_table, parse_csv_text, parse_column_ref};

/// Converts CSV data or CSVTable (from DataFusion) to Nivo Heatmap chart format.
///
/// **Output Format:** `[{ id: "Row1", data: [{ x: "Col1", y: 10 }, { x: "Col2", y: 20 }] }, ...]`
///
/// **Documentation:** https://nivo.rocks/heatmap/
///
/// **Accepts:**
/// - Raw CSV text with headers
/// - CSVTable struct from DataFusion SQL queries
#[crate::register_node]
#[derive(Default)]
pub struct CsvToHeatmapData;

impl CsvToHeatmapData {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for CsvToHeatmapData {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_csv_to_heatmap_data",
            "CSV to Heatmap Data",
            "Converts CSV or DataFusion CSVTable to Nivo Heatmap format. Docs: https://nivo.rocks/heatmap/",
            "A2UI/Elements/Charts/Heatmap",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin(
            "csv",
            "CSV",
            "CSV matrix with headers. First column = row IDs, other columns = cell values",
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
            "row_column",
            "Row Column",
            "Column name or 0-based index for row identifiers (default: 0)",
            VariableType::String,
        )
        .set_default_value(Some(json!("0")));

        node.add_input_pin(
            "value_columns",
            "Value Columns",
            "Comma-separated column names/indices for cell values. Empty = all except row column",
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

        node.add_output_pin("data", "Data", "Heatmap data array", VariableType::Generic);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let row_col: String = context.evaluate_pin("row_column").await?;
        let value_cols: String = context.evaluate_pin("value_columns").await?;
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

        let row_idx = parse_column_ref(&row_col, &headers);

        let col_indices: Vec<usize> = if value_cols.is_empty() {
            (0..headers.len()).filter(|&i| i != row_idx).collect()
        } else {
            value_cols.split(',').map(|s| parse_column_ref(s.trim(), &headers)).filter(|&i| i != row_idx && i < headers.len()).collect()
        };

        let data: Vec<Value> = rows.iter().map(|row| {
            let row_id = row.get(row_idx).cloned().unwrap_or_default();

            let cells: Vec<Value> = col_indices.iter().map(|&col_idx| {
                let col_name = headers.get(col_idx).cloned().unwrap_or_default();
                let val: f64 = row.get(col_idx).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                json!({ "x": col_name, "y": val })
            }).collect();

            json!({ "id": row_id, "data": cells })
        }).collect();

        context.set_pin_value("data", json!(data)).await?;

        Ok(())
    }
}
