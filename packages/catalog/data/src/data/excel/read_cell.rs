use crate::data::{
    excel::{parse_col_1_based, parse_row_1_based},
    path::FlowPath,
};
use calamine::{Data, DataType, Reader, open_workbook_auto_from_rs};
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json};
use std::io::Cursor;

/// Read a single cell from an Excel workbook (XLSX) located in object storage via `FlowPath`.
/// - Does **not** touch the local filesystem; reads from bytes and returns the raw string value.
/// - If the file, sheet or cell is missing, `value` is set to "" and `found` is false.
/// - Row/Col are 1-based; column can be letters (A, AA, ...) or a 1-based number.
#[crate::register_node]
#[derive(Default)]
pub struct ReadCellNode {}

impl ReadCellNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for ReadCellNode {
    fn get_node(&self) -> Node {
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
        node.add_input_pin(
            "col",
            "Column",
            "Column letters or number (1-based)",
            VariableType::String,
        )
        .set_default_value(Some(json!("A")));

        node.add_output_pin("exec_out", "Out", "Trigger", VariableType::Execution);
        node.add_output_pin(
            "file_out",
            "File",
            "Pass-through XLSX path",
            VariableType::Struct,
        )
        .set_schema::<FlowPath>();
        node.add_output_pin(
            "value",
            "Value",
            "Cell value (raw string)",
            VariableType::String,
        );
        node.add_output_pin(
            "found",
            "Found",
            "Cell exists and has a value",
            VariableType::Boolean,
        );

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

        let bytes = file.get(ctx, false).await?;

        if !bytes.is_empty() {
            let mut wb = open_workbook_auto_from_rs(Cursor::new(bytes))
                .map_err(|e| flow_like_types::anyhow!("Calamine open failed: {}", e))?;

            if let Ok(range) = wb.worksheet_range(&sheet) {
                let r0 = (parse_row_1_based(&row_str)? - 1) as u32;
                let c0 = (parse_col_1_based(&col_str)? - 1) as u32;

                if let Some(cell) = range.get_value((r0, c0)) {
                    found = !matches!(cell, Data::Empty);
                    out_value = cell.as_string().unwrap_or_default();
                } else {
                    ctx.log_message(
                        &format!("Cell not found at row {} col {}", row_str, col_str),
                        flow_like::flow::execution::LogLevel::Warn,
                    );
                }
            } else {
                return Err(flow_like_types::anyhow!("Sheet '{}' not found", sheet));
            }
        } else {
            return Err(flow_like_types::anyhow!(
                "Excel file is empty or could not be read"
            ));
        }

        ctx.set_pin_value("file_out", json!(file)).await?;
        ctx.set_pin_value("value", json!(out_value)).await?;
        ctx.set_pin_value("found", json!(found)).await?;

        ctx.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
