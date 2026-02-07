use super::element_utils::extract_element_id;
use flow_like::a2ui::components::ImageHotspotProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Adds a single hotspot to an ImageHotspot element.
#[crate::register_node]
#[derive(Default)]
pub struct AddHotspot;

impl AddHotspot {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for AddHotspot {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_add_hotspot",
            "Add Hotspot",
            "Adds a hotspot to an ImageHotspot element",
            "UI/Elements/Hotspot",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin("exec_in", "▶", "Execution input", VariableType::Execution);

        node.add_input_pin(
            "element_ref",
            "Hotspot Image",
            "Reference to the ImageHotspot element",
            VariableType::Struct,
        )
        .set_schema::<ImageHotspotProps>()
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_input_pin(
            "id",
            "ID",
            "Unique identifier for the hotspot (auto-generated if empty)",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "x",
            "X",
            "X coordinate (pixels or normalized 0-1)",
            VariableType::Float,
        );

        node.add_input_pin(
            "y",
            "Y",
            "Y coordinate (pixels or normalized 0-1)",
            VariableType::Float,
        );

        node.add_input_pin(
            "size",
            "Size",
            "Hotspot size in pixels (default: 24)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(24.0)));

        node.add_input_pin(
            "label",
            "Label",
            "Label text shown on hover",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "description",
            "Description",
            "Description shown in tooltip",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "color",
            "Color",
            "Hotspot color (e.g., '#3b82f6')",
            VariableType::String,
        )
        .set_default_value(Some(json!("#3b82f6")));

        node.add_input_pin(
            "action",
            "Action",
            "Action name to trigger when clicked",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);

        node.add_output_pin(
            "hotspot_id",
            "Hotspot ID",
            "The ID of the created hotspot",
            VariableType::String,
        );

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;

        let mut id: String = context.evaluate_pin("id").await.unwrap_or_default();
        if id.is_empty() {
            id = format!("hotspot_{}", chrono::Utc::now().timestamp_millis());
        }

        let x: f64 = context.evaluate_pin("x").await?;
        let y: f64 = context.evaluate_pin("y").await?;
        let size: f64 = context.evaluate_pin("size").await.unwrap_or(24.0);
        let label: String = context.evaluate_pin("label").await.unwrap_or_default();
        let description: String = context
            .evaluate_pin("description")
            .await
            .unwrap_or_default();
        let color: String = context
            .evaluate_pin("color")
            .await
            .unwrap_or_else(|_| "#3b82f6".to_string());
        let action: String = context.evaluate_pin("action").await.unwrap_or_default();

        let mut hotspot_data = json!({
            "id": id,
            "x": x,
            "y": y,
            "size": size,
            "color": color
        });

        if !label.is_empty() {
            hotspot_data["label"] = json!(label);
        }
        if !description.is_empty() {
            hotspot_data["description"] = json!(description);
        }
        if !action.is_empty() {
            hotspot_data["action"] = json!(action);
        }

        let update_value = json!({
            "type": "addHotspot",
            "hotspot": hotspot_data
        });

        context.upsert_element(&element_id, update_value).await?;
        context.set_pin_value("hotspot_id", json!(id)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
