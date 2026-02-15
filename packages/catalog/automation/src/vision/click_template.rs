use crate::types::handles::AutomationSession;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    variable::VariableType,
};
use flow_like_catalog_core::FlowPath;
use flow_like_types::{async_trait, json::json};

#[crate::register_node]
#[derive(Default)]
pub struct ClickTemplateNode {}

impl ClickTemplateNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for ClickTemplateNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "vision_click_template",
            "Click Template",
            "Finds a template image on screen and clicks on it",
            "Automation/Vision",
        );
        node.add_icon("/flow/icons/vision.svg");

        node.set_scores(
            flow_like::flow::node::NodeScores::new()
                .set_privacy(2)
                .set_security(4)
                .set_performance(6)
                .set_governance(5)
                .set_reliability(7)
                .set_cost(9)
                .build(),
        );
        node.set_only_offline(true);

        node.add_input_pin("exec_in", "▶", "Trigger", VariableType::Execution);

        node.add_input_pin(
            "session",
            "Session",
            "Automation session handle (provides template matching via rustautogui)",
            VariableType::Struct,
        )
        .set_schema::<AutomationSession>();

        node.add_input_pin(
            "template",
            "Template",
            "Path to the template image file (FlowPath with caching support)",
            VariableType::Struct,
        )
        .set_schema::<FlowPath>();

        node.add_input_pin(
            "confidence",
            "Confidence",
            "Minimum match confidence (0.0-1.0)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.8)));

        node.add_input_pin(
            "click_type",
            "Click Type",
            "Type of click to perform",
            VariableType::String,
        )
        .set_options(
            flow_like::flow::pin::PinOptions::new()
                .set_valid_values(vec![
                    "Left".to_string(),
                    "Right".to_string(),
                    "Double".to_string(),
                ])
                .build(),
        )
        .set_default_value(Some(json!("Left")));

        node.add_input_pin(
            "offset_x",
            "Offset X",
            "X offset from center of matched template",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(0)));

        node.add_input_pin(
            "offset_y",
            "Offset Y",
            "Y offset from center of matched template",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(0)));

        node.add_input_pin(
            "fallback_x",
            "Fallback X",
            "X coordinate to click if template not found (use -1 to disable fallback)",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(-1)));

        node.add_input_pin(
            "fallback_y",
            "Fallback Y",
            "Y coordinate to click if template not found (use -1 to disable fallback)",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(-1)));

        node.add_output_pin("exec_out", "▶", "Continue", VariableType::Execution);
        node.add_output_pin(
            "exec_not_found",
            "Not Found",
            "Triggered if template not found",
            VariableType::Execution,
        );

        node.add_output_pin(
            "found",
            "Found",
            "Whether the template was found and clicked",
            VariableType::Boolean,
        );

        node.add_output_pin(
            "x",
            "X",
            "X coordinate where clicked",
            VariableType::Integer,
        );
        node.add_output_pin(
            "y",
            "Y",
            "Y coordinate where clicked",
            VariableType::Integer,
        );

        node
    }

    #[cfg(feature = "execute")]
    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        use rustautogui::{MatchMode, MouseClick};
        use std::io::Write;

        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("exec_not_found").await?;

        let session: AutomationSession = context.evaluate_pin("session").await?;
        let template: FlowPath = context.evaluate_pin("template").await?;
        let confidence: f64 = context.evaluate_pin("confidence").await?;
        let click_type: String = context.evaluate_pin("click_type").await?;
        let offset_x: i64 = context.evaluate_pin("offset_x").await?;
        let offset_y: i64 = context.evaluate_pin("offset_y").await?;

        let fallback_x: i64 = context.evaluate_pin("fallback_x").await?;
        let fallback_y: i64 = context.evaluate_pin("fallback_y").await?;

        // Download template image using FlowPath's caching mechanism
        let template_bytes = template.get(context, false).await?;

        // Write to a temporary file for rustautogui
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(format!("template_{}.png", flow_like_types::create_id()));
        let mut temp_file = std::fs::File::create(&temp_path)
            .map_err(|e| flow_like_types::anyhow!("Failed to create temp file: {}", e))?;
        temp_file.write_all(&template_bytes).map_err(|e| {
            let _ = std::fs::remove_file(&temp_path);
            flow_like_types::anyhow!("Failed to write template to temp file: {}", e)
        })?;

        // Guard ensures temp file is cleaned up even on early returns
        struct TempFileGuard(std::path::PathBuf);
        impl Drop for TempFileGuard {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
        let _guard = TempFileGuard(temp_path.clone());

        let autogui = session.get_autogui(context).await?;
        let mut gui = autogui.lock().await;

        let template_path_str = temp_path.to_string_lossy().to_string();
        gui.prepare_template_from_file(&template_path_str, None, MatchMode::Segmented)
            .map_err(|e| flow_like_types::anyhow!("Failed to prepare template: {}", e))?;

        let result = gui
            .find_image_on_screen(confidence as f32)
            .map_err(|e| flow_like_types::anyhow!("Failed to search screen: {}", e))?;

        let perform_click =
            |gui: &mut rustautogui::RustAutoGui, x: u32, y: u32| -> flow_like_types::Result<()> {
                gui.move_mouse_to_pos(x, y, 0.1)
                    .map_err(|e| flow_like_types::anyhow!("Failed to move mouse: {}", e))?;

                match click_type.as_str() {
                    "Right" => gui.click(MouseClick::RIGHT),
                    "Double" => gui.double_click(),
                    _ => gui.click(MouseClick::LEFT),
                }
                .map_err(|e| flow_like_types::anyhow!("Failed to click: {}", e))?;
                Ok(())
            };

        if let Some(matches) = result
            && let Some((x, y, _conf)) = matches.first()
        {
            let click_x = (*x as i64 + offset_x) as u32;
            let click_y = (*y as i64 + offset_y) as u32;

            perform_click(&mut gui, click_x, click_y)?;

            context.set_pin_value("found", json!(true)).await?;
            context.set_pin_value("x", json!(click_x as i64)).await?;
            context.set_pin_value("y", json!(click_y as i64)).await?;
            context.activate_exec_pin("exec_out").await?;
            return Ok(());
        }

        // Template not found - try fallback coordinates if provided
        if fallback_x >= 0 && fallback_y >= 0 {
            let click_x = fallback_x as u32;
            let click_y = fallback_y as u32;

            perform_click(&mut gui, click_x, click_y)?;

            context.set_pin_value("found", json!(false)).await?;
            context.set_pin_value("x", json!(fallback_x)).await?;
            context.set_pin_value("y", json!(fallback_y)).await?;
            context.activate_exec_pin("exec_out").await?;
            return Ok(());
        }

        context.set_pin_value("found", json!(false)).await?;
        context.set_pin_value("x", json!(0)).await?;
        context.set_pin_value("y", json!(0)).await?;
        context.activate_exec_pin("exec_not_found").await?;

        Ok(())
    }

    #[cfg(not(feature = "execute"))]
    async fn run(&self, _context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        Err(flow_like_types::anyhow!(
            "Vision automation requires the 'execute' feature"
        ))
    }
}
