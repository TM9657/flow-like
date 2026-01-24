use crate::types::handles::AutomationSession;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json, rand};

#[crate::register_node]
#[derive(Default)]
pub struct ComputerMouseMoveNode {}

impl ComputerMouseMoveNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for ComputerMouseMoveNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "computer_mouse_move",
            "Mouse Move",
            "Moves the mouse cursor to the specified screen coordinates",
            "Automation/Computer/Mouse",
        );
        node.add_icon("/flow/icons/computer.svg");

        node.set_scores(
            flow_like::flow::node::NodeScores::new()
                .set_privacy(2)
                .set_security(3)
                .set_performance(8)
                .set_governance(5)
                .set_reliability(8)
                .set_cost(10)
                .build(),
        );
        node.set_only_offline(true);

        node.add_input_pin("exec_in", "▶", "Trigger", VariableType::Execution);

        node.add_input_pin(
            "session",
            "Session",
            "Computer session handle",
            VariableType::Struct,
        )
        .set_schema::<AutomationSession>();

        node.add_input_pin(
            "x",
            "X",
            "X coordinate (horizontal position)",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(0)));

        node.add_input_pin(
            "y",
            "Y",
            "Y coordinate (vertical position)",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(0)));

        node.add_output_pin("exec_out", "▶", "Continue", VariableType::Execution);

        node.add_output_pin(
            "session_out",
            "Session",
            "Computer session handle (pass-through)",
            VariableType::Struct,
        )
        .set_schema::<AutomationSession>();

        node
    }

    #[cfg(feature = "execute")]
    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        use enigo::{Coordinate, Mouse};

        context.deactivate_exec_pin("exec_out").await?;

        let session: AutomationSession = context.evaluate_pin("session").await?;
        let x: i64 = context.evaluate_pin("x").await?;
        let y: i64 = context.evaluate_pin("y").await?;

        {
            let mut enigo = session.create_enigo()?;
            enigo
                .move_mouse(x as i32, y as i32, Coordinate::Abs)
                .map_err(|e| flow_like_types::anyhow!("Failed to move mouse: {}", e))?;
        }

        context.set_pin_value("session_out", json!(session)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }

    #[cfg(not(feature = "execute"))]
    async fn run(&self, _context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        Err(flow_like_types::anyhow!(
            "Computer automation requires the 'execute' feature"
        ))
    }
}

/// Generates a point on a cubic Bezier curve
#[cfg(feature = "execute")]
fn bezier_point(
    p0: (f64, f64),
    p1: (f64, f64),
    p2: (f64, f64),
    p3: (f64, f64),
    t: f64,
) -> (f64, f64) {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;

    (
        mt3 * p0.0 + 3.0 * mt2 * t * p1.0 + 3.0 * mt * t2 * p2.0 + t3 * p3.0,
        mt3 * p0.1 + 3.0 * mt2 * t * p1.1 + 3.0 * mt * t2 * p2.1 + t3 * p3.1,
    )
}

/// Easing function for natural acceleration/deceleration
#[cfg(feature = "execute")]
fn ease_in_out_quad(t: f64) -> f64 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct ComputerNaturalMouseMoveNode {}

impl ComputerNaturalMouseMoveNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for ComputerNaturalMouseMoveNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "computer_natural_mouse_move",
            "Natural Mouse Move",
            "Moves the mouse cursor naturally using curved paths with variable speed to avoid bot detection",
            "Automation/Computer/Mouse",
        );
        node.add_icon("/flow/icons/computer.svg");

        node.set_scores(
            flow_like::flow::node::NodeScores::new()
                .set_privacy(2)
                .set_security(3)
                .set_performance(6)
                .set_governance(5)
                .set_reliability(8)
                .set_cost(10)
                .build(),
        );
        node.set_only_offline(true);

        node.add_input_pin("exec_in", "▶", "Trigger", VariableType::Execution);

        node.add_input_pin(
            "session",
            "Session",
            "Computer session handle",
            VariableType::Struct,
        )
        .set_schema::<AutomationSession>();

        node.add_input_pin("x", "X", "Target X coordinate", VariableType::Integer)
            .set_default_value(Some(json!(0)));

        node.add_input_pin("y", "Y", "Target Y coordinate", VariableType::Integer)
            .set_default_value(Some(json!(0)));

        node.add_input_pin(
            "duration_ms",
            "Duration (ms)",
            "Approximate duration of the movement in milliseconds",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(300)));

        node.add_input_pin(
            "curve_intensity",
            "Curve Intensity",
            "How curved the path is (0.0 = straight, 1.0 = very curved)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.3)));

        node.add_input_pin(
            "overshoot",
            "Overshoot",
            "Whether to slightly overshoot and correct (more human-like)",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_output_pin("exec_out", "▶", "Continue", VariableType::Execution);

        node.add_output_pin(
            "session_out",
            "Session",
            "Computer session handle (pass-through)",
            VariableType::Struct,
        )
        .set_schema::<AutomationSession>();

        node
    }

    #[cfg(feature = "execute")]
    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        use enigo::{Coordinate, Mouse};

        context.deactivate_exec_pin("exec_out").await?;

        let session: AutomationSession = context.evaluate_pin("session").await?;
        let target_x: i64 = context.evaluate_pin("x").await?;
        let target_y: i64 = context.evaluate_pin("y").await?;
        let duration_ms: i64 = context.evaluate_pin("duration_ms").await.unwrap_or(300);
        let curve_intensity: f64 = context.evaluate_pin("curve_intensity").await.unwrap_or(0.3);
        let overshoot: bool = context.evaluate_pin("overshoot").await.unwrap_or(false);

        // All synchronous work in a block that doesn't span await points
        {
            use rand::Rng;
            let mut rng = rand::rng();

            // Get current mouse position
            let (start_x, start_y) = {
                let enigo = session.create_enigo()?;
                enigo.location().unwrap_or((0, 0))
            };

            let start = (start_x as f64, start_y as f64);
            let end = (target_x as f64, target_y as f64);

            // Calculate distance for step count
            let dx = end.0 - start.0;
            let dy = end.1 - start.1;
            let distance = (dx * dx + dy * dy).sqrt();

            // More steps for longer distances, minimum 10 steps
            let steps = ((distance / 10.0) as i32).max(10).min(100);
            let step_delay_ms = (duration_ms as f64 / steps as f64) as u64;

            // Generate random control points for bezier curve
            let curve_offset = distance * curve_intensity;
            let perpendicular = if dx.abs() > 0.001 {
                (-dy / dx, 1.0)
            } else {
                (1.0, 0.0)
            };
            let perp_len =
                (perpendicular.0 * perpendicular.0 + perpendicular.1 * perpendicular.1).sqrt();
            let perp_norm = (perpendicular.0 / perp_len, perpendicular.1 / perp_len);

            // Randomize control point offsets
            let offset1 = rng.random_range(-curve_offset..curve_offset);
            let offset2 = rng.random_range(-curve_offset..curve_offset);

            let ctrl1 = (
                start.0 + dx * 0.3 + perp_norm.0 * offset1,
                start.1 + dy * 0.3 + perp_norm.1 * offset1,
            );
            let ctrl2 = (
                start.0 + dx * 0.7 + perp_norm.0 * offset2,
                start.1 + dy * 0.7 + perp_norm.1 * offset2,
            );

            // If overshoot enabled, extend the target slightly
            let overshoot_amount = if overshoot && distance > 50.0 {
                let overshoot_dist = rng.random_range(5.0..15.0);
                let angle = dy.atan2(dx);
                (overshoot_dist * angle.cos(), overshoot_dist * angle.sin())
            } else {
                (0.0, 0.0)
            };

            let overshoot_end = (end.0 + overshoot_amount.0, end.1 + overshoot_amount.1);

            // Move along the bezier curve with easing
            let mut enigo = session.create_enigo()?;

            for i in 1..=steps {
                let t = ease_in_out_quad(i as f64 / steps as f64);
                let (x, y) = bezier_point(start, ctrl1, ctrl2, overshoot_end, t);

                // Add tiny random jitter for more natural movement
                let jitter_x = rng.random_range(-1.0..1.0);
                let jitter_y = rng.random_range(-1.0..1.0);

                enigo
                    .move_mouse(
                        (x + jitter_x) as i32,
                        (y + jitter_y) as i32,
                        Coordinate::Abs,
                    )
                    .map_err(|e| flow_like_types::anyhow!("Failed to move mouse: {}", e))?;

                // Variable delay for more natural timing
                let delay_variance = rng.random_range(0.8..1.2);
                std::thread::sleep(std::time::Duration::from_millis(
                    (step_delay_ms as f64 * delay_variance) as u64,
                ));
            }

            // If we overshot, correct to the actual target
            if overshoot && (overshoot_amount.0.abs() > 0.1 || overshoot_amount.1.abs() > 0.1) {
                std::thread::sleep(std::time::Duration::from_millis(rng.random_range(30..80)));

                // Small correction movement
                let correction_steps = 5;
                for i in 1..=correction_steps {
                    let t = i as f64 / correction_steps as f64;
                    let x = overshoot_end.0 + (end.0 - overshoot_end.0) * t;
                    let y = overshoot_end.1 + (end.1 - overshoot_end.1) * t;

                    enigo
                        .move_mouse(x as i32, y as i32, Coordinate::Abs)
                        .map_err(|e| flow_like_types::anyhow!("Failed to move mouse: {}", e))?;

                    std::thread::sleep(std::time::Duration::from_millis(rng.random_range(10..20)));
                }
            }

            // Ensure we end exactly at target
            enigo
                .move_mouse(target_x as i32, target_y as i32, Coordinate::Abs)
                .map_err(|e| flow_like_types::anyhow!("Failed to move mouse: {}", e))?;
        }

        context.set_pin_value("session_out", json!(session)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }

    #[cfg(not(feature = "execute"))]
    async fn run(&self, _context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        Err(flow_like_types::anyhow!(
            "Computer automation requires the 'execute' feature"
        ))
    }
}
/// Performs natural mouse movement using Bezier curves
#[cfg(feature = "execute")]
fn perform_natural_move(
    enigo: &mut enigo::Enigo,
    start: (i32, i32),
    end: (i32, i32),
    duration_ms: u64,
    overshoot: bool,
) -> flow_like_types::Result<()> {
    use enigo::{Coordinate, Mouse};
    use rand::Rng;

    let mut rng = rand::rng();

    let start_f = (start.0 as f64, start.1 as f64);
    let end_f = (end.0 as f64, end.1 as f64);

    let dx = end_f.0 - start_f.0;
    let dy = end_f.1 - start_f.1;
    let distance = (dx * dx + dy * dy).sqrt();

    // Skip natural movement for very short distances
    if distance < 10.0 {
        enigo
            .move_mouse(end.0, end.1, Coordinate::Abs)
            .map_err(|e| flow_like_types::anyhow!("Failed to move mouse: {}", e))?;
        return Ok(());
    }

    let steps = ((distance / 10.0) as i32).max(10).min(100);
    let step_delay_ms = (duration_ms as f64 / steps as f64) as u64;

    // Generate random control points for bezier curve
    let curve_intensity = 0.3;
    let curve_offset = distance * curve_intensity;
    let perpendicular = if dx.abs() > 0.001 {
        (-dy / dx, 1.0)
    } else {
        (1.0, 0.0)
    };
    let perp_len = (perpendicular.0 * perpendicular.0 + perpendicular.1 * perpendicular.1).sqrt();
    let perp_norm = (perpendicular.0 / perp_len, perpendicular.1 / perp_len);

    let offset1 = rng.random_range(-curve_offset..curve_offset);
    let offset2 = rng.random_range(-curve_offset..curve_offset);

    let ctrl1 = (
        start_f.0 + dx * 0.3 + perp_norm.0 * offset1,
        start_f.1 + dy * 0.3 + perp_norm.1 * offset1,
    );
    let ctrl2 = (
        start_f.0 + dx * 0.7 + perp_norm.0 * offset2,
        start_f.1 + dy * 0.7 + perp_norm.1 * offset2,
    );

    // Overshoot if enabled
    let overshoot_amount = if overshoot && distance > 50.0 {
        let overshoot_dist = rng.random_range(5.0..15.0);
        let angle = dy.atan2(dx);
        (overshoot_dist * angle.cos(), overshoot_dist * angle.sin())
    } else {
        (0.0, 0.0)
    };

    let overshoot_end = (end_f.0 + overshoot_amount.0, end_f.1 + overshoot_amount.1);

    // Move along the bezier curve
    for i in 1..=steps {
        let t = ease_in_out_quad(i as f64 / steps as f64);
        let (x, y) = bezier_point(start_f, ctrl1, ctrl2, overshoot_end, t);

        let jitter_x = rng.random_range(-1.0..1.0);
        let jitter_y = rng.random_range(-1.0..1.0);

        enigo
            .move_mouse(
                (x + jitter_x) as i32,
                (y + jitter_y) as i32,
                Coordinate::Abs,
            )
            .map_err(|e| flow_like_types::anyhow!("Failed to move mouse: {}", e))?;

        let delay_variance = rng.random_range(0.8..1.2);
        std::thread::sleep(std::time::Duration::from_millis(
            (step_delay_ms as f64 * delay_variance) as u64,
        ));
    }

    // Correct from overshoot
    if overshoot && (overshoot_amount.0.abs() > 0.1 || overshoot_amount.1.abs() > 0.1) {
        std::thread::sleep(std::time::Duration::from_millis(rng.random_range(30..80)));

        let correction_steps = 5;
        for i in 1..=correction_steps {
            let t = i as f64 / correction_steps as f64;
            let x = overshoot_end.0 + (end_f.0 - overshoot_end.0) * t;
            let y = overshoot_end.1 + (end_f.1 - overshoot_end.1) * t;

            enigo
                .move_mouse(x as i32, y as i32, Coordinate::Abs)
                .map_err(|e| flow_like_types::anyhow!("Failed to move mouse: {}", e))?;

            std::thread::sleep(std::time::Duration::from_millis(rng.random_range(10..20)));
        }
    }

    // Ensure exact end position
    enigo
        .move_mouse(end.0, end.1, Coordinate::Abs)
        .map_err(|e| flow_like_types::anyhow!("Failed to move mouse: {}", e))?;

    Ok(())
}

#[crate::register_node]
#[derive(Default)]
pub struct ComputerMouseClickNode {}

impl ComputerMouseClickNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for ComputerMouseClickNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "computer_mouse_click",
            "Mouse Click",
            "Clicks the mouse at the specified coordinates",
            "Automation/Computer/Mouse",
        );
        node.add_icon("/flow/icons/computer.svg");

        node.set_scores(
            flow_like::flow::node::NodeScores::new()
                .set_privacy(2)
                .set_security(3)
                .set_performance(8)
                .set_governance(5)
                .set_reliability(8)
                .set_cost(10)
                .build(),
        );
        node.set_only_offline(true);

        node.add_input_pin("exec_in", "▶", "Trigger", VariableType::Execution);

        node.add_input_pin(
            "session",
            "Session",
            "Computer session handle",
            VariableType::Struct,
        )
        .set_schema::<AutomationSession>();

        node.add_input_pin(
            "x",
            "X",
            "X coordinate (horizontal position)",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(0)));

        node.add_input_pin(
            "y",
            "Y",
            "Y coordinate (vertical position)",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(0)));

        node.add_input_pin(
            "button",
            "Button",
            "Mouse button to click",
            VariableType::String,
        )
        .set_options(
            flow_like::flow::pin::PinOptions::new()
                .set_valid_values(vec![
                    "left".to_string(),
                    "right".to_string(),
                    "middle".to_string(),
                ])
                .build(),
        )
        .set_default_value(Some(json!("left")));

        node.add_input_pin(
            "use_template_matching",
            "Use Template Matching",
            "If enabled, use template matching to find the click target from a recorded screenshot",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_input_pin(
            "screenshot_ref",
            "Screenshot Ref",
            "Reference to recorded screenshot artifact for template matching",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "confidence",
            "Confidence",
            "Minimum confidence threshold for template matching (0.0-1.0)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.8)));

        node.add_input_pin(
            "natural_move",
            "Natural Movement",
            "Use curved, human-like mouse movement to avoid bot detection",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_input_pin(
            "move_duration_ms",
            "Move Duration (ms)",
            "Duration of natural mouse movement in milliseconds",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(200)));

        node.add_output_pin("exec_out", "▶", "Continue", VariableType::Execution);

        node.add_output_pin(
            "session_out",
            "Session",
            "Computer session handle (pass-through)",
            VariableType::Struct,
        )
        .set_schema::<AutomationSession>();

        node
    }

    #[cfg(feature = "execute")]
    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        use enigo::{Button, Coordinate, Mouse};
        use rustautogui::MatchMode;

        context.deactivate_exec_pin("exec_out").await?;

        let session: AutomationSession = context.evaluate_pin("session").await?;
        let mut x: i64 = context.evaluate_pin("x").await?;
        let mut y: i64 = context.evaluate_pin("y").await?;
        let button_str: String = context.evaluate_pin("button").await?;
        let use_template_matching: bool = context
            .evaluate_pin("use_template_matching")
            .await
            .unwrap_or(false);
        let screenshot_ref: String = context
            .evaluate_pin("screenshot_ref")
            .await
            .unwrap_or_default();
        let confidence: f64 = context.evaluate_pin("confidence").await.unwrap_or(0.8);
        let natural_move: bool = context.evaluate_pin("natural_move").await.unwrap_or(false);
        let move_duration_ms: i64 = context
            .evaluate_pin("move_duration_ms")
            .await
            .unwrap_or(200);

        if use_template_matching && !screenshot_ref.is_empty() {
            // Resolve the screenshot path relative to storage directory
            if let Some(ref context_cache) = context.execution_cache {
                let storage_path = context_cache.get_storage(false)?;
                let full_path = storage_path.child(screenshot_ref.clone());
                let path_str = full_path.to_string();

                let autogui = session.get_autogui(context).await?;
                let mut gui = autogui.lock().await;

                if let Err(e) =
                    gui.prepare_template_from_file(&path_str, None, MatchMode::Segmented)
                {
                    println!(
                        "[MouseClick] Template preparation failed: {}, falling back to coordinates",
                        e
                    );
                } else if let Ok(Some(matches)) = gui.find_image_on_screen(confidence as f32) {
                    if !matches.is_empty() {
                        let (mx, my, conf) = matches[0];
                        println!(
                            "[MouseClick] Template matched at ({}, {}) with confidence {}",
                            mx, my, conf
                        );
                        x = mx as i64;
                        y = my as i64;
                    }
                }
            }
        }

        let button = match button_str.as_str() {
            "right" => Button::Right,
            "middle" => Button::Middle,
            _ => Button::Left,
        };

        {
            let mut enigo = session.create_enigo()?;

            if natural_move {
                use rand::Rng;
                // Get current position for natural movement
                let start = enigo.location().unwrap_or((0, 0));
                let overshoot = rand::rng().random_bool(0.3); // 30% chance of overshoot
                perform_natural_move(
                    &mut enigo,
                    start,
                    (x as i32, y as i32),
                    move_duration_ms as u64,
                    overshoot,
                )?;
            } else {
                enigo
                    .move_mouse(x as i32, y as i32, Coordinate::Abs)
                    .map_err(|e| flow_like_types::anyhow!("Failed to move mouse: {}", e))?;
            }

            std::thread::sleep(std::time::Duration::from_millis(session.click_delay_ms));

            enigo
                .button(button, enigo::Direction::Click)
                .map_err(|e| flow_like_types::anyhow!("Failed to click mouse: {}", e))?;
        }

        context.set_pin_value("session_out", json!(session)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }

    #[cfg(not(feature = "execute"))]
    async fn run(&self, _context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        Err(flow_like_types::anyhow!(
            "Computer automation requires the 'execute' feature"
        ))
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct ComputerMouseDoubleClickNode {}

impl ComputerMouseDoubleClickNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for ComputerMouseDoubleClickNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "computer_mouse_double_click",
            "Mouse Double Click",
            "Double-clicks the mouse at the specified coordinates",
            "Automation/Computer/Mouse",
        );
        node.add_icon("/flow/icons/computer.svg");

        node.set_scores(
            flow_like::flow::node::NodeScores::new()
                .set_privacy(2)
                .set_security(3)
                .set_performance(8)
                .set_governance(5)
                .set_reliability(8)
                .set_cost(10)
                .build(),
        );
        node.set_only_offline(true);

        node.add_input_pin("exec_in", "▶", "Trigger", VariableType::Execution);

        node.add_input_pin(
            "session",
            "Session",
            "Computer session handle",
            VariableType::Struct,
        )
        .set_schema::<AutomationSession>();

        node.add_input_pin("x", "X", "X coordinate", VariableType::Integer)
            .set_default_value(Some(json!(0)));

        node.add_input_pin("y", "Y", "Y coordinate", VariableType::Integer)
            .set_default_value(Some(json!(0)));

        node.add_input_pin(
            "natural_move",
            "Natural Movement",
            "Use curved, human-like mouse movement to avoid bot detection",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_input_pin(
            "move_duration_ms",
            "Move Duration (ms)",
            "Duration of natural mouse movement in milliseconds",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(200)));

        node.add_output_pin("exec_out", "▶", "Continue", VariableType::Execution);

        node.add_output_pin(
            "session_out",
            "Session",
            "Computer session handle (pass-through)",
            VariableType::Struct,
        )
        .set_schema::<AutomationSession>();

        node
    }

    #[cfg(feature = "execute")]
    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        use enigo::{Button, Coordinate, Mouse};

        context.deactivate_exec_pin("exec_out").await?;

        let session: AutomationSession = context.evaluate_pin("session").await?;
        let x: i64 = context.evaluate_pin("x").await?;
        let y: i64 = context.evaluate_pin("y").await?;
        let natural_move: bool = context.evaluate_pin("natural_move").await.unwrap_or(false);
        let move_duration_ms: i64 = context
            .evaluate_pin("move_duration_ms")
            .await
            .unwrap_or(200);

        {
            let mut enigo = session.create_enigo()?;

            if natural_move {
                use rand::Rng;
                let start = enigo.location().unwrap_or((0, 0));
                let overshoot = rand::rng().random_bool(0.3);
                perform_natural_move(
                    &mut enigo,
                    start,
                    (x as i32, y as i32),
                    move_duration_ms as u64,
                    overshoot,
                )?;
            } else {
                enigo
                    .move_mouse(x as i32, y as i32, Coordinate::Abs)
                    .map_err(|e| flow_like_types::anyhow!("Failed to move mouse: {}", e))?;
            }

            std::thread::sleep(std::time::Duration::from_millis(session.click_delay_ms));

            enigo
                .button(Button::Left, enigo::Direction::Click)
                .map_err(|e| flow_like_types::anyhow!("Failed to click mouse: {}", e))?;
        }

        // Small delay between clicks (50-100ms is typical for double-click)
        flow_like_types::tokio::time::sleep(std::time::Duration::from_millis(80)).await;

        {
            let mut enigo = session.create_enigo()?;
            // Second click - no need to move again, cursor is already there
            enigo
                .button(Button::Left, enigo::Direction::Click)
                .map_err(|e| flow_like_types::anyhow!("Failed to double-click mouse: {}", e))?;
        }

        context.set_pin_value("session_out", json!(session)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }

    #[cfg(not(feature = "execute"))]
    async fn run(&self, _context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        Err(flow_like_types::anyhow!(
            "Computer automation requires the 'execute' feature"
        ))
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct ComputerMouseDragNode {}

impl ComputerMouseDragNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for ComputerMouseDragNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "computer_mouse_drag",
            "Mouse Drag",
            "Drags the mouse from one position to another",
            "Automation/Computer/Mouse",
        );
        node.add_icon("/flow/icons/computer.svg");

        node.set_scores(
            flow_like::flow::node::NodeScores::new()
                .set_privacy(2)
                .set_security(3)
                .set_performance(7)
                .set_governance(5)
                .set_reliability(8)
                .set_cost(10)
                .build(),
        );
        node.set_only_offline(true);

        node.add_input_pin("exec_in", "▶", "Trigger", VariableType::Execution);

        node.add_input_pin(
            "session",
            "Session",
            "Computer session handle",
            VariableType::Struct,
        )
        .set_schema::<AutomationSession>();

        node.add_input_pin(
            "from_x",
            "From X",
            "Starting X coordinate",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(0)));

        node.add_input_pin(
            "from_y",
            "From Y",
            "Starting Y coordinate",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(0)));

        node.add_input_pin("to_x", "To X", "Ending X coordinate", VariableType::Integer)
            .set_default_value(Some(json!(0)));

        node.add_input_pin("to_y", "To Y", "Ending Y coordinate", VariableType::Integer)
            .set_default_value(Some(json!(0)));

        node.add_input_pin(
            "button",
            "Button",
            "Mouse button to use for dragging",
            VariableType::String,
        )
        .set_options(
            flow_like::flow::pin::PinOptions::new()
                .set_valid_values(vec!["left".to_string(), "right".to_string()])
                .build(),
        )
        .set_default_value(Some(json!("left")));

        node.add_output_pin("exec_out", "▶", "Continue", VariableType::Execution);

        node.add_output_pin(
            "session_out",
            "Session",
            "Computer session handle (pass-through)",
            VariableType::Struct,
        )
        .set_schema::<AutomationSession>();

        node
    }

    #[cfg(feature = "execute")]
    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        use enigo::{Button, Coordinate, Mouse};

        context.deactivate_exec_pin("exec_out").await?;

        let session: AutomationSession = context.evaluate_pin("session").await?;
        let from_x: i64 = context.evaluate_pin("from_x").await?;
        let from_y: i64 = context.evaluate_pin("from_y").await?;
        let to_x: i64 = context.evaluate_pin("to_x").await?;
        let to_y: i64 = context.evaluate_pin("to_y").await?;
        let button_str: String = context.evaluate_pin("button").await?;

        let button = match button_str.as_str() {
            "right" => Button::Right,
            _ => Button::Left,
        };

        {
            let mut enigo = session.create_enigo()?;
            enigo
                .move_mouse(from_x as i32, from_y as i32, Coordinate::Abs)
                .map_err(|e| flow_like_types::anyhow!("Failed to move mouse: {}", e))?;
            enigo
                .button(button, enigo::Direction::Press)
                .map_err(|e| flow_like_types::anyhow!("Failed to press mouse: {}", e))?;
            enigo
                .move_mouse(to_x as i32, to_y as i32, Coordinate::Abs)
                .map_err(|e| flow_like_types::anyhow!("Failed to move mouse: {}", e))?;
            enigo
                .button(button, enigo::Direction::Release)
                .map_err(|e| flow_like_types::anyhow!("Failed to release mouse: {}", e))?;
        }

        context.set_pin_value("session_out", json!(session)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }

    #[cfg(not(feature = "execute"))]
    async fn run(&self, _context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        Err(flow_like_types::anyhow!(
            "Computer automation requires the 'execute' feature"
        ))
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct ComputerScrollNode {}

impl ComputerScrollNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for ComputerScrollNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "computer_scroll",
            "Scroll",
            "Scrolls the mouse wheel",
            "Automation/Computer/Mouse",
        );
        node.add_icon("/flow/icons/computer.svg");

        node.set_scores(
            flow_like::flow::node::NodeScores::new()
                .set_privacy(2)
                .set_security(4)
                .set_performance(8)
                .set_governance(5)
                .set_reliability(8)
                .set_cost(10)
                .build(),
        );
        node.set_only_offline(true);

        node.add_input_pin("exec_in", "▶", "Trigger", VariableType::Execution);

        node.add_input_pin(
            "session",
            "Session",
            "Computer session handle",
            VariableType::Struct,
        )
        .set_schema::<AutomationSession>();

        node.add_input_pin(
            "dx",
            "Delta X",
            "Horizontal scroll amount (positive = right)",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(0)));

        node.add_input_pin(
            "dy",
            "Delta Y",
            "Vertical scroll amount (positive = down)",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(3)));

        node.add_output_pin("exec_out", "▶", "Continue", VariableType::Execution);

        node.add_output_pin(
            "session_out",
            "Session",
            "Computer session handle (pass-through)",
            VariableType::Struct,
        )
        .set_schema::<AutomationSession>();

        node
    }

    #[cfg(feature = "execute")]
    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        use enigo::{Axis, Mouse};

        context.deactivate_exec_pin("exec_out").await?;

        let session: AutomationSession = context.evaluate_pin("session").await?;
        let dx: i64 = context.evaluate_pin("dx").await?;
        let dy: i64 = context.evaluate_pin("dy").await?;

        {
            let mut enigo = session.create_enigo()?;
            if dy != 0 {
                enigo
                    .scroll(dy as i32, Axis::Vertical)
                    .map_err(|e| flow_like_types::anyhow!("Failed to scroll vertically: {}", e))?;
            }
            if dx != 0 {
                enigo.scroll(dx as i32, Axis::Horizontal).map_err(|e| {
                    flow_like_types::anyhow!("Failed to scroll horizontally: {}", e)
                })?;
            }
        }

        context.set_pin_value("session_out", json!(session)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }

    #[cfg(not(feature = "execute"))]
    async fn run(&self, _context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        Err(flow_like_types::anyhow!(
            "Computer automation requires the 'execute' feature"
        ))
    }
}
