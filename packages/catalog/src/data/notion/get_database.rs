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
pub struct NotionDatabaseProperty {
    pub id: String,
    pub name: String,
    pub property_type: String,
    pub config: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NotionDatabaseSchema {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub url: String,
    pub properties: Vec<NotionDatabaseProperty>,
    pub is_inline: bool,
    pub created_time: String,
    pub last_edited_time: String,
}

#[crate::register_node]
#[derive(Default)]
pub struct GetNotionDatabaseNode {}

impl GetNotionDatabaseNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for GetNotionDatabaseNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_notion_get_database",
            "Get Notion Database",
            "Retrieves a Notion database schema with its properties",
            "Data/Notion",
        );
        node.add_icon("/flow/icons/database.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Trigger database retrieval",
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
            "The ID of the Notion database to retrieve",
            VariableType::String,
        );

        node.add_output_pin(
            "exec_out",
            "Success",
            "Triggered when database is successfully retrieved",
            VariableType::Execution,
        );

        node.add_output_pin(
            "error",
            "Error",
            "Triggered when an error occurs",
            VariableType::Execution,
        );

        node.add_output_pin(
            "database",
            "Database",
            "The Notion database schema",
            VariableType::Struct,
        )
        .set_schema::<NotionDatabaseSchema>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin("title", "Title", "The database title", VariableType::String);

        node.add_output_pin(
            "property_names",
            "Property Names",
            "List of property names in the database",
            VariableType::String,
        )
        .set_value_type(flow_like::flow::pin::ValueType::Array);

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

        if database_id.is_empty() {
            context.log_message("Database ID cannot be empty", LogLevel::Error);
            context.activate_exec_pin("error").await?;
            return Ok(());
        }

        let client = reqwest::Client::new();
        let url = format!("https://api.notion.com/v1/databases/{}", database_id);

        context.log_message(
            &format!("Fetching Notion database: {}", database_id),
            LogLevel::Debug,
        );

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Notion-Version", NOTION_API_VERSION)
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

                let db_data: Value = resp
                    .json()
                    .await
                    .map_err(|e| flow_like_types::anyhow!("Failed to parse response: {}", e))?;

                let title = db_data["title"]
                    .as_array()
                    .and_then(|arr| arr.first())
                    .and_then(|t| t["plain_text"].as_str())
                    .unwrap_or("Untitled")
                    .to_string();

                let description = db_data["description"]
                    .as_array()
                    .and_then(|arr| arr.first())
                    .and_then(|t| t["plain_text"].as_str())
                    .map(String::from);

                let mut properties: Vec<NotionDatabaseProperty> = Vec::new();
                let mut property_names: Vec<String> = Vec::new();

                if let Some(props) = db_data["properties"].as_object() {
                    for (name, prop) in props {
                        let prop_type = prop["type"].as_str().unwrap_or("unknown").to_string();
                        let prop_id = prop["id"].as_str().unwrap_or("").to_string();

                        property_names.push(name.clone());
                        properties.push(NotionDatabaseProperty {
                            id: prop_id,
                            name: name.clone(),
                            property_type: prop_type.clone(),
                            config: prop[&prop_type].clone(),
                        });
                    }
                }

                let schema = NotionDatabaseSchema {
                    id: db_data["id"].as_str().unwrap_or("").to_string(),
                    title: title.clone(),
                    description,
                    url: db_data["url"].as_str().unwrap_or("").to_string(),
                    properties,
                    is_inline: db_data["is_inline"].as_bool().unwrap_or(false),
                    created_time: db_data["created_time"].as_str().unwrap_or("").to_string(),
                    last_edited_time: db_data["last_edited_time"]
                        .as_str()
                        .unwrap_or("")
                        .to_string(),
                };

                context.log_message(
                    &format!("Successfully retrieved database: {}", title),
                    LogLevel::Info,
                );

                context.set_pin_value("database", json!(schema)).await?;
                context.set_pin_value("title", json!(title)).await?;
                context
                    .set_pin_value("property_names", json!(property_names))
                    .await?;
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
