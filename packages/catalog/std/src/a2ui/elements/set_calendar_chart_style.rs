use super::element_utils::extract_element_id;
use flow_like::a2ui::components::NivoChartProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Configures Calendar heatmap styling options.
///
/// **Documentation:** https://nivo.rocks/calendar/
///
/// Controls direction, spacing, borders, and empty day colors.
#[crate::register_node]
#[derive(Default)]
pub struct SetCalendarChartStyle;

impl SetCalendarChartStyle {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetCalendarChartStyle {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_calendar_chart_style",
            "Set Calendar Chart Style",
            "Configures Calendar heatmap appearance. Docs: https://nivo.rocks/calendar/",
            "A2UI/Elements/Charts/Calendar",
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
        .set_default_value(Some(json!("horizontal")));

        node.add_input_pin(
            "empty_color",
            "Empty Color",
            "Color for days without data",
            VariableType::String,
        )
        .set_default_value(Some(json!("#eeeeee")));

        node.add_input_pin(
            "year_spacing",
            "Year Spacing",
            "Space between years",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(40)));

        node.add_input_pin(
            "year_legend_offset",
            "Year Legend Offset",
            "Year legend offset from calendar",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(10)));

        node.add_input_pin(
            "month_spacing",
            "Month Spacing",
            "Space between months",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(0)));

        node.add_input_pin(
            "month_border_width",
            "Month Border Width",
            "Month border width",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(2)));

        node.add_input_pin(
            "month_border_color",
            "Month Border Color",
            "Month border color",
            VariableType::String,
        )
        .set_default_value(Some(json!("#ffffff")));

        node.add_input_pin(
            "day_spacing",
            "Day Spacing",
            "Space between day cells",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(0)));

        node.add_input_pin(
            "day_border_width",
            "Day Border Width",
            "Day cell border width",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(1)));

        node.add_input_pin(
            "day_border_color",
            "Day Border Color",
            "Day cell border color",
            VariableType::String,
        )
        .set_default_value(Some(json!("#ffffff")));

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
        let empty_color: String = context.evaluate_pin("empty_color").await?;
        let year_spacing: i64 = context.evaluate_pin("year_spacing").await?;
        let year_legend_offset: i64 = context.evaluate_pin("year_legend_offset").await?;
        let month_spacing: i64 = context.evaluate_pin("month_spacing").await?;
        let month_border_width: i64 = context.evaluate_pin("month_border_width").await?;
        let month_border_color: String = context.evaluate_pin("month_border_color").await?;
        let day_spacing: i64 = context.evaluate_pin("day_spacing").await?;
        let day_border_width: i64 = context.evaluate_pin("day_border_width").await?;
        let day_border_color: String = context.evaluate_pin("day_border_color").await?;

        let style = json!({
            "direction": direction,
            "emptyColor": empty_color,
            "yearSpacing": year_spacing,
            "yearLegendOffset": year_legend_offset,
            "monthSpacing": month_spacing,
            "monthBorderWidth": month_border_width,
            "monthBorderColor": month_border_color,
            "daySpacing": day_spacing,
            "dayBorderWidth": day_border_width,
            "dayBorderColor": day_border_color
        });

        context
            .upsert_element(&element_id, json!({ "calendarStyle": style }))
            .await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
