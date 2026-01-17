use super::element_utils::extract_element_id;
use flow_like::a2ui::components::TableProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Sets the column definitions for a table element.
///
/// Each column should have an id and header, with optional accessor, width, align, sortable, and hidden.
#[crate::register_node]
#[derive(Default)]
pub struct SetTableColumns;

impl SetTableColumns {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetTableColumns {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_table_columns",
            "Set Table Columns",
            "Sets the column definitions for a table element",
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
            "columns",
            "Columns",
            "Array of column definitions with id, header, accessor, width, align, sortable, hidden",
            VariableType::Generic,
        );

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;

        let columns: Value = context.evaluate_pin("columns").await?;

        let update_value = json!({
            "type": "setTableColumns",
            "columns": columns
        });

        context.upsert_element(&element_id, update_value).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
