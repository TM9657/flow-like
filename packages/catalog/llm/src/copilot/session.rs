#[cfg(feature = "execute")]
use super::CachedCopilotSession;
/// # Copilot Session Nodes
/// Nodes for creating and managing Copilot sessions.
use super::{
    COPILOT_SESSION_PREFIX, CopilotClientHandle, CopilotSessionConfig, CopilotSessionHandle,
};
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic, NodeScores},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{async_trait, json};

// =============================================================================
// Session Config Node (Pure)
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct CopilotSessionConfigNode {}

#[async_trait]
impl NodeLogic for CopilotSessionConfigNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "copilot_session_config",
            "Copilot Session Config",
            "Creates a session configuration object",
            "GitHub/Copilot/Session",
        );
        node.add_icon("/flow/icons/settings.svg");

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
            "model",
            "Model",
            "Optional model ID to use",
            VariableType::String,
        )
        .set_default_value(Some(json::json!("")));

        node.add_input_pin(
            "streaming",
            "Streaming",
            "Enable streaming responses",
            VariableType::Boolean,
        )
        .set_default_value(Some(json::json!(true)));

        node.add_output_pin(
            "config",
            "Config",
            "Session configuration",
            VariableType::Struct,
        )
        .set_schema::<CopilotSessionConfig>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let model: String = context.evaluate_pin("model").await.unwrap_or_default();
        let streaming: bool = context.evaluate_pin("streaming").await.unwrap_or(true);

        let config = CopilotSessionConfig {
            model: if model.is_empty() { None } else { Some(model) },
            streaming,
            ..Default::default()
        };

        context.set_pin_value("config", json::json!(config)).await?;
        Ok(())
    }
}

// =============================================================================
// Create Session Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct CopilotCreateSessionNode {}

#[async_trait]
impl NodeLogic for CopilotCreateSessionNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "copilot_create_session",
            "Copilot Create Session",
            "Creates a new Copilot chat session",
            "GitHub/Copilot/Session",
        );
        node.add_icon("/flow/icons/chat.svg");

        node.set_scores(
            NodeScores::new()
                .set_privacy(7)
                .set_security(8)
                .set_performance(7)
                .set_governance(8)
                .set_reliability(8)
                .set_cost(8)
                .build(),
        );

        node.add_input_pin("exec_in", "Input", "Trigger Pin", VariableType::Execution);

        node.add_input_pin(
            "client",
            "Client",
            "Running Copilot client",
            VariableType::Struct,
        )
        .set_schema::<CopilotClientHandle>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "config",
            "Config",
            "Optional session configuration",
            VariableType::Struct,
        )
        .set_schema::<CopilotSessionConfig>();

        node.add_output_pin(
            "exec_out",
            "Output",
            "Continues after session is created",
            VariableType::Execution,
        );

        node.add_output_pin("session", "Session", "Session handle", VariableType::Struct)
            .set_schema::<CopilotSessionHandle>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.set_long_running(true);

        node
    }

    #[cfg(feature = "execute")]
    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        use super::CachedCopilotClient;
        use copilot_sdk::SessionConfig;
        use flow_like::flow::execution::LogLevel;
        use flow_like_types::Cacheable;
        use std::sync::Arc;

        context.deactivate_exec_pin("exec_out").await?;

        let client_handle: CopilotClientHandle = context.evaluate_pin("client").await?;
        let config: Option<CopilotSessionConfig> = context.evaluate_pin("config").await.ok();

        let cached = {
            let cache = context.cache.read().await;
            cache.get(&client_handle.cache_key).cloned()
        };
        let cached =
            cached.ok_or_else(|| flow_like_types::anyhow!("Copilot client not found in cache"))?;
        let cached_client = cached
            .as_any()
            .downcast_ref::<CachedCopilotClient>()
            .ok_or_else(|| flow_like_types::anyhow!("Failed to downcast cached client"))?;

        context.log_message("Creating Copilot session...", LogLevel::Info);

        let sdk_config = if let Some(cfg) = config {
            SessionConfig {
                model: cfg.model,
                streaming: cfg.streaming,
                ..Default::default()
            }
        } else {
            SessionConfig::default()
        };

        let session = cached_client
            .client
            .create_session(sdk_config)
            .await
            .map_err(|e| flow_like_types::anyhow!("Failed to create session: {}", e))?;

        let session_id = session.session_id().to_string();
        let cache_key = format!("{}{}", COPILOT_SESSION_PREFIX, uuid::Uuid::new_v4());

        context.log_message(&format!("Created session: {}", session_id), LogLevel::Info);

        let cached_session = CachedCopilotSession { session };
        let cacheable: Arc<dyn Cacheable> = Arc::new(cached_session);
        context
            .cache
            .write()
            .await
            .insert(cache_key.clone(), cacheable);

        let handle = CopilotSessionHandle {
            cache_key,
            session_id,
            client_key: client_handle.cache_key,
        };

        context
            .set_pin_value("session", json::json!(handle))
            .await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }

    #[cfg(not(feature = "execute"))]
    async fn run(&self, _context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        Err(flow_like_types::anyhow!(
            "GitHub Copilot integration requires the 'execute' feature"
        ))
    }
}

// =============================================================================
// Destroy Session Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct CopilotDestroySessionNode {}

#[async_trait]
impl NodeLogic for CopilotDestroySessionNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "copilot_destroy_session",
            "Copilot Destroy Session",
            "Destroys a Copilot session",
            "GitHub/Copilot/Session",
        );
        node.add_icon("/flow/icons/delete.svg");

        node.set_scores(
            NodeScores::new()
                .set_privacy(10)
                .set_security(10)
                .set_performance(9)
                .set_governance(10)
                .set_reliability(9)
                .set_cost(10)
                .build(),
        );

        node.add_input_pin("exec_in", "Input", "Trigger Pin", VariableType::Execution);

        node.add_input_pin(
            "session",
            "Session",
            "Session handle to destroy",
            VariableType::Struct,
        )
        .set_schema::<CopilotSessionHandle>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "exec_out",
            "Output",
            "Continues after session is destroyed",
            VariableType::Execution,
        );

        node
    }

    #[cfg(feature = "execute")]
    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        use super::CachedCopilotSession;
        use flow_like::flow::execution::LogLevel;

        context.deactivate_exec_pin("exec_out").await?;

        let handle: CopilotSessionHandle = context.evaluate_pin("session").await?;

        let cached = {
            let cache = context.cache.read().await;
            cache.get(&handle.cache_key).cloned()
        };

        if let Some(cached) = cached {
            if let Some(cached_session) = cached.as_any().downcast_ref::<CachedCopilotSession>() {
                if let Err(e) = cached_session.session.destroy().await {
                    context.log_message(
                        &format!("Warning: Failed to destroy session: {}", e),
                        LogLevel::Warn,
                    );
                }
            }
        }

        context.cache.write().await.remove(&handle.cache_key);
        context.log_message("Session destroyed", LogLevel::Info);
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }

    #[cfg(not(feature = "execute"))]
    async fn run(&self, _context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        Err(flow_like_types::anyhow!(
            "GitHub Copilot integration requires the 'execute' feature"
        ))
    }
}
