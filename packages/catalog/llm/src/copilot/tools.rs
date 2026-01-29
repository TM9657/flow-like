/// # Copilot Tool Configuration Nodes
/// Nodes for configuring custom tools.
use super::CopilotToolConfig;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic, NodeScores},
    pin,
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{async_trait, json};

// =============================================================================
// Tool Config Node (Pure)
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct CopilotToolConfigNode {}

#[async_trait]
impl NodeLogic for CopilotToolConfigNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "copilot_tool_config",
            "Copilot Tool Config",
            "Creates a tool configuration for Copilot",
            "GitHub/Copilot/Tools",
        );
        node.add_icon("/flow/icons/tool.svg");

        node.set_scores(
            NodeScores::new()
                .set_privacy(10)
                .set_security(10)
                .set_performance(10)
                .set_governance(10)
                .set_reliability(10)
                .set_cost(10)
                .build(),
        );

        node.add_input_pin("name", "Name", "Tool name/identifier", VariableType::String);

        node.add_input_pin(
            "description",
            "Description",
            "Tool description for the AI",
            VariableType::String,
        );

        node.add_input_pin(
            "schema",
            "Schema",
            "JSON schema for tool parameters",
            VariableType::Generic,
        )
        .set_default_value(Some(json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })));

        node.add_output_pin("tool", "Tool", "Tool configuration", VariableType::Struct)
            .set_schema::<CopilotToolConfig>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let name: String = context.evaluate_pin("name").await?;
        let description: String = context.evaluate_pin("description").await?;
        let schema: flow_like_types::Value =
            context.evaluate_pin("schema").await.unwrap_or_else(|_| {
                json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                })
            });

        let tool = CopilotToolConfig {
            name,
            description,
            schema,
        };

        context.set_pin_value("tool", json::json!(tool)).await?;
        Ok(())
    }
}

// =============================================================================
// Combine Tools Node (Pure)
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct CopilotCombineToolsNode {}

#[async_trait]
impl NodeLogic for CopilotCombineToolsNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "copilot_combine_tools",
            "Copilot Combine Tools",
            "Combines multiple tools into an array",
            "GitHub/Copilot/Tools",
        );
        node.add_icon("/flow/icons/merge.svg");

        node.set_scores(
            NodeScores::new()
                .set_privacy(10)
                .set_security(10)
                .set_performance(10)
                .set_governance(10)
                .set_reliability(10)
                .set_cost(10)
                .build(),
        );

        node.add_input_pin(
            "tool_a",
            "Tool A",
            "First tool (optional)",
            VariableType::Struct,
        )
        .set_schema::<CopilotToolConfig>();

        node.add_input_pin(
            "tool_b",
            "Tool B",
            "Second tool (optional)",
            VariableType::Struct,
        )
        .set_schema::<CopilotToolConfig>();

        node.add_input_pin(
            "tool_c",
            "Tool C",
            "Third tool (optional)",
            VariableType::Struct,
        )
        .set_schema::<CopilotToolConfig>();

        node.add_input_pin(
            "existing_tools",
            "Existing Tools",
            "Existing tools array to extend",
            VariableType::Struct,
        )
        .set_value_type(pin::ValueType::Array);

        node.add_output_pin(
            "tools",
            "Tools",
            "Combined tools array",
            VariableType::Struct,
        )
        .set_value_type(pin::ValueType::Array);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let mut tools: Vec<CopilotToolConfig> = context
            .evaluate_pin("existing_tools")
            .await
            .unwrap_or_default();

        if let Ok(tool_a) = context.evaluate_pin::<CopilotToolConfig>("tool_a").await {
            tools.push(tool_a);
        }

        if let Ok(tool_b) = context.evaluate_pin::<CopilotToolConfig>("tool_b").await {
            tools.push(tool_b);
        }

        if let Ok(tool_c) = context.evaluate_pin::<CopilotToolConfig>("tool_c").await {
            tools.push(tool_c);
        }

        context.set_pin_value("tools", json::json!(tools)).await?;
        Ok(())
    }
}
