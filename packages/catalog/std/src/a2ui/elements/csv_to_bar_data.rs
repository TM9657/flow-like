use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json, json::Map, Value};
use super::chart_data_utils::{extract_from_csv_table, parse_csv_text, parse_column_ref, clean_field_name};

/// Converts CSV data or CSVTable (from DataFusion) to Nivo Bar chart format.
///
/// **Output Format:** `[{ category: "A", value1: 10, value2: 20 }, ...]`
///
/// **Documentation:** https://nivo.rocks/bar/
///
/// The Bar chart expects an array of objects where each object represents a group/category
/// with one or more numeric values for different series.
///
/// **Accepts:**
/// - Raw CSV text with headers
/// - CSVTable struct from DataFusion SQL queries
#[crate::register_node]
#[derive(Default)]
pub struct CsvToBarData;

impl CsvToBarData {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for CsvToBarData {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_csv_to_bar_data",
            "CSV to Bar Data",
            "Converts CSV or DataFusion CSVTable to Nivo Bar chart format. Docs: https://nivo.rocks/bar/",
            "A2UI/Elements/Charts/Bar",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin(
            "csv",
            "CSV",
            "CSV text with headers. First column = categories, other columns = values",
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
            "category_column",
            "Category Column",
            "Column name or 0-based index for categories (default: 0)",
            VariableType::String,
        )
        .set_default_value(Some(json!("0")));

        node.add_input_pin(
            "value_columns",
            "Value Columns",
            "Comma-separated column names/indices for values. Empty = all except category",
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

        node.add_output_pin("data", "Data", "Bar chart data array", VariableType::Generic);
        node.add_output_pin("keys", "Keys", "Value keys for the chart (series names)", VariableType::Generic);
        node.add_output_pin("index_by", "Index By", "Category field name", VariableType::String);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let category_col: String = context.evaluate_pin("category_column").await?;
        let value_columns: String = context.evaluate_pin("value_columns").await?;
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
            context.set_pin_value("keys", json!([])).await?;
            context.set_pin_value("index_by", json!("category")).await?;
            return Ok(());
        }

        let cat_idx = parse_column_ref(&category_col, &headers);
        let index_field = clean_field_name(headers.get(cat_idx).map(|s| s.as_str()).unwrap_or("category"));

        let value_indices: Vec<usize> = if value_columns.is_empty() {
            (0..headers.len()).filter(|&i| i != cat_idx).collect()
        } else {
            value_columns.split(',').map(|s| parse_column_ref(s.trim(), &headers)).filter(|&i| i != cat_idx && i < headers.len()).collect()
        };

        let keys: Vec<String> = value_indices.iter().map(|&i| clean_field_name(headers.get(i).map(|s| s.as_str()).unwrap_or(&format!("value{}", i)))).collect();

        let data: Vec<Value> = rows.iter().map(|row| {
            let mut obj = Map::new();
            obj.insert(index_field.clone(), json!(row.get(cat_idx).cloned().unwrap_or_default()));
            for (i, &col_idx) in value_indices.iter().enumerate() {
                let val: f64 = row.get(col_idx).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                obj.insert(keys[i].clone(), json!(val));
            }
            Value::Object(obj)
        }).collect();

        context.set_pin_value("data", json!(data)).await?;
        context.set_pin_value("keys", json!(keys)).await?;
        context.set_pin_value("index_by", json!(index_field)).await?;

        Ok(())
    }
}
