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
pub struct UpdatedNotionPage {
    pub id: String,
    pub url: String,
    pub last_edited_time: String,
}

#[crate::register_node]
#[derive(Default)]
pub struct UpdateNotionPageNode {}

impl UpdateNotionPageNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for UpdateNotionPageNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "data_notion_update_page",
            "Update Notion Page",
            "Updates properties of an existing Notion page",
            "Data/Notion",
        );
        node.add_icon("/flow/icons/notion.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Trigger page update",
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
            "page_id",
            "Page ID",
            "The ID of the page to update",
            VariableType::String,
        );

        node.add_input_pin(
            "properties",
            "Properties (JSON)",
            "Page properties to update in Notion format. Example: {\"Status\": {\"status\": {\"name\": \"Done\"}}}",
            VariableType::String,
        );

        node.add_input_pin(
            "icon_emoji",
            "Icon Emoji",
            "Optional: New emoji to use as the page icon (empty to keep current)",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "archived",
            "Archive",
            "Set to true to archive the page, false to unarchive",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_output_pin(
            "exec_out",
            "Success",
            "Triggered when page is successfully updated",
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
            "Updated Page",
            "The updated page info",
            VariableType::Struct,
        )
        .set_schema::<UpdatedNotionPage>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

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

        let page_id: String = context.evaluate_pin("page_id").await?;
        let properties_str: String = context.evaluate_pin("properties").await?;
        let icon_emoji: String = context.evaluate_pin("icon_emoji").await?;
        let archived: bool = context.evaluate_pin("archived").await?;

        if page_id.is_empty() {
            context.log_message("Page ID cannot be empty", LogLevel::Error);
            context.activate_exec_pin("error").await?;
            return Ok(());
        }

        let mut body = json!({});

        if !properties_str.is_empty() {
            match flow_like_types::json::from_str::<Value>(&properties_str) {
                Ok(properties) => {
                    body["properties"] = properties;
                }
                Err(e) => {
                    context
                        .log_message(&format!("Invalid properties JSON: {}", e), LogLevel::Error);
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

        if archived {
            body["archived"] = json!(true);
        }

        let client = reqwest::Client::new();
        let url = format!("https://api.notion.com/v1/pages/{}", page_id);

        context.log_message(
            &format!("Updating Notion page: {}", page_id),
            LogLevel::Debug,
        );

        let response = client
            .patch(&url)
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

                let updated_page = UpdatedNotionPage {
                    id: page_data["id"].as_str().unwrap_or("").to_string(),
                    url: page_data["url"].as_str().unwrap_or("").to_string(),
                    last_edited_time: page_data["last_edited_time"]
                        .as_str()
                        .unwrap_or("")
                        .to_string(),
                };

                context.log_message(
                    &format!("Successfully updated page: {}", page_id),
                    LogLevel::Info,
                );

                context.set_pin_value("page", json!(updated_page)).await?;
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
