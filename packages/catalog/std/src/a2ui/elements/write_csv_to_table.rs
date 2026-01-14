use super::element_utils::extract_element_id;
use flow_like::a2ui::components::TableProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json, json::Map, Value};

/// Writes CSV data directly to a table element.
///
/// Parses CSV text, automatically creates column definitions from headers,
/// and updates both columns and data on the table in a single operation.
#[crate::register_node]
#[derive(Default)]
pub struct WriteCsvToTable;

impl WriteCsvToTable {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for WriteCsvToTable {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_write_csv_to_table",
            "Write CSV to Table",
            "Parses CSV text and writes columns and data directly to a table element",
            "A2UI/Elements/Table",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin("exec_in", "▶", "Execution input", VariableType::Execution);

        node.add_input_pin(
            "element_ref",
            "Table",
            "Reference to the table element (ID or element object)",
            VariableType::Struct,
        )
        .set_schema::<TableProps>()
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_input_pin(
            "csv",
            "CSV",
            "CSV text content to parse and write to the table",
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

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);

        node.add_output_pin(
            "row_count",
            "Row Count",
            "Number of data rows written",
            VariableType::Integer,
        );

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;

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
            let update_value = json!({
                "type": "setProps",
                "props": {
                    "columns": { "literalOptions": [] },
                    "data": { "literalOptions": [] }
                }
            });
            context.upsert_element(&element_id, update_value).await?;
            context.set_pin_value("row_count", json!(0)).await?;
            context.activate_exec_pin("exec_out").await?;
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
                let id = h.to_lowercase().replace(' ', "_");
                json!({
                    "id": id,
                    "header": { "literalString": h },
                    "accessor": { "literalString": id }
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

        let update_value = json!({
            "type": "setProps",
            "props": {
                "columns": { "literalOptions": columns },
                "data": { "literalOptions": data }
            }
        });

        context.upsert_element(&element_id, update_value).await?;
        context.set_pin_value("row_count", json!(row_count)).await?;
        context.activate_exec_pin("exec_out").await?;

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
