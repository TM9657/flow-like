use super::element_utils::extract_element_id;
use flow_like::a2ui::components::NivoChartProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Configures Line chart styling options.
///
/// **Documentation:** https://nivo.rocks/line/
///
/// Controls curve type, line width, area fill, points, grid, and interactivity.
#[crate::register_node]
#[derive(Default)]
pub struct SetLineChartStyle;

impl SetLineChartStyle {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetLineChartStyle {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_line_chart_style",
            "Set Line Chart Style",
            "Configures Line chart appearance. Docs: https://nivo.rocks/line/",
            "A2UI/Elements/Charts/Line",
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
            "curve",
            "Curve Type",
            "Line interpolation: linear, monotoneX, natural, step, basis, cardinal, catmullRom",
            VariableType::String,
        )
        .set_default_value(Some(json!("monotoneX")));

        node.add_input_pin(
            "line_width",
            "Line Width",
            "Thickness of lines in pixels",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(2)));

        node.add_input_pin(
            "enable_area",
            "Enable Area",
            "Fill area under the line",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_input_pin(
            "area_opacity",
            "Area Opacity",
            "Opacity of filled area (0-1)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.2)));

        node.add_input_pin(
            "enable_points",
            "Enable Points",
            "Show data points on line",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_input_pin(
            "point_size",
            "Point Size",
            "Size of data points in pixels",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(10)));

        node.add_input_pin(
            "enable_slices",
            "Enable Slices",
            "Tooltip mode: 'x' (vertical), 'y' (horizontal), or false",
            VariableType::String,
        )
        .set_default_value(Some(json!("x")));

        node.add_input_pin(
            "enable_crosshair",
            "Enable Crosshair",
            "Show crosshair on hover",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_input_pin(
            "enable_grid_x",
            "Enable Grid X",
            "Show vertical grid lines",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_input_pin(
            "enable_grid_y",
            "Enable Grid Y",
            "Show horizontal grid lines",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;

        let curve: String = context.evaluate_pin("curve").await?;
        let line_width: i64 = context.evaluate_pin("line_width").await?;
        let enable_area: bool = context.evaluate_pin("enable_area").await?;
        let area_opacity: f64 = context.evaluate_pin("area_opacity").await?;
        let enable_points: bool = context.evaluate_pin("enable_points").await?;
        let point_size: i64 = context.evaluate_pin("point_size").await?;
        let enable_slices: String = context.evaluate_pin("enable_slices").await?;
        let enable_crosshair: bool = context.evaluate_pin("enable_crosshair").await?;
        let enable_grid_x: bool = context.evaluate_pin("enable_grid_x").await?;
        let enable_grid_y: bool = context.evaluate_pin("enable_grid_y").await?;

        let slices_value = if enable_slices == "false" || enable_slices.is_empty() {
            json!(false)
        } else {
            json!(enable_slices)
        };

        let style = json!({
            "curve": curve,
            "lineWidth": line_width,
            "enableArea": enable_area,
            "areaOpacity": area_opacity,
            "enablePoints": enable_points,
            "pointSize": point_size,
            "enableSlices": slices_value,
            "enableCrosshair": enable_crosshair,
            "enableGridX": enable_grid_x,
            "enableGridY": enable_grid_y
        });

        context
            .upsert_element(&element_id, json!({ "lineStyle": style }))
            .await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
