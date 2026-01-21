use super::chart_data_utils::{
    clean_field_name, extract_from_csv_table, parse_column_ref, parse_csv_text,
};
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::Map, json::json};

/// Converts CSV data or CSVTable (from DataFusion) to Nivo Radar chart format.
///
/// **Output Format:** `[{ category: "Speed", series1: 70, series2: 50 }, ...]`
///
/// **Documentation:** https://nivo.rocks/radar/
///
/// **Accepts:**
/// - Raw CSV text with headers
/// - CSVTable struct from DataFusion SQL queries
#[crate::register_node]
#[derive(Default)]
pub struct CsvToRadarData;

impl CsvToRadarData {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for CsvToRadarData {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_csv_to_radar_data",
            "CSV to Radar Data",
            "Converts CSV or DataFusion CSVTable to Nivo Radar/Spider chart format. Docs: https://nivo.rocks/radar/",
            "A2UI/Elements/Charts/Radar",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin(
            "csv",
            "CSV",
            "CSV with headers. First column = dimensions, other columns = series values",
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
            "dimension_column",
            "Dimension Column",
            "Column name or 0-based index for radar dimensions (default: 0)",
            VariableType::String,
        )
        .set_default_value(Some(json!("0")));

        node.add_input_pin(
            "series_columns",
            "Series Columns",
            "Comma-separated column names/indices for series. Empty = all except dimension",
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

        node.add_output_pin(
            "data",
            "Data",
            "Radar chart data array",
            VariableType::Generic,
        );
        node.add_output_pin(
            "keys",
            "Keys",
            "Series keys for the chart",
            VariableType::Generic,
        );
        node.add_output_pin(
            "index_by",
            "Index By",
            "Dimension field name",
            VariableType::String,
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let dim_col: String = context.evaluate_pin("dimension_column").await?;
        let series_cols: String = context.evaluate_pin("series_columns").await?;
        let delimiter: String = context.evaluate_pin("delimiter").await?;

        let (headers, rows) = if let Ok(table_value) = context.evaluate_pin::<Value>("table").await
        {
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
            context
                .set_pin_value("index_by", json!("dimension"))
                .await?;
            return Ok(());
        }

        let dim_idx = parse_column_ref(&dim_col, &headers);
        let index_field = clean_field_name(
            headers
                .get(dim_idx)
                .map(|s| s.as_str())
                .unwrap_or("dimension"),
        );

        let series_indices: Vec<usize> = if series_cols.is_empty() {
            (0..headers.len()).filter(|&i| i != dim_idx).collect()
        } else {
            series_cols
                .split(',')
                .map(|s| parse_column_ref(s.trim(), &headers))
                .filter(|&i| i != dim_idx && i < headers.len())
                .collect()
        };

        let keys: Vec<String> = series_indices
            .iter()
            .map(|&i| {
                clean_field_name(
                    headers
                        .get(i)
                        .map(|s| s.as_str())
                        .unwrap_or(&format!("series{}", i)),
                )
            })
            .collect();

        let data: Vec<Value> = rows
            .iter()
            .map(|row| {
                let mut obj = Map::new();
                obj.insert(
                    index_field.clone(),
                    json!(row.get(dim_idx).cloned().unwrap_or_default()),
                );
                for (i, &col_idx) in series_indices.iter().enumerate() {
                    let val: f64 = row.get(col_idx).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    obj.insert(keys[i].clone(), json!(val));
                }
                Value::Object(obj)
            })
            .collect();

        context.set_pin_value("data", json!(data)).await?;
        context.set_pin_value("keys", json!(keys)).await?;
        context
            .set_pin_value("index_by", json!(index_field))
            .await?;

        Ok(())
    }
}
