/// # Copilot Utility Nodes
/// Nodes for getting status, models, and authentication information.
use super::{CopilotAuthStatus, CopilotClientHandle, CopilotModelInfo, CopilotStatusInfo};
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic, NodeScores},
    pin,
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{async_trait, json};

// =============================================================================
// List Models Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct CopilotListModelsNode {}

#[async_trait]
impl NodeLogic for CopilotListModelsNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "copilot_list_models",
            "Copilot List Models",
            "Lists available Copilot models",
            "GitHub/Copilot/Utilities",
        );
        node.add_icon("/flow/icons/list.svg");

        node.set_scores(
            NodeScores::new()
                .set_privacy(8)
                .set_security(8)
                .set_performance(8)
                .set_governance(8)
                .set_reliability(8)
                .set_cost(9)
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

        node.add_output_pin(
            "exec_out",
            "Output",
            "Continues after models are listed",
            VariableType::Execution,
        );

        node.add_output_pin("models", "Models", "Available models", VariableType::Struct)
            .set_value_type(pin::ValueType::Array)
            .set_schema::<CopilotModelInfo>();

        node
    }

    #[cfg(feature = "execute")]
    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        use super::CachedCopilotClient;
        use flow_like::flow::execution::LogLevel;

        context.deactivate_exec_pin("exec_out").await?;

        let handle: CopilotClientHandle = context.evaluate_pin("client").await?;

        let cached = {
            let cache = context.cache.read().await;
            cache.get(&handle.cache_key).cloned()
        };
        let cached =
            cached.ok_or_else(|| flow_like_types::anyhow!("Copilot client not found in cache"))?;
        let cached_client = cached
            .as_any()
            .downcast_ref::<CachedCopilotClient>()
            .ok_or_else(|| flow_like_types::anyhow!("Failed to downcast cached client"))?;

        context.log_message("Fetching available Copilot models...", LogLevel::Debug);

        let models_list = cached_client
            .client
            .list_models()
            .await
            .map_err(|e| flow_like_types::anyhow!("Failed to list models: {}", e))?;

        let models: Vec<CopilotModelInfo> = models_list
            .iter()
            .map(|m| CopilotModelInfo {
                id: m.id.clone(),
                name: m.name.clone(),
            })
            .collect();

        context.log_message(
            &format!("Found {} Copilot models", models.len()),
            LogLevel::Info,
        );

        context.set_pin_value("models", json::json!(models)).await?;
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
// Get Status Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct CopilotGetStatusNode {}

#[async_trait]
impl NodeLogic for CopilotGetStatusNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "copilot_get_status",
            "Copilot Get Status",
            "Gets CLI status including version and protocol info",
            "GitHub/Copilot/Utilities",
        );
        node.add_icon("/flow/icons/info.svg");

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
            "client",
            "Client",
            "Running Copilot client",
            VariableType::Struct,
        )
        .set_schema::<CopilotClientHandle>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "exec_out",
            "Output",
            "Continues after status is retrieved",
            VariableType::Execution,
        );

        node.add_output_pin(
            "status",
            "Status",
            "Status information",
            VariableType::Struct,
        )
        .set_schema::<CopilotStatusInfo>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin("version", "Version", "CLI version", VariableType::String);

        node.add_output_pin(
            "protocol_version",
            "Protocol Version",
            "Protocol version",
            VariableType::String,
        );

        node
    }

    #[cfg(feature = "execute")]
    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        use super::CachedCopilotClient;
        use flow_like::flow::execution::LogLevel;

        context.deactivate_exec_pin("exec_out").await?;

        let handle: CopilotClientHandle = context.evaluate_pin("client").await?;

        let cached = {
            let cache = context.cache.read().await;
            cache.get(&handle.cache_key).cloned()
        };
        let cached =
            cached.ok_or_else(|| flow_like_types::anyhow!("Copilot client not found in cache"))?;
        let cached_client = cached
            .as_any()
            .downcast_ref::<CachedCopilotClient>()
            .ok_or_else(|| flow_like_types::anyhow!("Failed to downcast cached client"))?;

        context.log_message("Fetching Copilot CLI status...", LogLevel::Debug);

        let status_response = cached_client
            .client
            .get_status()
            .await
            .map_err(|e| flow_like_types::anyhow!("Failed to get status: {}", e))?;

        let status = CopilotStatusInfo {
            version: status_response.version.clone(),
            protocol_version: status_response.protocol_version.to_string(),
        };

        context.log_message(
            &format!(
                "Copilot CLI v{} (protocol {})",
                status.version, status.protocol_version
            ),
            LogLevel::Info,
        );

        context.set_pin_value("status", json::json!(status)).await?;
        context
            .set_pin_value("version", json::json!(status_response.version))
            .await?;
        context
            .set_pin_value(
                "protocol_version",
                json::json!(status_response.protocol_version.to_string()),
            )
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
// Get Auth Status Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct CopilotGetAuthStatusNode {}

#[async_trait]
impl NodeLogic for CopilotGetAuthStatusNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "copilot_get_auth_status",
            "Copilot Get Auth Status",
            "Gets authentication status",
            "GitHub/Copilot/Utilities",
        );
        node.add_icon("/flow/icons/key.svg");

        node.set_scores(
            NodeScores::new()
                .set_privacy(6)
                .set_security(8)
                .set_performance(9)
                .set_governance(8)
                .set_reliability(9)
                .set_cost(10)
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

        node.add_output_pin(
            "exec_out",
            "Output",
            "Continues after auth status is retrieved",
            VariableType::Execution,
        );

        node.add_output_pin(
            "auth_status",
            "Auth Status",
            "Authentication status",
            VariableType::Struct,
        )
        .set_schema::<CopilotAuthStatus>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "authenticated",
            "Authenticated",
            "Whether the user is authenticated",
            VariableType::Boolean,
        );

        node.add_output_pin(
            "user",
            "User",
            "GitHub username if authenticated",
            VariableType::String,
        );

        node
    }

    #[cfg(feature = "execute")]
    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        use super::CachedCopilotClient;
        use flow_like::flow::execution::LogLevel;

        context.deactivate_exec_pin("exec_out").await?;

        let handle: CopilotClientHandle = context.evaluate_pin("client").await?;

        let cached = {
            let cache = context.cache.read().await;
            cache.get(&handle.cache_key).cloned()
        };
        let cached =
            cached.ok_or_else(|| flow_like_types::anyhow!("Copilot client not found in cache"))?;
        let cached_client = cached
            .as_any()
            .downcast_ref::<CachedCopilotClient>()
            .ok_or_else(|| flow_like_types::anyhow!("Failed to downcast cached client"))?;

        context.log_message("Fetching Copilot auth status...", LogLevel::Debug);

        let auth_response = cached_client
            .client
            .get_auth_status()
            .await
            .map_err(|e| flow_like_types::anyhow!("Failed to get auth status: {}", e))?;

        let authenticated = auth_response.is_authenticated;

        let auth_status = CopilotAuthStatus {
            authenticated,
            login: auth_response.login.clone(),
        };

        if authenticated {
            context.log_message(
                &format!(
                    "Copilot authenticated as: {}",
                    auth_response.login.as_deref().unwrap_or("unknown")
                ),
                LogLevel::Info,
            );
        } else {
            context.log_message("Copilot not authenticated", LogLevel::Warn);
        }

        context
            .set_pin_value("auth_status", json::json!(auth_status))
            .await?;
        context
            .set_pin_value("authenticated", json::json!(authenticated))
            .await?;
        context
            .set_pin_value("user", json::json!(auth_response.login.unwrap_or_default()))
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
// Ping Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct CopilotPingNode {}

#[async_trait]
impl NodeLogic for CopilotPingNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "copilot_ping",
            "Copilot Ping",
            "Pings the Copilot CLI to verify connection health",
            "GitHub/Copilot/Utilities",
        );
        node.add_icon("/flow/icons/heartbeat.svg");

        node.set_scores(
            NodeScores::new()
                .set_privacy(10)
                .set_security(10)
                .set_performance(10)
                .set_governance(10)
                .set_reliability(9)
                .set_cost(10)
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
            "message",
            "Message",
            "Optional ping message",
            VariableType::String,
        )
        .set_default_value(Some(json::json!("")));

        node.add_output_pin(
            "exec_out",
            "Output",
            "Continues after ping",
            VariableType::Execution,
        );

        node.add_output_pin(
            "success",
            "Success",
            "Whether the ping was successful",
            VariableType::Boolean,
        );

        node
    }

    #[cfg(feature = "execute")]
    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        use super::CachedCopilotClient;
        use flow_like::flow::execution::LogLevel;

        context.deactivate_exec_pin("exec_out").await?;

        let handle: CopilotClientHandle = context.evaluate_pin("client").await?;
        let message: String = context.evaluate_pin("message").await.unwrap_or_default();

        let cached = {
            let cache = context.cache.read().await;
            cache.get(&handle.cache_key).cloned()
        };
        let cached =
            cached.ok_or_else(|| flow_like_types::anyhow!("Copilot client not found in cache"))?;
        let cached_client = cached
            .as_any()
            .downcast_ref::<CachedCopilotClient>()
            .ok_or_else(|| flow_like_types::anyhow!("Failed to downcast cached client"))?;

        let ping_msg = if message.is_empty() {
            None
        } else {
            Some(message)
        };

        let result = cached_client.client.ping(ping_msg).await;

        let success = result.is_ok();

        if success {
            context.log_message("Copilot ping successful", LogLevel::Debug);
        } else {
            context.log_message(
                &format!("Copilot ping failed: {:?}", result.err()),
                LogLevel::Warn,
            );
        }

        context
            .set_pin_value("success", json::json!(success))
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
