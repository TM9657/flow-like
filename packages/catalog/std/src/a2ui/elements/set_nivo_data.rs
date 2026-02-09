use super::element_utils::extract_element_id;
use flow_like::a2ui::components::NivoChartProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Sets the data for a Nivo chart element.
///
/// Accepts chart data as JSON that will be passed to the Nivo chart.
/// The data format varies by chart type.
#[crate::register_node]
#[derive(Default)]
pub struct SetNivoData;

impl SetNivoData {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetNivoData {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_nivo_data",
            "Set Nivo Chart Data",
            "Sets the data for a Nivo chart element",
            "UI/Elements/Charts",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin("exec_in", "▶", "Execution input", VariableType::Execution);

        node.add_input_pin(
            "element_ref",
            "Chart",
            "Reference to the Nivo chart element (ID or element object)",
            VariableType::Struct,
        )
        .set_schema::<NivoChartProps>()
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_input_pin(
            "data",
            "Data",
            "Chart data (format depends on chart type - see Nivo docs)",
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
            "type": "setNivoData",
            "data": data
        });

        context.upsert_element(&element_id, update_value).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
