use super::element_utils::extract_element_id;
use flow_like::a2ui::components::NivoChartProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Configures Radar/Spider chart styling options.
///
/// **Documentation:** https://nivo.rocks/radar/
///
/// Controls grid shape, levels, dots, fill opacity, and border appearance.
#[crate::register_node]
#[derive(Default)]
pub struct SetRadarChartStyle;

impl SetRadarChartStyle {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetRadarChartStyle {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_radar_chart_style",
            "Set Radar Chart Style",
            "Configures Radar/Spider chart appearance. Docs: https://nivo.rocks/radar/",
            "UI/Elements/Charts/Radar",
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
            "grid_shape",
            "Grid Shape",
            "Grid line shape: 'circular' or 'linear' (polygon)",
            VariableType::String,
        )
        .set_default_value(Some(json!("circular")));

        node.add_input_pin(
            "grid_levels",
            "Grid Levels",
            "Number of concentric grid rings",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(5)));

        node.add_input_pin(
            "grid_label_offset",
            "Label Offset",
            "Distance of axis labels from center (pixels)",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(20)));

        node.add_input_pin(
            "enable_dots",
            "Enable Dots",
            "Show data point dots",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_input_pin(
            "dot_size",
            "Dot Size",
            "Size of data point dots (pixels)",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(10)));

        node.add_input_pin(
            "dot_border_width",
            "Dot Border Width",
            "Border thickness on dots (pixels)",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(2)));

        node.add_input_pin(
            "enable_dot_label",
            "Dot Labels",
            "Show value labels at data points",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_input_pin(
            "fill_opacity",
            "Fill Opacity",
            "Opacity of filled areas (0-1)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.25)));

        node.add_input_pin(
            "border_width",
            "Border Width",
            "Thickness of radar polygon borders (pixels)",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(2)));

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;

        let grid_shape: String = context.evaluate_pin("grid_shape").await?;
        let grid_levels: i64 = context.evaluate_pin("grid_levels").await?;
        let grid_label_offset: i64 = context.evaluate_pin("grid_label_offset").await?;
        let enable_dots: bool = context.evaluate_pin("enable_dots").await?;
        let dot_size: i64 = context.evaluate_pin("dot_size").await?;
        let dot_border_width: i64 = context.evaluate_pin("dot_border_width").await?;
        let enable_dot_label: bool = context.evaluate_pin("enable_dot_label").await?;
        let fill_opacity: f64 = context.evaluate_pin("fill_opacity").await?;
        let border_width: i64 = context.evaluate_pin("border_width").await?;

        let style = json!({
            "gridShape": grid_shape,
            "gridLevels": grid_levels,
            "gridLabelOffset": grid_label_offset,
            "enableDots": enable_dots,
            "dotSize": dot_size,
            "dotBorderWidth": dot_border_width,
            "enableDotLabel": enable_dot_label,
            "fillOpacity": fill_opacity,
            "borderWidth": border_width
        });

        context
            .upsert_element(&element_id, json!({ "radarStyle": style }))
            .await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
