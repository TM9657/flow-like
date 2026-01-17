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
pub struct SetScene3dControls;

impl SetScene3dControls {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetScene3dControls {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_scene3d_controls",
            "Set Scene3D Controls",
            "Sets the control mode and options for a 3D scene",
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
            "control_mode",
            "Control Mode",
            "Camera control mode: orbit, fly, fixed, auto-rotate",
            VariableType::String,
        )
        .set_options(
            PinOptions::new()
                .set_valid_values(vec![
                    "orbit".to_string(),
                    "fly".to_string(),
                    "fixed".to_string(),
                    "auto-rotate".to_string(),
                ])
                .build(),
        );

        node.add_input_pin(
            "fixed_view",
            "Fixed View",
            "For fixed mode: front, back, left, right, top, bottom, isometric",
            VariableType::String,
        )
        .set_options(
            PinOptions::new()
                .set_valid_values(vec![
                    "front".to_string(),
                    "back".to_string(),
                    "left".to_string(),
                    "right".to_string(),
                    "top".to_string(),
                    "bottom".to_string(),
                    "isometric".to_string(),
                ])
                .build(),
        );

        node.add_input_pin(
            "auto_rotate_speed",
            "Auto Rotate Speed",
            "Auto-rotation speed in degrees per second (default: 30)",
            VariableType::Float,
        );

        node.add_input_pin(
            "enable_zoom",
            "Enable Zoom",
            "Enable zoom controls",
            VariableType::Boolean,
        );

        node.add_input_pin(
            "enable_pan",
            "Enable Pan",
            "Enable pan controls",
            VariableType::Boolean,
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

        let control_mode: String = context.evaluate_pin("control_mode").await?;

        let mut props = json!({
            "controlMode": control_mode
        });

        if let Ok(fixed_view) = context.evaluate_pin::<String>("fixed_view").await {
            props["fixedView"] = json!(fixed_view);
        }

        if let Ok(speed) = context.evaluate_pin::<f64>("auto_rotate_speed").await {
            props["autoRotateSpeed"] = json!(speed);
        }

        if let Ok(zoom) = context.evaluate_pin::<bool>("enable_zoom").await {
            props["enableZoom"] = json!(zoom);
        }

        if let Ok(pan) = context.evaluate_pin::<bool>("enable_pan").await {
            props["enablePan"] = json!(pan);
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
