use super::provider::{NOTION_PROVIDER_ID, NotionProvider};
use flow_like::{
    flow::{
        execution::{LogLevel, context::ExecutionContext},
        node::{Node, NodeLogic, NodeScores},
        pin::{PinOptions, ValueType},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{JsonSchema, Value, async_trait, json::json, reqwest};
use serde::{Deserialize, Serialize};

const NOTION_API_VERSION: &str = "2022-06-28";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NotionDatabase {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub url: String,
    pub created_time: String,
    pub last_edited_time: String,
    pub icon_emoji: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    results: Vec<Value>,
    next_cursor: Option<String>,
    has_more: bool,
}

#[crate::register_node]
#[derive(Default)]
pub struct ListNotionDatabasesNode {}

impl ListNotionDatabasesNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for ListNotionDatabasesNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_notion_list_databases",
            "List Notion Databases",
            "Lists all databases the integration has access to",
            "Data/Notion",
        );
        node.add_icon("/flow/icons/database.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Trigger the database listing",
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
            "query",
            "Search Query",
            "Optional search query to filter databases by title",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "page_size",
            "Page Size",
            "Maximum number of databases to return (1-100)",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(100)));

        node.add_output_pin(
            "exec_out",
            "Success",
            "Triggered when databases are successfully listed",
            VariableType::Execution,
        );

        node.add_output_pin(
            "error",
            "Error",
            "Triggered when an error occurs",
            VariableType::Execution,
        );

        node.add_output_pin(
            "databases",
            "Databases",
            "Array of Notion databases",
            VariableType::Struct,
        )
        .set_value_type(ValueType::Array)
        .set_schema::<Vec<NotionDatabase>>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "count",
            "Count",
            "Number of databases returned",
            VariableType::Integer,
        );

        node.add_required_oauth_scopes(NOTION_PROVIDER_ID, vec![]);
        node.set_scores(
            NodeScores::new()
                .set_privacy(6)
                .set_security(8)
                .set_performance(7)
                .set_governance(7)
                .set_reliability(9)
                .set_cost(7)
                .build(),
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: NotionProvider = context.evaluate_pin("provider").await?;
        let access_token = provider.access_token;

        let query: String = context.evaluate_pin("query").await?;
        let page_size: i64 = context.evaluate_pin("page_size").await?;

        let client = reqwest::Client::new();

        let mut body = json!({
            "filter": {
                "value": "database",
                "property": "object"
            },
            "page_size": page_size.clamp(1, 100)
        });

        if !query.is_empty() {
            body["query"] = json!(query);
        }

        context.log_message("Fetching Notion databases...", LogLevel::Debug);

        let response = client
            .post("https://api.notion.com/v1/search")
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

                let search_response: SearchResponse = resp.json().await.map_err(|e| {
                    context
                        .log_message(&format!("Failed to parse response: {}", e), LogLevel::Error);
                    flow_like_types::anyhow!("Failed to parse Notion response")
                })?;

                let databases: Vec<NotionDatabase> = search_response
                    .results
                    .into_iter()
                    .filter_map(|db| {
                        let id = db["id"].as_str()?.to_string();
                        let title = db["title"]
                            .as_array()
                            .and_then(|arr| arr.first())
                            .and_then(|t| t["plain_text"].as_str())
                            .unwrap_or("Untitled")
                            .to_string();
                        let description = db["description"]
                            .as_array()
                            .and_then(|arr| arr.first())
                            .and_then(|t| t["plain_text"].as_str())
                            .map(String::from);
                        let url = db["url"].as_str()?.to_string();
                        let created_time = db["created_time"].as_str()?.to_string();
                        let last_edited_time = db["last_edited_time"].as_str()?.to_string();
                        let icon_emoji = db["icon"]["emoji"].as_str().map(String::from);

                        Some(NotionDatabase {
                            id,
                            title,
                            description,
                            url,
                            created_time,
                            last_edited_time,
                            icon_emoji,
                        })
                    })
                    .collect();

                let count = databases.len() as i64;

                context.log_message(&format!("Found {} Notion databases", count), LogLevel::Info);

                context.set_pin_value("databases", json!(databases)).await?;
                context.set_pin_value("count", json!(count)).await?;
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
