use super::element_utils::extract_element_id;
use flow_like::a2ui::components::NivoChartProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Configures Chord diagram styling options.
///
/// **Documentation:** https://nivo.rocks/chord/
///
/// Controls arc and ribbon appearance, labels, and angles.
#[crate::register_node]
#[derive(Default)]
pub struct SetChordChartStyle;

impl SetChordChartStyle {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetChordChartStyle {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_chord_chart_style",
            "Set Chord Chart Style",
            "Configures Chord diagram appearance. Docs: https://nivo.rocks/chord/",
            "UI/Elements/Charts/Chord",
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
            "pad_angle",
            "Pad Angle",
            "Angle between arcs in radians",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.02)));

        node.add_input_pin(
            "inner_radius_ratio",
            "Inner Radius Ratio",
            "Inner radius as ratio of outer radius",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.9)));

        node.add_input_pin(
            "inner_radius_offset",
            "Inner Radius Offset",
            "Offset to apply to inner radius",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.0)));

        node.add_input_pin(
            "arc_opacity",
            "Arc Opacity",
            "Opacity of arc segments (0-1)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(1.0)));

        node.add_input_pin(
            "arc_border_width",
            "Arc Border Width",
            "Arc border width",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(1)));

        node.add_input_pin(
            "arc_border_color",
            "Arc Border Color",
            "Arc border color",
            VariableType::String,
        )
        .set_default_value(Some(json!("inherit:darker(0.4)")));

        node.add_input_pin(
            "ribbon_opacity",
            "Ribbon Opacity",
            "Opacity of connecting ribbons (0-1)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.5)));

        node.add_input_pin(
            "ribbon_border_width",
            "Ribbon Border Width",
            "Ribbon border width",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(1)));

        node.add_input_pin(
            "ribbon_border_color",
            "Ribbon Border Color",
            "Ribbon border color",
            VariableType::String,
        )
        .set_default_value(Some(json!("inherit:darker(0.4)")));

        node.add_input_pin(
            "enable_label",
            "Enable Labels",
            "Show arc labels",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_input_pin(
            "label_offset",
            "Label Offset",
            "Label distance from arc",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(12)));

        node.add_input_pin(
            "label_rotation",
            "Label Rotation",
            "Label rotation in degrees",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.0)));

        node.add_input_pin(
            "label_text_color",
            "Label Text Color",
            "Label text color",
            VariableType::String,
        )
        .set_default_value(Some(json!("inherit:darker(1)")));

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;

        let pad_angle: f64 = context.evaluate_pin("pad_angle").await?;
        let inner_radius_ratio: f64 = context.evaluate_pin("inner_radius_ratio").await?;
        let inner_radius_offset: f64 = context.evaluate_pin("inner_radius_offset").await?;
        let arc_opacity: f64 = context.evaluate_pin("arc_opacity").await?;
        let arc_border_width: i64 = context.evaluate_pin("arc_border_width").await?;
        let arc_border_color: String = context.evaluate_pin("arc_border_color").await?;
        let ribbon_opacity: f64 = context.evaluate_pin("ribbon_opacity").await?;
        let ribbon_border_width: i64 = context.evaluate_pin("ribbon_border_width").await?;
        let ribbon_border_color: String = context.evaluate_pin("ribbon_border_color").await?;
        let enable_label: bool = context.evaluate_pin("enable_label").await?;
        let label_offset: i64 = context.evaluate_pin("label_offset").await?;
        let label_rotation: f64 = context.evaluate_pin("label_rotation").await?;
        let label_text_color: String = context.evaluate_pin("label_text_color").await?;

        let style = json!({
            "padAngle": pad_angle,
            "innerRadiusRatio": inner_radius_ratio,
            "innerRadiusOffset": inner_radius_offset,
            "arcOpacity": arc_opacity,
            "arcBorderWidth": arc_border_width,
            "arcBorderColor": arc_border_color,
            "ribbonOpacity": ribbon_opacity,
            "ribbonBorderWidth": ribbon_border_width,
            "ribbonBorderColor": ribbon_border_color,
            "enableLabel": enable_label,
            "labelOffset": label_offset,
            "labelRotation": label_rotation,
            "labelTextColor": label_text_color
        });

        context
            .upsert_element(&element_id, json!({ "chordStyle": style }))
            .await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
