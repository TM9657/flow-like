use super::element_utils::extract_element_id;
use flow_like::a2ui::components::GeoMapProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Pushes a single marker onto a GeoMap element without clearing existing ones.
#[crate::register_node]
#[derive(Default)]
pub struct PushGeoMapMarker;

impl PushGeoMapMarker {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for PushGeoMapMarker {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_push_geomap_marker",
            "Push GeoMap Marker",
            "Adds a single marker to a GeoMap without removing existing markers",
            "A2UI/Elements/GeoMap",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin("exec_in", "▶", "Execution input", VariableType::Execution);

        node.add_input_pin(
            "element_ref",
            "GeoMap",
            "Reference to the GeoMap element",
            VariableType::Struct,
        )
        .set_schema::<GeoMapProps>()
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_input_pin(
            "id",
            "Marker ID",
            "Unique identifier for the marker",
            VariableType::String,
        );

        node.add_input_pin(
            "latitude",
            "Latitude",
            "Marker latitude",
            VariableType::Float,
        );

        node.add_input_pin(
            "longitude",
            "Longitude",
            "Marker longitude",
            VariableType::Float,
        );

        node.add_input_pin(
            "color",
            "Color",
            "Marker color (red, blue, green, yellow, orange, purple, pink, gray)",
            VariableType::String,
        )
        .set_default_value(Some(json!("blue")));

        node.add_input_pin(
            "label",
            "Label",
            "Optional label displayed near the marker",
            VariableType::String,
        );

        node.add_input_pin(
            "popup",
            "Popup",
            "Optional popup text shown on marker click",
            VariableType::String,
        );

        node.add_input_pin(
            "draggable",
            "Draggable",
            "Whether the marker can be dragged by the user",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;

        let id: String = context.evaluate_pin("id").await?;
        let latitude: f64 = context.evaluate_pin("latitude").await?;
        let longitude: f64 = context.evaluate_pin("longitude").await?;
        let color: String = context.evaluate_pin("color").await.unwrap_or_default();
        let label: String = context.evaluate_pin("label").await.unwrap_or_default();
        let popup: String = context.evaluate_pin("popup").await.unwrap_or_default();
        let draggable: bool = context.evaluate_pin("draggable").await.unwrap_or(false);

        let mut marker = json!({
            "id": id,
            "coordinate": { "latitude": latitude, "longitude": longitude },
            "draggable": draggable
        });

        if !color.is_empty() {
            marker["color"] = json!(color);
        }
        if !label.is_empty() {
            marker["label"] = json!(label);
        }
        if !popup.is_empty() {
            marker["popup"] = json!(popup);
        }

        let update_value = json!({
            "type": "pushGeoMapMarker",
            "marker": marker
        });

        context.upsert_element(&element_id, update_value).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
