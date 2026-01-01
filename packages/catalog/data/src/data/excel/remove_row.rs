use crate::data::path::FlowPath;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json};
use std::io::Cursor;
use umya_spreadsheet::{self};

/// Remove one or more rows from an XLSX worksheet (object-store aware).
/// - Works entirely in-memory using `FlowPath` bytes (no local filesystem I/O).
/// - If the file/sheet doesn't exist yet, a new workbook/sheet is created and the op is a no-op.
/// - `row` is 1-based.
#[crate::register_node]
#[derive(Default)]
pub struct RemoveRowNode {}

impl RemoveRowNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for RemoveRowNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "excel_remove_row",
            "Excel Remove Row",
            "Delete one or more rows from an XLSX sheet",
            "Data/Excel",
        );
        node.add_icon("/flow/icons/file-spreadsheet.svg");

        node.add_input_pin("exec_in", "In", "Trigger", VariableType::Execution);

        node.add_input_pin("file", "File", "Target XLSX file", VariableType::Struct)
            .set_schema::<FlowPath>();
        node.add_input_pin("sheet", "Sheet", "Worksheet name", VariableType::String)
            .set_default_value(Some(json!("Sheet1")));
        node.add_input_pin("row", "Row", "Start row (1-based)", VariableType::String)
            .set_default_value(Some(json!("1")));
        node.add_input_pin(
            "count",
            "Count",
            "How many rows to remove",
            VariableType::String,
        )
        .set_default_value(Some(json!("1")));

        node.add_output_pin("exec_out", "Out", "Trigger", VariableType::Execution);
        node.add_output_pin(
            "file_out",
            "File",
            "Updated XLSX path",
            VariableType::Struct,
        )
        .set_schema::<FlowPath>();
        node.add_output_pin("ok", "OK", "Operation success", VariableType::Boolean);

        node
    }

    async fn run(&self, ctx: &mut ExecutionContext) -> flow_like_types::Result<()> {
        ctx.deactivate_exec_pin("exec_out").await?;

        let file: FlowPath = ctx.evaluate_pin("file").await?;
        let sheet: String = ctx.evaluate_pin("sheet").await?;
        let row_in: String = ctx.evaluate_pin("row").await?;
        let count_in: String = ctx.evaluate_pin("count").await?;

        let row: u32 = parse_row_1_based(&row_in)?;
        let count: u32 = count_in
            .trim()
            .parse()
            .map_err(|e| flow_like_types::anyhow!("Invalid 'count' value '{}': {}", count_in, e))?;
        if count == 0 {
            return Err(flow_like_types::anyhow!("'count' must be >= 1"));
        }

        let mut book = match file.get(ctx, false).await {
            Ok(bytes) if !bytes.is_empty() => {
                match umya_spreadsheet::reader::xlsx::read_reader(Cursor::new(bytes), true) {
                    Ok(b) => b,
                    Err(e) => {
                        return Err(flow_like_types::anyhow!(
                            "Failed to read workbook bytes: {}",
                            e
                        ));
                    }
                }
            }
            _ => umya_spreadsheet::new_file(),
        };

        if book.get_sheet_by_name(&sheet).is_none() {
            let _ = book.new_sheet(&sheet);
        }
        let ws = book.get_sheet_by_name_mut(&sheet).ok_or_else(|| {
            flow_like_types::anyhow!("Failed to access or create sheet: {}", sheet)
        })?;

        ws.remove_row(&row, &count);

        let mut out: Vec<u8> = Vec::new();
        if let Err(e) = umya_spreadsheet::writer::xlsx::write_writer(&book, &mut out) {
            return Err(flow_like_types::anyhow!(
                "Failed to serialize workbook: {}",
                e
            ));
        }

        if let Err(e) = file.put(ctx, out, false).await {
            return Err(flow_like_types::anyhow!(
                "Failed to store updated workbook: {}",
                e
            ));
        }

        ctx.set_pin_value("file_out", json!(file)).await?;
        ctx.set_pin_value("ok", json!(true)).await?;
        ctx.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}

fn parse_row_1_based(s: &str) -> flow_like_types::Result<u32> {
    let trimmed = s.trim();
    let n: u32 = trimmed
        .parse()
        .map_err(|e| flow_like_types::anyhow!("Invalid row '{}': {}", s, e))?;
    if n == 0 {
        return Err(flow_like_types::anyhow!("Row must be 1-based (>=1): {}", s));
    }
    Ok(n)
}
