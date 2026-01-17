use super::element_utils::extract_element_id;
use flow_like::a2ui::components::Model3dProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

#[crate::register_node]
#[derive(Default)]
pub struct SetModel3dAnimation;

impl SetModel3dAnimation {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetModel3dAnimation {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_model3d_animation",
            "Set Model3D Animation",
            "Sets the animation and rotation options for a 3D model",
            "A2UI/Elements/Game",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin("exec_in", "▶", "Execution input", VariableType::Execution);

        node.add_input_pin(
            "element_ref",
            "Model3D",
            "Reference to the 3D model element",
            VariableType::Struct,
        )
        .set_schema::<Model3dProps>()
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_input_pin(
            "animation",
            "Animation Name",
            "Name of the animation to play (or empty to stop)",
            VariableType::String,
        );

        node.add_input_pin(
            "auto_rotate",
            "Auto Rotate",
            "Enable automatic rotation of the model",
            VariableType::Boolean,
        );

        node.add_input_pin(
            "rotate_speed",
            "Rotate Speed",
            "Rotation speed in radians per second",
            VariableType::Float,
        )
        .set_options(
            PinOptions::new()
                .set_range((0.0, 10.0))
                .set_step(0.1)
                .build(),
        );

        node.add_input_pin(
            "cast_shadow",
            "Cast Shadow",
            "Whether the model casts shadows",
            VariableType::Boolean,
        );

        node.add_input_pin(
            "receive_shadow",
            "Receive Shadow",
            "Whether the model receives shadows",
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

        let mut props = json!({});

        if let Ok(animation) = context.evaluate_pin::<String>("animation").await {
            props["animation"] = json!(animation);
        }

        if let Ok(auto_rotate) = context.evaluate_pin::<bool>("auto_rotate").await {
            props["autoRotate"] = json!(auto_rotate);
        }

        if let Ok(speed) = context.evaluate_pin::<f64>("rotate_speed").await {
            props["rotateSpeed"] = json!(speed);
        }

        if let Ok(cast_shadow) = context.evaluate_pin::<bool>("cast_shadow").await {
            props["castShadow"] = json!(cast_shadow);
        }

        if let Ok(receive_shadow) = context.evaluate_pin::<bool>("receive_shadow").await {
            props["receiveShadow"] = json!(receive_shadow);
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
