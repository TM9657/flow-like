use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like::a2ui::components::NivoChartProps;
use flow_like_types::{Value, async_trait, json::json};
use super::element_utils::extract_element_id;

/// Configures Sankey diagram styling options.
///
/// **Documentation:** https://nivo.rocks/sankey/
///
/// Controls layout, alignment, node and link appearance, and labels.
#[crate::register_node]
#[derive(Default)]
pub struct SetSankeyChartStyle;

impl SetSankeyChartStyle {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetSankeyChartStyle {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_sankey_chart_style",
            "Set Sankey Chart Style",
            "Configures Sankey diagram appearance. Docs: https://nivo.rocks/sankey/",
            "A2UI/Elements/Charts/Sankey",
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
            "layout",
            "Layout",
            "'horizontal' or 'vertical'",
            VariableType::String,
        )
        .set_default_value(Some(json!("horizontal")));

        node.add_input_pin(
            "align",
            "Alignment",
            "'center', 'justify', 'start', 'end'",
            VariableType::String,
        )
        .set_default_value(Some(json!("justify")));

        node.add_input_pin(
            "sort",
            "Sort",
            "'auto', 'input', 'ascending', 'descending'",
            VariableType::String,
        )
        .set_default_value(Some(json!("auto")));

        node.add_input_pin(
            "node_opacity",
            "Node Opacity",
            "Node opacity (0-1)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(1.0)));

        node.add_input_pin(
            "node_thickness",
            "Node Thickness",
            "Thickness of node bars",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(18)));

        node.add_input_pin(
            "node_spacing",
            "Node Spacing",
            "Spacing between nodes",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(24)));

        node.add_input_pin(
            "node_inner_padding",
            "Node Inner Padding",
            "Inner padding within nodes",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(3)));

        node.add_input_pin(
            "node_border_width",
            "Node Border Width",
            "Node border width",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(0)));

        node.add_input_pin(
            "link_opacity",
            "Link Opacity",
            "Link opacity (0-1)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.5)));

        node.add_input_pin(
            "link_blend_mode",
            "Link Blend Mode",
            "CSS blend mode for links (e.g., 'multiply')",
            VariableType::String,
        )
        .set_default_value(Some(json!("multiply")));

        node.add_input_pin(
            "enable_link_gradient",
            "Enable Link Gradient",
            "Apply gradient to links",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_input_pin(
            "enable_labels",
            "Enable Labels",
            "Show node labels",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_input_pin(
            "label_position",
            "Label Position",
            "'inside' or 'outside'",
            VariableType::String,
        )
        .set_default_value(Some(json!("outside")));

        node.add_input_pin(
            "label_padding",
            "Label Padding",
            "Padding between label and node",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(9)));

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;

        let layout: String = context.evaluate_pin("layout").await?;
        let align: String = context.evaluate_pin("align").await?;
        let sort: String = context.evaluate_pin("sort").await?;
        let node_opacity: f64 = context.evaluate_pin("node_opacity").await?;
        let node_thickness: i64 = context.evaluate_pin("node_thickness").await?;
        let node_spacing: i64 = context.evaluate_pin("node_spacing").await?;
        let node_inner_padding: i64 = context.evaluate_pin("node_inner_padding").await?;
        let node_border_width: i64 = context.evaluate_pin("node_border_width").await?;
        let link_opacity: f64 = context.evaluate_pin("link_opacity").await?;
        let link_blend_mode: String = context.evaluate_pin("link_blend_mode").await?;
        let enable_link_gradient: bool = context.evaluate_pin("enable_link_gradient").await?;
        let enable_labels: bool = context.evaluate_pin("enable_labels").await?;
        let label_position: String = context.evaluate_pin("label_position").await?;
        let label_padding: i64 = context.evaluate_pin("label_padding").await?;

        let style = json!({
            "layout": layout,
            "align": align,
            "sort": sort,
            "nodeOpacity": node_opacity,
            "nodeThickness": node_thickness,
            "nodeSpacing": node_spacing,
            "nodeInnerPadding": node_inner_padding,
            "nodeBorderWidth": node_border_width,
            "linkOpacity": link_opacity,
            "linkBlendMode": link_blend_mode,
            "enableLinkGradient": enable_link_gradient,
            "enableLabels": enable_labels,
            "labelPosition": label_position,
            "labelPadding": label_padding
        });

        context.upsert_element(&element_id, json!({ "sankeyStyle": style })).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
