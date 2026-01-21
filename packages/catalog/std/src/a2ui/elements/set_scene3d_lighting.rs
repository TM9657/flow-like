use super::element_utils::extract_element_id;
use flow_like::a2ui::components::Scene3dProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

#[crate::register_node]
#[derive(Default)]
pub struct SetScene3dLighting;

impl SetScene3dLighting {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetScene3dLighting {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_scene3d_lighting",
            "Set Scene3D Lighting",
            "Sets the lighting options for a 3D scene",
            "A2UI/Elements/Game",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin("exec_in", "▶", "Execution input", VariableType::Execution);

        node.add_input_pin(
            "element_ref",
            "Scene3D",
            "Reference to the 3D scene element",
            VariableType::Struct,
        )
        .set_schema::<Scene3dProps>()
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_input_pin(
            "ambient_intensity",
            "Ambient Intensity",
            "Ambient light intensity (0-1)",
            VariableType::Float,
        )
        .set_options(
            PinOptions::new()
                .set_range((0.0, 1.0))
                .set_step(0.1)
                .build(),
        );

        node.add_input_pin(
            "ambient_color",
            "Ambient Color",
            "Ambient light color (hex or CSS color)",
            VariableType::String,
        );

        node.add_input_pin(
            "directional_intensity",
            "Directional Intensity",
            "Directional light intensity (0-1)",
            VariableType::Float,
        )
        .set_options(
            PinOptions::new()
                .set_range((0.0, 1.0))
                .set_step(0.1)
                .build(),
        );

        node.add_input_pin(
            "directional_color",
            "Directional Color",
            "Directional light color (hex or CSS color)",
            VariableType::String,
        );

        node.add_input_pin(
            "directional_position",
            "Directional Position",
            "Directional light position as [x, y, z]",
            VariableType::Generic,
        );

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);
        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;

        let mut props = json!({});

        // Build ambient light object if any ambient properties are provided
        let mut ambient = json!({});
        if let Ok(intensity) = context.evaluate_pin::<f64>("ambient_intensity").await {
            ambient["intensity"] = json!(intensity);
        }
        if let Ok(color) = context.evaluate_pin::<String>("ambient_color").await {
            ambient["color"] = json!(color);
        }
        if !ambient.as_object().unwrap().is_empty() {
            props["ambientLight"] = ambient;
        }

        // Build directional light object if any directional properties are provided
        let mut directional = json!({});
        if let Ok(intensity) = context.evaluate_pin::<f64>("directional_intensity").await {
            directional["intensity"] = json!(intensity);
        }
        if let Ok(color) = context.evaluate_pin::<String>("directional_color").await {
            directional["color"] = json!(color);
        }
        if let Ok(position) = context.evaluate_pin::<Value>("directional_position").await {
            directional["position"] = position;
        }
        if !directional.as_object().unwrap().is_empty() {
            props["directionalLight"] = directional;
        }

        let update_value = json!({
            "type": "setProps",
            "props": props
        });

        context.upsert_element(&element_id, update_value).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
