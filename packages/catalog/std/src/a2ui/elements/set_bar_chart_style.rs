use super::element_utils::extract_element_id;
use flow_like::a2ui::components::NivoChartProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Configures Bar chart styling options.
///
/// **Documentation:** https://nivo.rocks/bar/
///
/// Controls layout, grouping, padding, labels, grid, and visual appearance.
#[crate::register_node]
#[derive(Default)]
pub struct SetBarChartStyle;

impl SetBarChartStyle {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetBarChartStyle {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_bar_chart_style",
            "Set Bar Chart Style",
            "Configures Bar chart appearance. Docs: https://nivo.rocks/bar/",
            "A2UI/Elements/Charts/Bar",
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
            "Chart layout: 'vertical' (columns) or 'horizontal' (rows)",
            VariableType::String,
        )
        .set_default_value(Some(json!("vertical")));

        node.add_input_pin(
            "group_mode",
            "Group Mode",
            "How multiple series are displayed: 'grouped' (side by side) or 'stacked'",
            VariableType::String,
        )
        .set_default_value(Some(json!("grouped")));

        node.add_input_pin(
            "padding",
            "Padding",
            "Space between bars (0-1, default: 0.3)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.3)));

        node.add_input_pin(
            "inner_padding",
            "Inner Padding",
            "Space between grouped bars (0-1, default: 0)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.0)));

        node.add_input_pin(
            "border_radius",
            "Border Radius",
            "Rounded corners on bars (pixels)",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(0)));

        node.add_input_pin(
            "enable_label",
            "Enable Labels",
            "Show value labels on bars",
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

        let layout: String = context.evaluate_pin("layout").await?;
        let group_mode: String = context.evaluate_pin("group_mode").await?;
        let padding: f64 = context.evaluate_pin("padding").await?;
        let inner_padding: f64 = context.evaluate_pin("inner_padding").await?;
        let border_radius: i64 = context.evaluate_pin("border_radius").await?;
        let enable_label: bool = context.evaluate_pin("enable_label").await?;
        let enable_grid_x: bool = context.evaluate_pin("enable_grid_x").await?;
        let enable_grid_y: bool = context.evaluate_pin("enable_grid_y").await?;

        let style = json!({
            "layout": layout,
            "groupMode": group_mode,
            "padding": padding,
            "innerPadding": inner_padding,
            "borderRadius": border_radius,
            "enableLabel": enable_label,
            "enableGridX": enable_grid_x,
            "enableGridY": enable_grid_y
        });

        context
            .upsert_element(&element_id, json!({ "barStyle": style }))
            .await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
