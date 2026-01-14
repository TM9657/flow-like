use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json, json::Map, Value};

/// Parses CSV text and outputs Nivo-compatible chart data.
///
/// Supports multiple output formats for different chart types:
/// - Bar/Line: array of objects with keys from headers
/// - Pie: array of {id, label, value} objects
/// - Radar: array with index and multiple value keys
/// - Heatmap: hierarchical data with rows and cells
#[crate::register_node]
#[derive(Default)]
pub struct CsvToNivoData;

impl CsvToNivoData {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for CsvToNivoData {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_csv_to_nivo_data",
            "CSV to Nivo Data",
            "Converts CSV to Nivo chart data format",
            "A2UI/Elements/Charts",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin(
            "csv",
            "CSV",
            "CSV text content to parse",
            VariableType::String,
        );

        node.add_input_pin(
            "format",
            "Format",
            "Output format: 'bar', 'line', 'pie', 'radar', 'heatmap', 'generic'",
            VariableType::String,
        )
        .set_default_value(Some(json!("generic")));

        node.add_input_pin(
            "index_column",
            "Index Column",
            "Column name or index to use as category/index (0-based)",
            VariableType::String,
        )
        .set_default_value(Some(json!("0")));

        node.add_input_pin(
            "value_columns",
            "Value Columns",
            "Comma-separated column names or indices for values (empty = all non-index)",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "delimiter",
            "Delimiter",
            "Column delimiter (default: comma)",
            VariableType::String,
        )
        .set_default_value(Some(json!(",")));

        node.add_output_pin(
            "data",
            "Data",
            "Nivo-compatible chart data",
            VariableType::Generic,
        );

        node.add_output_pin(
            "keys",
            "Keys",
            "Data keys (for bar/radar charts)",
            VariableType::Generic,
        );

        node.add_output_pin(
            "index_by",
            "Index By",
            "Index field name (for bar/radar charts)",
            VariableType::String,
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let csv_text: String = context.evaluate_pin("csv").await?;
        let format: String = context.evaluate_pin("format").await?;
        let index_column: String = context.evaluate_pin("index_column").await?;
        let value_columns: String = context.evaluate_pin("value_columns").await?;
        let delimiter: String = context.evaluate_pin("delimiter").await?;

        let delimiter_char = delimiter.chars().next().unwrap_or(',');

        let lines: Vec<&str> = csv_text
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();

        if lines.is_empty() {
            context.set_pin_value("data", json!([])).await?;
            context.set_pin_value("keys", json!([])).await?;
            context.set_pin_value("index_by", json!("")).await?;
            return Ok(());
        }

        let headers = parse_csv_row(lines[0], delimiter_char);

        // Determine index column
        let index_col_idx = parse_column_ref(&index_column, &headers);
        let index_field = headers.get(index_col_idx)
            .cloned()
            .unwrap_or_else(|| "category".to_string())
            .to_lowercase()
            .replace(' ', "_");

        // Determine value columns
        let value_col_indices: Vec<usize> = if value_columns.is_empty() {
            (0..headers.len()).filter(|&i| i != index_col_idx).collect()
        } else {
            value_columns
                .split(',')
                .map(|s| parse_column_ref(s.trim(), &headers))
                .filter(|&i| i != index_col_idx && i < headers.len())
                .collect()
        };

        let value_keys: Vec<String> = value_col_indices
            .iter()
            .map(|&i| headers.get(i).cloned().unwrap_or_else(|| format!("value{}", i)))
            .map(|s| s.to_lowercase().replace(' ', "_"))
            .collect();

        // Parse data rows
        let data_rows: Vec<Vec<String>> = lines[1..]
            .iter()
            .map(|line| parse_csv_row(line, delimiter_char))
            .collect();

        let output = match format.as_str() {
            "bar" | "radar" | "generic" => {
                // Array of objects: [{category: "A", value1: 10, value2: 20}, ...]
                let data: Vec<Value> = data_rows
                    .iter()
                    .map(|row| {
                        let mut obj = Map::new();
                        obj.insert(index_field.clone(), json!(row.get(index_col_idx).cloned().unwrap_or_default()));
                        for (i, &col_idx) in value_col_indices.iter().enumerate() {
                            let val = row.get(col_idx).cloned().unwrap_or_default();
                            let num_val: f64 = val.parse().unwrap_or(0.0);
                            obj.insert(value_keys[i].clone(), json!(num_val));
                        }
                        Value::Object(obj)
                    })
                    .collect();
                json!(data)
            }
            "line" => {
                // Array of series: [{id: "Series", data: [{x: "A", y: 10}, ...]}, ...]
                let series: Vec<Value> = value_keys
                    .iter()
                    .enumerate()
                    .map(|(i, key)| {
                        let points: Vec<Value> = data_rows
                            .iter()
                            .map(|row| {
                                let x = row.get(index_col_idx).cloned().unwrap_or_default();
                                let y_str = row.get(value_col_indices[i]).cloned().unwrap_or_default();
                                let y: f64 = y_str.parse().unwrap_or(0.0);
                                json!({"x": x, "y": y})
                            })
                            .collect();
                        json!({
                            "id": key,
                            "data": points
                        })
                    })
                    .collect();
                json!(series)
            }
            "pie" => {
                // Array: [{id: "A", label: "A", value: 10}, ...]
                let data: Vec<Value> = data_rows
                    .iter()
                    .map(|row| {
                        let id = row.get(index_col_idx).cloned().unwrap_or_default();
                        let val_str = row.get(*value_col_indices.first().unwrap_or(&1)).cloned().unwrap_or_default();
                        let val: f64 = val_str.parse().unwrap_or(0.0);
                        json!({
                            "id": id,
                            "label": id,
                            "value": val
                        })
                    })
                    .collect();
                json!(data)
            }
            "heatmap" => {
                // Rows with nested data: [{id: "Row1", data: [{x: "Col1", y: 10}, ...]}, ...]
                let data: Vec<Value> = data_rows
                    .iter()
                    .map(|row| {
                        let row_id = row.get(index_col_idx).cloned().unwrap_or_default();
                        let cells: Vec<Value> = value_col_indices
                            .iter()
                            .enumerate()
                            .map(|(i, &col_idx)| {
                                let val_str = row.get(col_idx).cloned().unwrap_or_default();
                                let val: f64 = val_str.parse().unwrap_or(0.0);
                                json!({
                                    "x": value_keys[i],
                                    "y": val
                                })
                            })
                            .collect();
                        json!({
                            "id": row_id,
                            "data": cells
                        })
                    })
                    .collect();
                json!(data)
            }
            _ => {
                // Generic: same as bar
                let data: Vec<Value> = data_rows
                    .iter()
                    .map(|row| {
                        let mut obj = Map::new();
                        obj.insert(index_field.clone(), json!(row.get(index_col_idx).cloned().unwrap_or_default()));
                        for (i, &col_idx) in value_col_indices.iter().enumerate() {
                            let val = row.get(col_idx).cloned().unwrap_or_default();
                            let num_val: f64 = val.parse().unwrap_or(0.0);
                            obj.insert(value_keys[i].clone(), json!(num_val));
                        }
                        Value::Object(obj)
                    })
                    .collect();
                json!(data)
            }
        };

        context.set_pin_value("data", output).await?;
        context.set_pin_value("keys", json!(value_keys)).await?;
        context.set_pin_value("index_by", json!(index_field)).await?;

        Ok(())
    }
}

fn parse_column_ref(col_ref: &str, headers: &[String]) -> usize {
    // Try as number first
    if let Ok(idx) = col_ref.parse::<usize>() {
        return idx;
    }
    // Try as header name
    let col_ref_lower = col_ref.to_lowercase();
    headers
        .iter()
        .position(|h| h.to_lowercase() == col_ref_lower)
        .unwrap_or(0)
}

fn parse_csv_row(line: &str, delimiter: char) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '"' {
            if in_quotes {
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                in_quotes = true;
            }
        } else if c == delimiter && !in_quotes {
            result.push(current.trim().to_string());
            current = String::new();
        } else {
            current.push(c);
        }
    }
    result.push(current.trim().to_string());
    result
}
