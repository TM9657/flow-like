use super::element_utils::extract_element_id;
use flow_like::a2ui::components::NivoChartProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Configures Heatmap chart styling options.
///
/// **Documentation:** https://nivo.rocks/heatmap/
///
/// Controls cell appearance, labels, borders, and grid display.
#[crate::register_node]
#[derive(Default)]
pub struct SetHeatmapChartStyle;

impl SetHeatmapChartStyle {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetHeatmapChartStyle {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_heatmap_chart_style",
            "Set Heatmap Chart Style",
            "Configures Heatmap appearance. Docs: https://nivo.rocks/heatmap/",
            "A2UI/Elements/Charts/Heatmap",
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
            "force_square",
            "Force Square",
            "Force cells to be square shaped",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_input_pin(
            "size_variation",
            "Size Variation",
            "Cell size variation based on value (0-1)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.0)));

        node.add_input_pin(
            "cell_opacity",
            "Cell Opacity",
            "Cell opacity (0-1)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(1.0)));

        node.add_input_pin(
            "cell_border_width",
            "Cell Border Width",
            "Cell border width in pixels",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(0)));

        node.add_input_pin(
            "cell_border_color",
            "Cell Border Color",
            "Cell border color (e.g., '#000000')",
            VariableType::String,
        )
        .set_default_value(Some(json!("#000000")));

        node.add_input_pin(
            "enable_labels",
            "Enable Labels",
            "Show cell value labels",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_input_pin(
            "label_text_color",
            "Label Text Color",
            "Label text color (supports 'inherit:darker(1.4)' syntax)",
            VariableType::String,
        )
        .set_default_value(Some(json!("inherit:darker(1.4)")));

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;

        let force_square: bool = context.evaluate_pin("force_square").await?;
        let size_variation: f64 = context.evaluate_pin("size_variation").await?;
        let cell_opacity: f64 = context.evaluate_pin("cell_opacity").await?;
        let cell_border_width: i64 = context.evaluate_pin("cell_border_width").await?;
        let cell_border_color: String = context.evaluate_pin("cell_border_color").await?;
        let enable_labels: bool = context.evaluate_pin("enable_labels").await?;
        let label_text_color: String = context.evaluate_pin("label_text_color").await?;

        let style = json!({
            "forceSquare": force_square,
            "sizeVariation": size_variation,
            "cellOpacity": cell_opacity,
            "cellBorderWidth": cell_border_width,
            "cellBorderColor": cell_border_color,
            "enableLabels": enable_labels,
            "labelTextColor": label_text_color
        });

        context
            .upsert_element(&element_id, json!({ "heatmapStyle": style }))
            .await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
