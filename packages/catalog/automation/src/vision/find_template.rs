use crate::types::handles::AutomationSession;
use crate::types::templates::TemplateMatchResult;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json};

#[crate::register_node]
#[derive(Default)]
pub struct FindTemplateNode {}

impl FindTemplateNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for FindTemplateNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "vision_find_template",
            "Find Template",
            "Searches the screen for a template image and returns its location",
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
            "Automation session handle for screen operations",
            VariableType::Struct,
        )
        .set_schema::<AutomationSession>();

        node.add_input_pin(
            "template_path",
            "Template Path",
            "Path to the template image file",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "confidence",
            "Confidence",
            "Minimum match confidence (0.0-1.0)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.8)));

        node.add_input_pin(
            "match_mode",
            "Match Mode",
            "Algorithm for template matching",
            VariableType::String,
        )
        .set_options(
            flow_like::flow::pin::PinOptions::new()
                .set_valid_values(vec!["Segmented".to_string(), "FFT".to_string()])
                .build(),
        )
        .set_default_value(Some(json!("Segmented")));

        node.add_output_pin("exec_out", "▶", "Continue", VariableType::Execution);

        node.add_output_pin(
            "found",
            "Found",
            "Whether the template was found",
            VariableType::Boolean,
        );

        node.add_output_pin(
            "result",
            "Result",
            "Match result with location and confidence",
            VariableType::Struct,
        )
        .set_schema::<TemplateMatchResult>();

        node.add_output_pin(
            "x",
            "X",
            "X coordinate of match center",
            VariableType::Integer,
        );
        node.add_output_pin(
            "y",
            "Y",
            "Y coordinate of match center",
            VariableType::Integer,
        );

        node
    }

    #[cfg(feature = "execute")]
    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        use rustautogui::MatchMode;

        context.deactivate_exec_pin("exec_out").await?;

        let session: AutomationSession = context.evaluate_pin("session").await?;
        let template_path: String = context.evaluate_pin("template_path").await?;
        let confidence: f64 = context.evaluate_pin("confidence").await?;
        let match_mode_str: String = context.evaluate_pin("match_mode").await?;

        let autogui = session.get_autogui(context).await?;
        let mut gui = autogui.lock().await;

        let match_mode = match match_mode_str.as_str() {
            "FFT" => MatchMode::FFT,
            _ => MatchMode::Segmented,
        };

        gui.prepare_template_from_file(&template_path, None, match_mode)
            .map_err(|e| flow_like_types::anyhow!("Failed to prepare template: {}", e))?;

        let result = gui
            .find_image_on_screen(confidence as f32)
            .map_err(|e| flow_like_types::anyhow!("Failed to search screen: {}", e))?;

        let (found, x, y) = match result {
            Some(matches) if !matches.is_empty() => {
                let (mx, my, _conf) = matches[0];
                (true, mx, my)
            }
            _ => (false, 0, 0),
        };

        let match_result = TemplateMatchResult {
            found,
            x: x as i32,
            y: y as i32,
            confidence: if found { confidence } else { 0.0 },
            template_path: template_path.clone(),
        };

        context.set_pin_value("found", json!(found)).await?;
        context.set_pin_value("result", json!(match_result)).await?;
        context.set_pin_value("x", json!(x as i64)).await?;
        context.set_pin_value("y", json!(y as i64)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }

    #[cfg(not(feature = "execute"))]
    async fn run(&self, _context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        Err(flow_like_types::anyhow!(
            "Vision automation requires the 'execute' feature"
        ))
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct FindAllTemplatesNode {}

impl FindAllTemplatesNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for FindAllTemplatesNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "vision_find_all_templates",
            "Find All Templates",
            "Searches the screen for all occurrences of a template image",
            "Automation/Vision",
        );
        node.add_icon("/flow/icons/vision.svg");

        node.set_scores(
            flow_like::flow::node::NodeScores::new()
                .set_privacy(2)
                .set_security(4)
                .set_performance(5)
                .set_governance(5)
                .set_reliability(7)
                .set_cost(8)
                .build(),
        );
        node.set_only_offline(true);

        node.add_input_pin("exec_in", "▶", "Trigger", VariableType::Execution);

        node.add_input_pin(
            "session",
            "Session",
            "Automation session handle for screen operations",
            VariableType::Struct,
        )
        .set_schema::<AutomationSession>();

        node.add_input_pin(
            "template_path",
            "Template Path",
            "Path to the template image file",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "confidence",
            "Confidence",
            "Minimum match confidence (0.0-1.0)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.8)));

        node.add_input_pin(
            "max_results",
            "Max Results",
            "Maximum number of matches to return",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(10)));

        node.add_output_pin("exec_out", "▶", "Continue", VariableType::Execution);

        node.add_output_pin(
            "count",
            "Count",
            "Number of matches found",
            VariableType::Integer,
        );

        node.add_output_pin(
            "results",
            "Results",
            "Array of match results (as JSON)",
            VariableType::Generic,
        );

        node
    }

    #[cfg(feature = "execute")]
    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        use rustautogui::MatchMode;

        context.deactivate_exec_pin("exec_out").await?;

        let session: AutomationSession = context.evaluate_pin("session").await?;
        let template_path: String = context.evaluate_pin("template_path").await?;
        let confidence: f64 = context.evaluate_pin("confidence").await?;
        let max_results: i64 = context.evaluate_pin("max_results").await?;

        let autogui = session.get_autogui(context).await?;
        let mut gui = autogui.lock().await;

        gui.prepare_template_from_file(&template_path, None, MatchMode::Segmented)
            .map_err(|e| flow_like_types::anyhow!("Failed to prepare template: {}", e))?;

        let positions = gui
            .find_image_on_screen(confidence as f32)
            .map_err(|e| flow_like_types::anyhow!("Failed to search screen: {}", e))?;

        let results: Vec<TemplateMatchResult> = positions
            .unwrap_or_default()
            .into_iter()
            .take(max_results as usize)
            .map(|(x, y, conf)| TemplateMatchResult {
                found: true,
                x: x as i32,
                y: y as i32,
                confidence: conf as f64,
                template_path: template_path.clone(),
            })
            .collect();

        context
            .set_pin_value("count", json!(results.len() as i64))
            .await?;
        context.set_pin_value("results", json!(results)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }

    #[cfg(not(feature = "execute"))]
    async fn run(&self, _context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        Err(flow_like_types::anyhow!(
            "Vision automation requires the 'execute' feature"
        ))
    }
}
