use super::Agent;
#[cfg(feature = "execute")]
use super::{
    EditorMcpConfig,
    editor_mcp::{device_value, element_reference},
};
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::{PinOptions, ValueType},
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json};
#[cfg(feature = "execute")]
use std::{collections::HashSet, time::Duration};

#[crate::register_node]
#[derive(Default)]
pub struct RegisterEditorMcpToolsNode;

#[async_trait]
impl NodeLogic for RegisterEditorMcpToolsNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "agent_register_editor_mcp_tools",
            "Register Editor MCP Tools",
            "Gives the agent the tools exposed by the document open in a page's file editor. Reopening the document requires registering its new session.",
            "AI/Agents/Builder",
        );
        node.set_flowscript_name("agent", "registerEditorMcpTools");
        node.set_receiver("agent_in");
        node.add_icon("/flow/icons/bot-invoke.svg");
        node.add_input_pin(
            "agent_in",
            "Agent",
            "Agent to receive the editor tools",
            VariableType::Struct,
        )
        .set_schema::<Agent>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());
        node.add_input_pin(
            "element_ref",
            "Editor Element",
            "Connect Element from Get Element, select the file editor, or use its full surface/component reference",
            VariableType::Generic,
        )
        .set_options(
            PinOptions::new()
                .set_enforce_generic_value_type(true)
                .set_enforce_schema(false)
                .build(),
        );
        node.add_input_pin("tools", "Tools", "Exact MCP tool names to expose. Leave empty to expose all tools provided by this editor.", VariableType::String)
            .set_value_type(ValueType::Array)
            .set_default_value(Some(json!([])));
        node.add_output_pin(
            "agent_out",
            "Agent",
            "Agent connected to this live editor document",
            VariableType::Struct,
        )
        .set_schema::<Agent>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());
        node
    }

    #[cfg(feature = "execute")]
    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let mut agent: Agent = context.evaluate_pin("agent_in").await?;
        let element = context.evaluate_pin("element_ref").await?;
        let element_ref = element_reference(&element)?;
        let tools: Vec<String> = context.evaluate_pin("tools").await?;
        let tool_filter = (!tools.is_empty()).then(|| tools.into_iter().collect::<HashSet<_>>());
        let session = device_value(
            context
                .request_device(
                    "craft.session",
                    json!({"elementRef": element_ref}),
                    Duration::from_secs(15),
                )
                .await?,
        )?;
        let config = EditorMcpConfig::from_session(element_ref, session, tool_filter)?;
        // Re-registering this element selects its current document instead of retaining a stale peer.
        agent
            .editor_mcp_servers
            .retain(|server| server.element_ref != config.element_ref);
        agent.editor_mcp_servers.push(config);
        context.set_pin_value("agent_out", json!(agent)).await?;
        Ok(())
    }

    #[cfg(not(feature = "execute"))]
    async fn run(&self, _context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        Err(flow_like_types::anyhow!(
            "LLM processing requires the 'execute' feature"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_registration_has_no_remote_endpoint_or_process_inputs() {
        let node = RegisterEditorMcpToolsNode.get_node();
        assert_eq!(node.name, "agent_register_editor_mcp_tools");
        assert!(!node.only_offline);
        assert_eq!(node.flowscript_namespace(), "agent");
        assert_eq!(node.flowscript_alias(), "registerEditorMcpTools");
        assert_eq!(node.flowscript_receiver().as_deref(), Some("agent_in"));
        for name in ["agent_in", "element_ref", "tools", "agent_out"] {
            assert!(node.get_pin_by_name(name).is_some());
        }
        assert!(node.get_pin_by_name("uri").is_none());
        assert!(node.get_pin_by_name("command").is_none());
        let editor = node.get_pin_by_name("element_ref").unwrap();
        assert_eq!(editor.data_type, VariableType::Generic);
        assert_eq!(editor.value_type, ValueType::Normal);
        assert_eq!(
            editor.options.as_ref().unwrap().enforce_generic_value_type,
            Some(true)
        );
        for name in ["agent_in", "agent_out"] {
            let pin = node.get_pin_by_name(name).unwrap();
            assert_eq!(pin.options.as_ref().unwrap().enforce_schema, Some(true));
            assert!(pin.schema.is_some());
        }
        assert_eq!(
            node.get_pin_by_name("tools").unwrap().value_type,
            ValueType::Array
        );
    }
}
