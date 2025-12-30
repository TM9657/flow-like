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
pub struct NotionSearchResult {
    pub id: String,
    pub object_type: String,
    pub title: String,
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
pub struct SearchNotionNode {}

impl SearchNotionNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for SearchNotionNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "data_notion_search",
            "Search Notion",
            "Searches across all pages and databases the integration has access to",
            "Data/Notion",
        );
        node.add_icon("/flow/icons/notion.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Trigger search",
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

        node.add_input_pin("query", "Query", "Search query text", VariableType::String);

        node.add_input_pin(
            "filter_type",
            "Filter Type",
            "Filter results by type: all, page, or database",
            VariableType::String,
        )
        .set_default_value(Some(json!("all")))
        .set_options(
            PinOptions::new()
                .set_valid_values(vec![
                    "all".to_string(),
                    "page".to_string(),
                    "database".to_string(),
                ])
                .build(),
        );

        node.add_input_pin(
            "sort_direction",
            "Sort Direction",
            "Sort by last edited time",
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
            "Triggered when search completes successfully",
            VariableType::Execution,
        );

        node.add_output_pin(
            "error",
            "Error",
            "Triggered when an error occurs",
            VariableType::Execution,
        );

        node.add_output_pin(
            "results",
            "Results",
            "Array of search results",
            VariableType::Struct,
        )
        .set_value_type(ValueType::Array)
        .set_schema::<NotionSearchResult>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "count",
            "Count",
            "Number of results returned",
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

        let query: String = context.evaluate_pin("query").await?;
        let filter_type: String = context.evaluate_pin("filter_type").await?;
        let sort_direction: String = context.evaluate_pin("sort_direction").await?;
        let page_size: i64 = context.evaluate_pin("page_size").await?;

        let mut body = json!({
            "page_size": page_size.clamp(1, 100),
            "sort": {
                "direction": sort_direction,
                "timestamp": "last_edited_time"
            }
        });

        if !query.is_empty() {
            body["query"] = json!(query);
        }

        if filter_type != "all" {
            body["filter"] = json!({
                "value": filter_type,
                "property": "object"
            });
        }

        let client = reqwest::Client::new();

        context.log_message(&format!("Searching Notion for: {}", query), LogLevel::Debug);

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

                let search_response: SearchResponse = resp
                    .json()
                    .await
                    .map_err(|e| flow_like_types::anyhow!("Failed to parse response: {}", e))?;

                let results: Vec<NotionSearchResult> = search_response
                    .results
                    .into_iter()
                    .filter_map(|item| {
                        let id = item["id"].as_str()?.to_string();
                        let object_type = item["object"].as_str()?.to_string();
                        let url = item["url"].as_str()?.to_string();
                        let created_time = item["created_time"].as_str()?.to_string();
                        let last_edited_time = item["last_edited_time"].as_str()?.to_string();
                        let icon_emoji = item["icon"]["emoji"].as_str().map(String::from);

                        let title = if object_type == "database" {
                            item["title"]
                                .as_array()
                                .and_then(|arr| arr.first())
                                .and_then(|t| t["plain_text"].as_str())
                                .unwrap_or("Untitled")
                                .to_string()
                        } else {
                            item["properties"]["title"]["title"]
                                .as_array()
                                .or_else(|| item["properties"]["Name"]["title"].as_array())
                                .and_then(|arr| arr.first())
                                .and_then(|t| t["plain_text"].as_str())
                                .unwrap_or("Untitled")
                                .to_string()
                        };

                        Some(NotionSearchResult {
                            id,
                            object_type,
                            title,
                            url,
                            created_time,
                            last_edited_time,
                            icon_emoji,
                        })
                    })
                    .collect();

                let count = results.len() as i64;

                context.log_message(&format!("Found {} results", count), LogLevel::Info);

                context.set_pin_value("results", json!(results)).await?;
                context.set_pin_value("count", json!(count)).await?;
                context
                    .set_pin_value("has_more", json!(search_response.has_more))
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
