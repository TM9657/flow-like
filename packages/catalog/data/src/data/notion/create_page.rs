use super::provider::{NOTION_PROVIDER_ID, NotionProvider};
use flow_like::{
    flow::{
        execution::{LogLevel, context::ExecutionContext},
        node::{Node, NodeLogic, NodeScores},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{JsonSchema, Value, async_trait, json::json, reqwest};
use serde::{Deserialize, Serialize};

const NOTION_API_VERSION: &str = "2022-06-28";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreatedNotionPage {
    pub id: String,
    pub url: String,
    pub created_time: String,
}

#[crate::register_node]
#[derive(Default)]
pub struct CreateNotionPageNode {}

impl CreateNotionPageNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for CreateNotionPageNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "data_notion_create_page",
            "Create Notion Page",
            "Creates a new page in a Notion database",
            "Data/Notion",
        );
        node.add_icon("/flow/icons/notion.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Trigger page creation",
            VariableType::Execution,
        );

        node.add_input_pin(
            "provider",
            "Provider",
            "Notion provider (from Notion node)",
            VariableType::Struct,
        )
        .set_schema::<NotionProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "database_id",
            "Database ID",
            "The ID of the database to create the page in",
            VariableType::String,
        );

        node.add_input_pin(
            "properties",
            "Properties (JSON)",
            "Page properties in Notion format. Example: {\"Name\": {\"title\": [{\"text\": {\"content\": \"My Page\"}}]}}",
            VariableType::String,
        );

        node.add_input_pin(
            "content",
            "Content (JSON)",
            "Optional: Page content as array of block objects in Notion format",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "icon_emoji",
            "Icon Emoji",
            "Optional: Emoji to use as the page icon",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_output_pin(
            "exec_out",
            "Success",
            "Triggered when page is successfully created",
            VariableType::Execution,
        );

        node.add_output_pin(
            "error",
            "Error",
            "Triggered when an error occurs",
            VariableType::Execution,
        );

        node.add_output_pin(
            "page",
            "Created Page",
            "The created page info",
            VariableType::Struct,
        )
        .set_schema::<CreatedNotionPage>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "page_id",
            "Page ID",
            "The ID of the created page",
            VariableType::String,
        );

        node.add_output_pin(
            "page_url",
            "Page URL",
            "The URL of the created page",
            VariableType::String,
        );

        node.add_required_oauth_scopes(NOTION_PROVIDER_ID, vec![]);
        node.set_scores(
            NodeScores::new()
                .set_privacy(6)
                .set_security(7)
                .set_performance(7)
                .set_governance(7)
                .set_reliability(9)
                .set_cost(10)
                .build(),
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: NotionProvider = context.evaluate_pin("provider").await?;
        let access_token = provider.access_token;

        let database_id: String = context.evaluate_pin("database_id").await?;
        let properties_str: String = context.evaluate_pin("properties").await?;
        let content_str: String = context.evaluate_pin("content").await?;
        let icon_emoji: String = context.evaluate_pin("icon_emoji").await?;

        if database_id.is_empty() {
            context.log_message("Database ID cannot be empty", LogLevel::Error);
            context.activate_exec_pin("error").await?;
            return Ok(());
        }

        if properties_str.is_empty() {
            context.log_message("Properties cannot be empty", LogLevel::Error);
            context.activate_exec_pin("error").await?;
            return Ok(());
        }

        let properties: Value = match flow_like_types::json::from_str(&properties_str) {
            Ok(p) => p,
            Err(e) => {
                context.log_message(&format!("Invalid properties JSON: {}", e), LogLevel::Error);
                context.activate_exec_pin("error").await?;
                return Ok(());
            }
        };

        let mut body = json!({
            "parent": { "database_id": database_id },
            "properties": properties
        });

        if !content_str.is_empty() {
            match flow_like_types::json::from_str::<Value>(&content_str) {
                Ok(children) => {
                    body["children"] = children;
                }
                Err(e) => {
                    context.log_message(&format!("Invalid content JSON: {}", e), LogLevel::Error);
                    context.activate_exec_pin("error").await?;
                    return Ok(());
                }
            }
        }

        if !icon_emoji.is_empty() {
            body["icon"] = json!({
                "type": "emoji",
                "emoji": icon_emoji
            });
        }

        let client = reqwest::Client::new();

        context.log_message("Creating Notion page...", LogLevel::Debug);

        let response = client
            .post("https://api.notion.com/v1/pages")
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Notion-Version", NOTION_API_VERSION)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if !resp.status().is_success() {
                    let status = resp.status();
                    let error_text = resp.text().await.unwrap_or_default();
                    context.log_message(
                        &format!("Notion API error {}: {}", status, error_text),
                        LogLevel::Error,
                    );
                    context.activate_exec_pin("error").await?;
                    return Ok(());
                }

                let page_data: Value = resp
                    .json()
                    .await
                    .map_err(|e| flow_like_types::anyhow!("Failed to parse response: {}", e))?;

                let page_id = page_data["id"].as_str().unwrap_or("").to_string();
                let page_url = page_data["url"].as_str().unwrap_or("").to_string();
                let created_time = page_data["created_time"].as_str().unwrap_or("").to_string();

                let created_page = CreatedNotionPage {
                    id: page_id.clone(),
                    url: page_url.clone(),
                    created_time,
                };

                context.log_message(
                    &format!("Successfully created page: {}", page_id),
                    LogLevel::Info,
                );

                context.set_pin_value("page", json!(created_page)).await?;
                context.set_pin_value("page_id", json!(page_id)).await?;
                context.set_pin_value("page_url", json!(page_url)).await?;
                context.activate_exec_pin("exec_out").await?;
            }
            Err(e) => {
                context.log_message(&format!("Network error: {}", e), LogLevel::Error);
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}
