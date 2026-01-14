use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json, json::Map, Value};

/// Parses CSV text and outputs table-compatible columns and data.
///
/// Automatically extracts headers from the first row and converts
/// remaining rows into data objects keyed by header names.
#[crate::register_node]
#[derive(Default)]
pub struct CsvToTable;

impl CsvToTable {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for CsvToTable {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_csv_to_table",
            "CSV to Table",
            "Parses CSV text and outputs table columns and data rows",
            "A2UI/Elements/Table",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin(
            "csv",
            "CSV",
            "CSV text content to parse",
            VariableType::String,
        );

        node.add_input_pin(
            "delimiter",
            "Delimiter",
            "Column delimiter (default: comma)",
            VariableType::String,
        )
        .set_default_value(Some(json!(",")));

        node.add_input_pin(
            "has_header",
            "Has Header",
            "Whether the first row contains column headers",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_output_pin(
            "columns",
            "Columns",
            "Table column definitions array",
            VariableType::Generic,
        );

        node.add_output_pin(
            "data",
            "Data",
            "Table data rows array",
            VariableType::Generic,
        );

        node.add_output_pin(
            "row_count",
            "Row Count",
            "Number of data rows parsed",
            VariableType::Integer,
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let csv_text: String = context.evaluate_pin("csv").await?;
        let delimiter: String = context.evaluate_pin("delimiter").await?;
        let has_header: bool = context.evaluate_pin("has_header").await?;

        let delimiter_char = delimiter.chars().next().unwrap_or(',');

        let lines: Vec<&str> = csv_text
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();

        if lines.is_empty() {
            context.set_pin_value("columns", json!([])).await?;
            context.set_pin_value("data", json!([])).await?;
            context.set_pin_value("row_count", json!(0)).await?;
            return Ok(());
        }

        let (headers, data_start) = if has_header {
            let header_row = parse_csv_row(lines[0], delimiter_char);
            (header_row, 1)
        } else {
            let first_row = parse_csv_row(lines[0], delimiter_char);
            let headers: Vec<String> = (0..first_row.len())
                .map(|i| format!("Column {}", i + 1))
                .collect();
            (headers, 0)
        };

        let columns: Vec<Value> = headers
            .iter()
            .map(|h| {
                json!({
                    "id": h.to_lowercase().replace(' ', "_"),
                    "header": { "literalString": h },
                    "accessor": { "literalString": h.to_lowercase().replace(' ', "_") }
                })
            })
            .collect();

        let data: Vec<Value> = lines[data_start..]
            .iter()
            .map(|line| {
                let cells = parse_csv_row(line, delimiter_char);
                let mut row = Map::new();
                for (i, header) in headers.iter().enumerate() {
                    let key = header.to_lowercase().replace(' ', "_");
                    let value = cells.get(i).cloned().unwrap_or_default();
                    row.insert(key, json!(value));
                }
                Value::Object(row)
            })
            .collect();

        let row_count = data.len() as i64;

        context.set_pin_value("columns", json!(columns)).await?;
        context.set_pin_value("data", json!(data)).await?;
        context.set_pin_value("row_count", json!(row_count)).await?;

        Ok(())
    }
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
