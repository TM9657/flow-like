use super::provider::{NOTION_PROVIDER_ID, NotionProvider};
use flow_like::flow::{
    execution::{LogLevel, context::ExecutionContext},
    node::{Node, NodeLogic, NodeScores},
    pin::{PinOptions, ValueType},
    variable::VariableType,
};
use flow_like_types::{JsonSchema, Value, async_trait, json::json, reqwest};
use serde::{Deserialize, Serialize};

const NOTION_API_VERSION: &str = "2022-06-28";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NotionPage {
    pub id: String,
    pub url: String,
    pub created_time: String,
    pub last_edited_time: String,
    pub archived: bool,
    pub icon_emoji: Option<String>,
    pub properties: Value,
}

#[derive(Debug, Deserialize)]
struct QueryResponse {
    results: Vec<Value>,
    next_cursor: Option<String>,
    has_more: bool,
}

#[crate::register_node]
#[derive(Default)]
pub struct QueryNotionDatabaseNode {}

impl QueryNotionDatabaseNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for QueryNotionDatabaseNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "data_notion_query_database",
            "Query Notion Database",
            "Queries a Notion database and returns matching pages",
            "Data/Notion",
        );
        node.add_icon("/flow/icons/notion.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Trigger the database query",
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
            "The ID of the Notion database to query",
            VariableType::String,
        );

        node.add_input_pin(
            "filter",
            "Filter (JSON)",
            "Optional filter object in Notion filter format (JSON string). Example: {\"property\": \"Status\", \"status\": {\"equals\": \"Done\"}}",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "sort_property",
            "Sort Property",
            "Property name to sort by",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "sort_direction",
            "Sort Direction",
            "Sort direction (ascending or descending)",
            VariableType::String,
        )
        .set_default_value(Some(json!("descending")))
        .set_options(
            PinOptions::new()
                .set_valid_values(vec!["ascending".to_string(), "descending".to_string()])
                .build(),
        );

        node.add_input_pin(
            "page_size",
            "Page Size",
            "Maximum number of results to return (1-100)",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(100)));

        node.add_output_pin(
            "exec_out",
            "Success",
            "Triggered when query completes successfully",
            VariableType::Execution,
        );

        node.add_output_pin(
            "error",
            "Error",
            "Triggered when an error occurs",
            VariableType::Execution,
        );

        node.add_output_pin(
            "pages",
            "Pages",
            "Array of Notion pages matching the query",
            VariableType::Struct,
        )
        .set_value_type(ValueType::Array)
        .set_schema::<NotionPage>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "count",
            "Count",
            "Number of pages returned",
            VariableType::Integer,
        );

        node.add_output_pin(
            "has_more",
            "Has More",
            "Whether there are more results available",
            VariableType::Boolean,
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

        let database_id: String = context.evaluate_pin("database_id").await?;
        let filter_str: String = context.evaluate_pin("filter").await?;
        let sort_property: String = context.evaluate_pin("sort_property").await?;
        let sort_direction: String = context.evaluate_pin("sort_direction").await?;
        let page_size: i64 = context.evaluate_pin("page_size").await?;

        if database_id.is_empty() {
            context.log_message("Database ID cannot be empty", LogLevel::Error);
            context.activate_exec_pin("error").await?;
            return Ok(());
        }

        let client = reqwest::Client::new();
        let url = format!("https://api.notion.com/v1/databases/{}/query", database_id);

        let mut body = json!({
            "page_size": page_size.clamp(1, 100)
        });

        if !filter_str.is_empty() {
            match flow_like_types::json::from_str::<Value>(&filter_str) {
                Ok(filter) => {
                    body["filter"] = filter;
                }
                Err(e) => {
                    context.log_message(&format!("Invalid filter JSON: {}", e), LogLevel::Error);
                    context.activate_exec_pin("error").await?;
                    return Ok(());
                }
            }
        }

        if !sort_property.is_empty() {
            body["sorts"] = json!([{
                "property": sort_property,
                "direction": sort_direction
            }]);
        }

        context.log_message(
            &format!("Querying Notion database: {}", database_id),
            LogLevel::Debug,
        );

        let response = client
            .post(&url)
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

                let query_response: QueryResponse = resp.json().await.map_err(|e| {
                    context
                        .log_message(&format!("Failed to parse response: {}", e), LogLevel::Error);
                    flow_like_types::anyhow!("Failed to parse Notion response")
                })?;

                let pages: Vec<NotionPage> = query_response
                    .results
                    .into_iter()
                    .filter_map(|page| {
                        let id = page["id"].as_str()?.to_string();
                        let url = page["url"].as_str()?.to_string();
                        let created_time = page["created_time"].as_str()?.to_string();
                        let last_edited_time = page["last_edited_time"].as_str()?.to_string();
                        let archived = page["archived"].as_bool().unwrap_or(false);
                        let icon_emoji = page["icon"]["emoji"].as_str().map(String::from);
                        let properties = page["properties"].clone();

                        Some(NotionPage {
                            id,
                            url,
                            created_time,
                            last_edited_time,
                            archived,
                            icon_emoji,
                            properties,
                        })
                    })
                    .collect();

                let count = pages.len() as i64;

                context.log_message(
                    &format!("Found {} pages in database", count),
                    LogLevel::Info,
                );

                context.set_pin_value("pages", json!(pages)).await?;
                context.set_pin_value("count", json!(count)).await?;
                context
                    .set_pin_value("has_more", json!(query_response.has_more))
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
