use crate::data::{excel::{parse_col_1_based, parse_row_1_based}, path::FlowPath};
use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, json::json};
use std::io::Cursor;
use umya_spreadsheet::{self};

/// Read a single cell from an Excel workbook (XLSX) located in object storage via `FlowPath`.
/// - Does **not** touch the local filesystem; reads from bytes and returns the raw string value.
/// - If the file, sheet or cell is missing, `value` is set to "" and `found` is false.
/// - Row/Col are 1-based; column can be letters (A, AA, ...) or a 1-based number.
#[derive(Default)]
pub struct ReadCellNode {}

impl ReadCellNode {
    pub fn new() -> Self { Self {} }
}

#[async_trait]
impl NodeLogic for ReadCellNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "excel_read_cell",
            "Excel Read Cell",
            "Read a single cell value from an XLSX sheet",
            "Data/Excel",
        );
        node.add_icon("/flow/icons/file-spreadsheet.svg");

        node.add_input_pin("exec_in", "In", "Trigger", VariableType::Execution);

        node.add_input_pin("file", "File", "Source XLSX file", VariableType::Struct)
            .set_schema::<FlowPath>();
        node.add_input_pin("sheet", "Sheet", "Worksheet name", VariableType::String)
            .set_default_value(Some(json!("Sheet1")));
        node.add_input_pin("row", "Row", "Row number (1-based)", VariableType::String)
            .set_default_value(Some(json!("1")));
        node.add_input_pin("col", "Column", "Column letters or number (1-based)", VariableType::String)
            .set_default_value(Some(json!("A")));

        node.add_output_pin("exec_out", "Out", "Trigger", VariableType::Execution);
        node.add_output_pin("file", "File", "Pass-through XLSX path", VariableType::Struct)
            .set_schema::<FlowPath>();
        node.add_output_pin("value", "Value", "Cell value (raw string)", VariableType::String);
        node.add_output_pin("found", "Found", "Cell exists and has a value", VariableType::Boolean);

        node
    }

    async fn run(&self, ctx: &mut ExecutionContext) -> flow_like_types::Result<()> {
        ctx.deactivate_exec_pin("exec_out").await?;

        let file: FlowPath = ctx.evaluate_pin("file").await?;
        let sheet: String = ctx.evaluate_pin("sheet").await?;
        let row_str: String = ctx.evaluate_pin("row").await?;
        let col_str: String = ctx.evaluate_pin("col").await?;

        let mut out_value = String::new();
        let mut found = false;

        match file.get(ctx, false).await {
            Ok(bytes) if !bytes.is_empty() => {
                let book = match umya_spreadsheet::reader::xlsx::read_reader(Cursor::new(bytes), true) {
                    Ok(b) => b,
                    Err(e) => return Err(flow_like_types::anyhow!("Failed to read workbook bytes: {}", e)),
                };

                if let Some(ws) = book.get_sheet_by_name(&sheet) {
                    let row = parse_row_1_based(&row_str)?;
                    let col = parse_col_1_based(&col_str)?;

                    found = ws.get_cell((col, row)).is_some();
                    if found {
                        out_value = ws.get_value((col, row)); // raw string
                    } else {
                        out_value.clear();
                    }
                } else {}
            }
            _ => {}
        }

        ctx.set_pin_value("file", json!(file)).await?;
        ctx.set_pin_value("value", json!(out_value)).await?;
        ctx.set_pin_value("found", json!(found)).await?;

        ctx.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}