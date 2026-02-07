use super::element_utils::extract_element_id;
use flow_like::a2ui::components::NivoChartProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Configures Scatter plot styling options.
///
/// **Documentation:** https://nivo.rocks/scatterplot/
///
/// Controls node size, grid visibility, and mesh interactions.
#[crate::register_node]
#[derive(Default)]
pub struct SetScatterChartStyle;

impl SetScatterChartStyle {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetScatterChartStyle {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_scatter_chart_style",
            "Set Scatter Chart Style",
            "Configures Scatter plot appearance. Docs: https://nivo.rocks/scatterplot/",
            "UI/Elements/Charts/Scatter",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin("exec_in", "▶", "Execution input", VariableType::Execution);

        node.add_input_pin(
            "element_ref",
            "Chart",
            "Reference to the NivoChart element",
            VariableType::Struct,
        )
        .set_schema::<NivoChartProps>()
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_input_pin(
            "node_size",
            "Node Size",
            "Size of scatter points",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(9)));

        node.add_input_pin(
            "enable_grid_x",
            "Enable Grid X",
            "Show vertical grid lines",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_input_pin(
            "enable_grid_y",
            "Enable Grid Y",
            "Show horizontal grid lines",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_input_pin(
            "use_mesh",
            "Use Mesh",
            "Enable Voronoi mesh for interactions",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_input_pin(
            "debug_mesh",
            "Debug Mesh",
            "Show Voronoi mesh for debugging",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;

        let node_size: i64 = context.evaluate_pin("node_size").await?;
        let enable_grid_x: bool = context.evaluate_pin("enable_grid_x").await?;
        let enable_grid_y: bool = context.evaluate_pin("enable_grid_y").await?;
        let use_mesh: bool = context.evaluate_pin("use_mesh").await?;
        let debug_mesh: bool = context.evaluate_pin("debug_mesh").await?;

        let style = json!({
            "nodeSize": node_size,
            "enableGridX": enable_grid_x,
            "enableGridY": enable_grid_y,
            "useMesh": use_mesh,
            "debugMesh": debug_mesh
        });

        context
            .upsert_element(&element_id, json!({ "scatterStyle": style }))
            .await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
