use super::element_utils::extract_element_id;
use flow_like::a2ui::components::TableProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json, Value};

/// Updates a specific cell value in a table element.
#[crate::register_node]
#[derive(Default)]
pub struct UpdateTableCell;

impl UpdateTableCell {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for UpdateTableCell {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_update_table_cell",
            "Update Table Cell",
            "Updates a specific cell value in a table element",
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
            "row_index",
            "Row Index",
            "Index of the row to update (0-based)",
            VariableType::Integer,
        );

        node.add_input_pin(
            "column",
            "Column",
            "Column accessor/key to update",
            VariableType::String,
        );

        node.add_input_pin("value", "Value", "New cell value", VariableType::Generic);

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;

        let row_index: i64 = context.evaluate_pin("row_index").await?;
        let column: String = context.evaluate_pin("column").await?;
        let value: Value = context.evaluate_pin("value").await?;

        let update_value = json!({
            "type": "updateTableCell",
            "rowIndex": row_index,
            "column": column,
            "value": value
        });

        context.upsert_element(&element_id, update_value).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
