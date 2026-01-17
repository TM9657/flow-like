use super::element_utils::extract_element_id;
use flow_like::a2ui::components::NivoChartProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Configures Funnel chart styling options.
///
/// **Documentation:** https://nivo.rocks/funnel/
///
/// Controls direction, interpolation, spacing, and labels.
#[crate::register_node]
#[derive(Default)]
pub struct SetFunnelChartStyle;

impl SetFunnelChartStyle {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetFunnelChartStyle {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_funnel_chart_style",
            "Set Funnel Chart Style",
            "Configures Funnel chart appearance. Docs: https://nivo.rocks/funnel/",
            "A2UI/Elements/Charts/Funnel",
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
            "direction",
            "Direction",
            "'horizontal' or 'vertical'",
            VariableType::String,
        )
        .set_default_value(Some(json!("vertical")));

        node.add_input_pin(
            "interpolation",
            "Interpolation",
            "'smooth' or 'linear'",
            VariableType::String,
        )
        .set_default_value(Some(json!("smooth")));

        node.add_input_pin(
            "spacing",
            "Spacing",
            "Spacing between parts",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(0)));

        node.add_input_pin(
            "shape_blending",
            "Shape Blending",
            "Blending between parts (0-1)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.66)));

        node.add_input_pin(
            "enable_label",
            "Enable Labels",
            "Show part labels",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_input_pin(
            "label_color",
            "Label Color",
            "Label text color",
            VariableType::String,
        )
        .set_default_value(Some(json!("inherit:darker(1.4)")));

        node.add_input_pin(
            "border_width",
            "Border Width",
            "Part border width",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(0)));

        node.add_input_pin(
            "border_opacity",
            "Border Opacity",
            "Part border opacity (0-1)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.5)));

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;

        let direction: String = context.evaluate_pin("direction").await?;
        let interpolation: String = context.evaluate_pin("interpolation").await?;
        let spacing: i64 = context.evaluate_pin("spacing").await?;
        let shape_blending: f64 = context.evaluate_pin("shape_blending").await?;
        let enable_label: bool = context.evaluate_pin("enable_label").await?;
        let label_color: String = context.evaluate_pin("label_color").await?;
        let border_width: i64 = context.evaluate_pin("border_width").await?;
        let border_opacity: f64 = context.evaluate_pin("border_opacity").await?;

        let style = json!({
            "direction": direction,
            "interpolation": interpolation,
            "spacing": spacing,
            "shapeBlending": shape_blending,
            "enableLabel": enable_label,
            "labelColor": label_color,
            "borderWidth": border_width,
            "borderOpacity": border_opacity
        });

        context
            .upsert_element(&element_id, json!({ "funnelStyle": style }))
            .await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
