use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json, Value};
use super::chart_data_utils::{extract_from_csv_table, parse_csv_text, parse_column_ref};

/// Converts CSV data or CSVTable (from DataFusion) to Nivo Sankey diagram format.
///
/// **Output Format:** `{ nodes: [{ id: "A" }, ...], links: [{ source: "A", target: "B", value: 10 }, ...] }`
///
/// **Documentation:** https://nivo.rocks/sankey/
///
/// **Accepts:**
/// - Raw CSV text with headers
/// - CSVTable struct from DataFusion SQL queries
#[crate::register_node]
#[derive(Default)]
pub struct CsvToSankeyData;

impl CsvToSankeyData {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for CsvToSankeyData {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_csv_to_sankey_data",
            "CSV to Sankey Data",
            "Converts CSV or DataFusion CSVTable to Nivo Sankey diagram format. Docs: https://nivo.rocks/sankey/",
            "A2UI/Elements/Charts/Sankey",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin(
            "csv",
            "CSV",
            "CSV with source, target, and value columns for flow data",
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
            "source_column",
            "Source Column",
            "Column name or 0-based index for source nodes (default: 0)",
            VariableType::String,
        )
        .set_default_value(Some(json!("0")));

        node.add_input_pin(
            "target_column",
            "Target Column",
            "Column name or 0-based index for target nodes (default: 1)",
            VariableType::String,
        )
        .set_default_value(Some(json!("1")));

        node.add_input_pin(
            "value_column",
            "Value Column",
            "Column name or 0-based index for flow values (default: 2)",
            VariableType::String,
        )
        .set_default_value(Some(json!("2")));

        node.add_input_pin(
            "delimiter",
            "Delimiter",
            "Column delimiter for CSV text (default: comma)",
            VariableType::String,
        )
        .set_default_value(Some(json!(",")));

        node.add_output_pin("data", "Data", "Sankey data object with nodes and links", VariableType::Generic);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let source_col: String = context.evaluate_pin("source_column").await?;
        let target_col: String = context.evaluate_pin("target_column").await?;
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
            context.set_pin_value("data", json!({ "nodes": [], "links": [] })).await?;
            return Ok(());
        }

        let source_idx = parse_column_ref(&source_col, &headers);
        let target_idx = parse_column_ref(&target_col, &headers);
        let value_idx = parse_column_ref(&value_col, &headers);

        let mut node_set = std::collections::HashSet::new();
        let mut links: Vec<Value> = Vec::new();

        for row in &rows {
            let source = row.get(source_idx).cloned().unwrap_or_default();
            let target = row.get(target_idx).cloned().unwrap_or_default();
            let value: f64 = row.get(value_idx).and_then(|s| s.parse().ok()).unwrap_or(0.0);

            if !source.is_empty() && !target.is_empty() {
                node_set.insert(source.clone());
                node_set.insert(target.clone());
                links.push(json!({ "source": source, "target": target, "value": value }));
            }
        }

        let nodes: Vec<Value> = node_set.into_iter().map(|id| json!({ "id": id })).collect();

        context.set_pin_value("data", json!({ "nodes": nodes, "links": links })).await?;

        Ok(())
    }
}
