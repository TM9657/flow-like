use crate::data::{
    excel::{parse_col_1_based, parse_row_1_based},
    path::FlowPath,
};
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json};
#[cfg(feature = "execute")]
use umya_spreadsheet::{self};

/// Write a single cell inside an Excel workbook (XLSX).
/// Works with virtual/object-store files via `FlowPath` (no local filesystem I/O).
/// Creates the file if it does not exist and the sheet if it is missing.
/// Column and Row correspond to the components of an A1 address
/// (e.g. for "B3": col = "B", row = "3").
/// The updated (same) `FlowPath` is returned so downstream nodes can re-use the file.
#[crate::register_node]
#[derive(Default)]
pub struct WriteCellNode {}

impl WriteCellNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for WriteCellNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "excel_write_cell",
            "Excel Write Cell",
            "Write/update a single cell value in an XLSX sheet",
            "Data/Excel",
        );
        node.add_icon("/flow/icons/file-spreadsheet.svg");

        // Impure node → needs execution pins
        node.add_input_pin("exec_in", "In", "Trigger", VariableType::Execution);

        node.add_input_pin("file", "File", "Target XLSX file", VariableType::Struct)
            .set_schema::<FlowPath>();
        node.add_input_pin("sheet", "Sheet", "Worksheet name", VariableType::String)
            .set_default_value(Some(json!("Sheet1")));
        node.add_input_pin("row", "Row", "Row number (1-based)", VariableType::String)
            .set_default_value(Some(json!("1")));
        node.add_input_pin(
            "col",
            "Column",
            "Column (letter(s) like A, AA, or 1-based number)",
            VariableType::String,
        )
        .set_default_value(Some(json!("A")));
        node.add_input_pin(
            "value",
            "Value",
            "Value to write (string)",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_output_pin("exec_out", "Out", "Trigger", VariableType::Execution);
        node.add_output_pin(
            "file_out",
            "File",
            "Updated XLSX path",
            VariableType::Struct,
        )
        .set_schema::<FlowPath>();

        node
    }

    #[cfg(feature = "execute")]
    async fn run(&self, ctx: &mut ExecutionContext) -> flow_like_types::Result<()> {
        ctx.deactivate_exec_pin("exec_out").await?;

        let file: FlowPath = ctx.evaluate_pin("file").await?;
        let sheet: String = ctx.evaluate_pin("sheet").await?;
        let row_str: String = ctx.evaluate_pin("row").await?;
        let col_str: String = ctx.evaluate_pin("col").await?;
        let value: String = ctx.evaluate_pin("value").await?;

        let file_content: Vec<u8> = file.get(ctx, false).await?;
        let file_content_reader = std::io::Cursor::new(&file_content);
        let mut book = match umya_spreadsheet::reader::xlsx::read_reader(file_content_reader, true)
        {
            Ok(b) => b,
            Err(e) => return Err(flow_like_types::anyhow!("Failed to read workbook: {}", e)),
        };

        let _ = if book.get_sheet_by_name(&sheet).is_some() {
        } else {
            book.new_sheet(&sheet)
                .map_err(|e| flow_like_types::anyhow!("Failed to create sheet: {}", e))?;
        };
        let ws = book.get_sheet_by_name_mut(&sheet).ok_or_else(|| {
            flow_like_types::anyhow!("Failed to access or create sheet: {}", sheet)
        })?;

        // Parse row & column (both 1-based)
        let row = parse_row_1_based(&row_str)?;
        let col = parse_col_1_based(&col_str)?;

        // Set cell value
        {
            let cell = ws.get_cell_mut((col, row));
            cell.set_value(value.clone());
        }

        let mut out: Vec<u8> = Vec::new();
        if let Err(e) = umya_spreadsheet::writer::xlsx::write_writer(&book, &mut out) {
            return Err(flow_like_types::anyhow!(
                "Failed to serialize workbook: {}",
                e
            ));
        }

        file.put(ctx, out, false).await?;

        ctx.set_pin_value("file_out", json!(file)).await?;
        ctx.activate_exec_pin("exec_out").await?;
        Ok(())
    }

    #[cfg(not(feature = "execute"))]
    async fn run(&self, _context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        Err(flow_like_types::anyhow!(
            "Data processing requires the 'execute' feature"
        ))
    }
}
