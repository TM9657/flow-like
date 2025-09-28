use crate::data::path::FlowPath;
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

/// Remove one or more columns from an XLSX worksheet (object-store aware).
/// - Works entirely in-memory using `FlowPath` bytes (no local filesystem I/O).
/// - If the file/sheet doesn't exist yet, a new workbook/sheet is created and the op is a no-op.
/// - `col` accepts Excel letters (A, AA, ...) **or** a 1-based number ("1", "27").
#[derive(Default)]
pub struct RemoveColumnNode {}

impl RemoveColumnNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for RemoveColumnNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "excel_remove_column",
            "Excel Remove Column",
            "Delete one or more columns from an XLSX sheet",
            "Data/Excel",
        );
        node.add_icon("/flow/icons/file-spreadsheet.svg");

        node.add_input_pin("exec_in", "In", "Trigger", VariableType::Execution);

        node.add_input_pin("file", "File", "Target XLSX file", VariableType::Struct)
            .set_schema::<FlowPath>();
        node.add_input_pin("sheet", "Sheet", "Worksheet name", VariableType::String)
            .set_default_value(Some(json!("Sheet1")));
        node.add_input_pin(
            "col",
            "Column",
            "Column letter(s) or 1-based number",
            VariableType::String,
        )
        .set_default_value(Some(json!("B")));
        node.add_input_pin(
            "count",
            "Count",
            "How many columns to remove",
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
        let col_in: String = ctx.evaluate_pin("col").await?;
        let count_in: String = ctx.evaluate_pin("count").await?;

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

        let col_letters = normalize_col_letters(&col_in)?;

        ws.remove_column(&col_letters, &count);

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

fn normalize_col_letters(input: &str) -> flow_like_types::Result<String> {
    let s = input.trim();
    // If it's numeric, convert to letters
    if let Ok(n) = s.parse::<u32>() {
        if n == 0 {
            return Err(flow_like_types::anyhow!(
                "Column number must be 1-based (>=1): {}",
                s
            ));
        }
        return Ok(col_index_to_letters_1_based(n));
    }
    // Otherwise validate letters and uppercase
    let upper = s.to_ascii_uppercase();
    for ch in upper.chars() {
        if !(('A'..='Z').contains(&ch)) {
            return Err(flow_like_types::anyhow!(
                "Invalid column '{}': only letters A-Z or a positive number are allowed",
                input
            ));
        }
    }
    Ok(upper)
}

fn col_index_to_letters_1_based(mut n: u32) -> String {
    let mut s = String::new();
    while n > 0 {
        let rem = ((n - 1) % 26) as u8;
        s.insert(0, (b'A' + rem) as char);
        n = (n - 1) / 26;
    }
    s
}
