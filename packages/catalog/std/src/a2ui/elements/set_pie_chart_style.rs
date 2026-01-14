use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like::a2ui::components::NivoChartProps;
use flow_like_types::{Value, async_trait, json::json};
use super::element_utils::extract_element_id;

/// Configures Pie/Donut chart styling options.
///
/// **Documentation:** https://nivo.rocks/pie/
///
/// Controls inner radius (pie vs donut), padding, labels, and arc appearance.
#[crate::register_node]
#[derive(Default)]
pub struct SetPieChartStyle;

impl SetPieChartStyle {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetPieChartStyle {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_pie_chart_style",
            "Set Pie Chart Style",
            "Configures Pie/Donut chart appearance. Docs: https://nivo.rocks/pie/",
            "A2UI/Elements/Charts/Pie",
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
            "inner_radius",
            "Inner Radius",
            "0 = pie chart, >0 = donut chart (0-1, e.g., 0.5 for half)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.5)));

        node.add_input_pin(
            "pad_angle",
            "Pad Angle",
            "Gap between slices in degrees",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.7)));

        node.add_input_pin(
            "corner_radius",
            "Corner Radius",
            "Rounded corners on slices (pixels)",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(3)));

        node.add_input_pin(
            "start_angle",
            "Start Angle",
            "Starting angle in degrees (default: 0 = top)",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(0)));

        node.add_input_pin(
            "end_angle",
            "End Angle",
            "Ending angle in degrees (360 = full circle)",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(360)));

        node.add_input_pin(
            "sort_by_value",
            "Sort By Value",
            "Sort slices by value (largest first)",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_input_pin(
            "enable_arc_labels",
            "Arc Labels",
            "Show value labels inside slices",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_input_pin(
            "enable_arc_link_labels",
            "Link Labels",
            "Show labels with lines outside slices",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_input_pin(
            "arc_labels_skip_angle",
            "Skip Small Arcs",
            "Hide arc labels for slices smaller than this angle",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(10)));

        node.add_input_pin(
            "active_outer_radius_offset",
            "Hover Offset",
            "How much slice expands on hover (pixels)",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(8)));

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;

        let inner_radius: f64 = context.evaluate_pin("inner_radius").await?;
        let pad_angle: f64 = context.evaluate_pin("pad_angle").await?;
        let corner_radius: i64 = context.evaluate_pin("corner_radius").await?;
        let start_angle: i64 = context.evaluate_pin("start_angle").await?;
        let end_angle: i64 = context.evaluate_pin("end_angle").await?;
        let sort_by_value: bool = context.evaluate_pin("sort_by_value").await?;
        let enable_arc_labels: bool = context.evaluate_pin("enable_arc_labels").await?;
        let enable_arc_link_labels: bool = context.evaluate_pin("enable_arc_link_labels").await?;
        let arc_labels_skip_angle: i64 = context.evaluate_pin("arc_labels_skip_angle").await?;
        let active_outer_radius_offset: i64 = context.evaluate_pin("active_outer_radius_offset").await?;

        let style = json!({
            "innerRadius": inner_radius,
            "padAngle": pad_angle,
            "cornerRadius": corner_radius,
            "startAngle": start_angle,
            "endAngle": end_angle,
            "sortByValue": sort_by_value,
            "enableArcLabels": enable_arc_labels,
            "enableArcLinkLabels": enable_arc_link_labels,
            "arcLabelsSkipAngle": arc_labels_skip_angle,
            "arcLinkLabelsSkipAngle": arc_labels_skip_angle,
            "activeOuterRadiusOffset": active_outer_radius_offset
        });

        context.upsert_element(&element_id, json!({ "pieStyle": style })).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
