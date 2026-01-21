use super::element_utils::extract_element_id;
use flow_like::a2ui::components::NivoChartProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Configures Treemap chart styling options.
///
/// **Documentation:** https://nivo.rocks/treemap/
///
/// Also applies to sunburst and circle packing charts with hierarchical data.
/// Controls tiling method, padding, labels, and borders.
#[crate::register_node]
#[derive(Default)]
pub struct SetTreemapChartStyle;

impl SetTreemapChartStyle {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetTreemapChartStyle {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_treemap_chart_style",
            "Set Treemap Chart Style",
            "Configures Treemap/Sunburst appearance. Docs: https://nivo.rocks/treemap/",
            "A2UI/Elements/Charts/Treemap",
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
            "tile",
            "Tile Method",
            "'squarify', 'binary', 'slice', 'dice', 'sliceDice', 'resquarify'",
            VariableType::String,
        )
        .set_default_value(Some(json!("squarify")));

        node.add_input_pin(
            "leaves_only",
            "Leaves Only",
            "Only show leaf nodes",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_input_pin(
            "inner_padding",
            "Inner Padding",
            "Padding between parent and children",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(0)));

        node.add_input_pin(
            "outer_padding",
            "Outer Padding",
            "Padding around the treemap",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(0)));

        node.add_input_pin(
            "enable_label",
            "Enable Labels",
            "Show node labels",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_input_pin(
            "enable_parent_label",
            "Enable Parent Labels",
            "Show parent node labels",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_input_pin(
            "label_skip_size",
            "Label Skip Size",
            "Skip labels for nodes smaller than this",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(0)));

        node.add_input_pin(
            "label_text_color",
            "Label Text Color",
            "Label text color",
            VariableType::String,
        )
        .set_default_value(Some(json!("inherit:darker(1.2)")));

        node.add_input_pin(
            "border_width",
            "Border Width",
            "Node border width",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(0)));

        node.add_input_pin(
            "border_color",
            "Border Color",
            "Node border color",
            VariableType::String,
        )
        .set_default_value(Some(json!("inherit:darker(0.3)")));

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;

        let tile: String = context.evaluate_pin("tile").await?;
        let leaves_only: bool = context.evaluate_pin("leaves_only").await?;
        let inner_padding: i64 = context.evaluate_pin("inner_padding").await?;
        let outer_padding: i64 = context.evaluate_pin("outer_padding").await?;
        let enable_label: bool = context.evaluate_pin("enable_label").await?;
        let enable_parent_label: bool = context.evaluate_pin("enable_parent_label").await?;
        let label_skip_size: i64 = context.evaluate_pin("label_skip_size").await?;
        let label_text_color: String = context.evaluate_pin("label_text_color").await?;
        let border_width: i64 = context.evaluate_pin("border_width").await?;
        let border_color: String = context.evaluate_pin("border_color").await?;

        let style = json!({
            "tile": tile,
            "leavesOnly": leaves_only,
            "innerPadding": inner_padding,
            "outerPadding": outer_padding,
            "enableLabel": enable_label,
            "enableParentLabel": enable_parent_label,
            "labelSkipSize": label_skip_size,
            "labelTextColor": label_text_color,
            "borderWidth": border_width,
            "borderColor": border_color
        });

        context
            .upsert_element(&element_id, json!({ "treemapStyle": style }))
            .await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
