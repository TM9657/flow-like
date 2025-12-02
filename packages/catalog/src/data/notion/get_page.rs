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
pub struct NotionBlock {
    pub id: String,
    pub block_type: String,
    pub has_children: bool,
    pub content: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NotionPageContent {
    pub id: String,
    pub url: String,
    pub title: String,
    pub created_time: String,
    pub last_edited_time: String,
    pub archived: bool,
    pub icon_emoji: Option<String>,
    pub properties: Value,
    pub blocks: Vec<NotionBlock>,
    pub plain_text: String,
}

#[derive(Debug, Deserialize)]
struct BlocksResponse {
    results: Vec<Value>,
    next_cursor: Option<String>,
    has_more: bool,
}

#[crate::register_node]
#[derive(Default)]
pub struct GetNotionPageNode {}

impl GetNotionPageNode {
    pub fn new() -> Self {
        Self {}
    }
}

fn extract_plain_text_from_rich_text(rich_text: &Value) -> String {
    rich_text
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|t| t["plain_text"].as_str())
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}

fn extract_block_text(block: &Value) -> String {
    let block_type = block["type"].as_str().unwrap_or("");

    match block_type {
        "paragraph" | "heading_1" | "heading_2" | "heading_3" | "bulleted_list_item"
        | "numbered_list_item" | "quote" | "callout" | "toggle" => {
            extract_plain_text_from_rich_text(&block[block_type]["rich_text"])
        }
        "code" => extract_plain_text_from_rich_text(&block["code"]["rich_text"]),
        "to_do" => {
            let text = extract_plain_text_from_rich_text(&block["to_do"]["rich_text"]);
            let checked = block["to_do"]["checked"].as_bool().unwrap_or(false);
            format!("[{}] {}", if checked { "x" } else { " " }, text)
        }
        "divider" => "---".to_string(),
        _ => String::new(),
    }
}

#[async_trait]
impl NodeLogic for GetNotionPageNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_notion_get_page",
            "Get Notion Page",
            "Retrieves a Notion page with its content and blocks",
            "Data/Notion",
        );
        node.add_icon("/flow/icons/file-text.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Trigger page retrieval",
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
            "The ID of the Notion page to retrieve",
            VariableType::String,
        );

        node.add_input_pin(
            "include_content",
            "Include Content",
            "Whether to fetch the page content blocks",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_output_pin(
            "exec_out",
            "Success",
            "Triggered when page is successfully retrieved",
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
            "Page",
            "The Notion page with content",
            VariableType::Struct,
        )
        .set_schema::<NotionPageContent>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin("title", "Title", "The page title", VariableType::String);

        node.add_output_pin(
            "plain_text",
            "Plain Text",
            "The page content as plain text",
            VariableType::String,
        );

        node.add_output_pin(
            "blocks",
            "Blocks",
            "Array of content blocks",
            VariableType::Struct,
        )
        .set_value_type(ValueType::Array)
        .set_schema::<Vec<NotionBlock>>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_required_oauth_scopes(NOTION_PROVIDER_ID, vec![]);
        node.set_scores(
            NodeScores::new()
                .set_privacy(6)
                .set_security(8)
                .set_performance(6)
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

        let page_id: String = context.evaluate_pin("page_id").await?;
        let include_content: bool = context.evaluate_pin("include_content").await?;

        if page_id.is_empty() {
            context.log_message("Page ID cannot be empty", LogLevel::Error);
            context.activate_exec_pin("error").await?;
            return Ok(());
        }

        let client = reqwest::Client::new();

        context.log_message(
            &format!("Fetching Notion page: {}", page_id),
            LogLevel::Debug,
        );

        let page_response = client
            .get(&format!("https://api.notion.com/v1/pages/{}", page_id))
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Notion-Version", NOTION_API_VERSION)
            .send()
            .await;

        let page_data: Value = match page_response {
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
                resp.json()
                    .await
                    .map_err(|e| flow_like_types::anyhow!("Failed to parse page response: {}", e))?
            }
            Err(e) => {
                context.log_message(&format!("Network error: {}", e), LogLevel::Error);
                context.activate_exec_pin("error").await?;
                return Ok(());
            }
        };

        let title = page_data["properties"]["title"]["title"]
            .as_array()
            .or_else(|| page_data["properties"]["Name"]["title"].as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| t["plain_text"].as_str())
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_else(|| "Untitled".to_string());

        let mut blocks: Vec<NotionBlock> = Vec::new();
        let mut plain_text_parts: Vec<String> = Vec::new();

        if include_content {
            let blocks_response = client
                .get(&format!(
                    "https://api.notion.com/v1/blocks/{}/children",
                    page_id
                ))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Notion-Version", NOTION_API_VERSION)
                .send()
                .await;

            if let Ok(resp) = blocks_response {
                if resp.status().is_success() {
                    if let Ok(blocks_data) = resp.json::<BlocksResponse>().await {
                        for block in blocks_data.results {
                            let block_type =
                                block["type"].as_str().unwrap_or("unknown").to_string();
                            let text = extract_block_text(&block);

                            if !text.is_empty() {
                                plain_text_parts.push(text);
                            }

                            blocks.push(NotionBlock {
                                id: block["id"].as_str().unwrap_or("").to_string(),
                                block_type: block_type.clone(),
                                has_children: block["has_children"].as_bool().unwrap_or(false),
                                content: block[&block_type].clone(),
                            });
                        }
                    }
                }
            }
        }

        let plain_text = plain_text_parts.join("\n");

        let page_content = NotionPageContent {
            id: page_data["id"].as_str().unwrap_or("").to_string(),
            url: page_data["url"].as_str().unwrap_or("").to_string(),
            title: title.clone(),
            created_time: page_data["created_time"].as_str().unwrap_or("").to_string(),
            last_edited_time: page_data["last_edited_time"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            archived: page_data["archived"].as_bool().unwrap_or(false),
            icon_emoji: page_data["icon"]["emoji"].as_str().map(String::from),
            properties: page_data["properties"].clone(),
            blocks: blocks.clone(),
            plain_text: plain_text.clone(),
        };

        context.log_message(
            &format!("Successfully retrieved page: {}", title),
            LogLevel::Info,
        );

        context.set_pin_value("page", json!(page_content)).await?;
        context.set_pin_value("title", json!(title)).await?;
        context
            .set_pin_value("plain_text", json!(plain_text))
            .await?;
        context.set_pin_value("blocks", json!(blocks)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
