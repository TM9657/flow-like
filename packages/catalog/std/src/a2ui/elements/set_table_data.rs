use super::element_utils::extract_element_id;
use flow_like::a2ui::components::TableProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json, Value};

/// Sets the data for a table element.
///
/// The data should be an array of objects where each object represents a row.
#[crate::register_node]
#[derive(Default)]
pub struct SetTableData;

impl SetTableData {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetTableData {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_table_data",
            "Set Table Data",
            "Sets the data rows for a table element",
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
            "data",
            "Data",
            "Array of row objects",
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

        let data: Value = context.evaluate_pin("data").await?;

        let update_value = json!({
            "type": "setTableData",
            "data": data
        });

        context.upsert_element(&element_id, update_value).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
