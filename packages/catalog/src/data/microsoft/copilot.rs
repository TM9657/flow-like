use super::provider::{MICROSOFT_PROVIDER_ID, MicrosoftGraphProvider};
use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::{PinOptions, ValueType},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{JsonSchema, Value, async_trait, json::json, reqwest};
use serde::{Deserialize, Serialize};

// =============================================================================
// Copilot Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CopilotInteraction {
    pub id: String,
    pub session_id: String,
    pub request_id: Option<String>,
    pub app_class: String,
    pub interaction_type: String,
    pub conversation_type: Option<String>,
    pub created_date_time: String,
    pub locale: Option<String>,
    pub body_content: String,
    pub body_content_type: String,
    pub from_user_id: Option<String>,
    pub from_user_name: Option<String>,
    pub from_app_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MeetingInsight {
    pub id: String,
    pub call_id: String,
    pub content_correlation_id: Option<String>,
    pub created_date_time: Option<String>,
    pub end_date_time: Option<String>,
    pub action_items: Vec<ActionItem>,
    pub meeting_notes: Vec<MeetingNote>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ActionItem {
    pub id: Option<String>,
    pub description: String,
    pub owner_name: Option<String>,
    pub due_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MeetingNote {
    pub id: Option<String>,
    pub content: String,
    pub created_date_time: Option<String>,
}

fn parse_interaction(value: &Value) -> Option<CopilotInteraction> {
    Some(CopilotInteraction {
        id: value["id"].as_str()?.to_string(),
        session_id: value["sessionId"].as_str()?.to_string(),
        request_id: value["requestId"].as_str().map(String::from),
        app_class: value["appClass"].as_str().unwrap_or("").to_string(),
        interaction_type: value["interactionType"].as_str().unwrap_or("").to_string(),
        conversation_type: value["conversationType"].as_str().map(String::from),
        created_date_time: value["createdDateTime"].as_str()?.to_string(),
        locale: value["locale"].as_str().map(String::from),
        body_content: value["body"]["content"].as_str().unwrap_or("").to_string(),
        body_content_type: value["body"]["contentType"]
            .as_str()
            .unwrap_or("text")
            .to_string(),
        from_user_id: value["from"]["user"]["id"].as_str().map(String::from),
        from_user_name: value["from"]["user"]["displayName"]
            .as_str()
            .map(String::from),
        from_app_name: value["from"]["application"]["displayName"]
            .as_str()
            .map(String::from),
    })
}

fn parse_action_item(value: &Value) -> Option<ActionItem> {
    Some(ActionItem {
        id: value["id"].as_str().map(String::from),
        description: value["description"].as_str()?.to_string(),
        owner_name: value["owner"]["displayName"].as_str().map(String::from),
        due_date: value["dueDateTime"].as_str().map(String::from),
    })
}

fn parse_meeting_note(value: &Value) -> Option<MeetingNote> {
    Some(MeetingNote {
        id: value["id"].as_str().map(String::from),
        content: value["content"].as_str()?.to_string(),
        created_date_time: value["createdDateTime"].as_str().map(String::from),
    })
}

fn parse_meeting_insight(value: &Value) -> Option<MeetingInsight> {
    Some(MeetingInsight {
        id: value["id"].as_str()?.to_string(),
        call_id: value["callId"].as_str()?.to_string(),
        content_correlation_id: value["contentCorrelationId"].as_str().map(String::from),
        created_date_time: value["createdDateTime"].as_str().map(String::from),
        end_date_time: value["endDateTime"].as_str().map(String::from),
        action_items: value["actionItems"]
            .as_array()
            .map(|arr| arr.iter().filter_map(parse_action_item).collect())
            .unwrap_or_default(),
        meeting_notes: value["meetingNotes"]
            .as_array()
            .map(|arr| arr.iter().filter_map(parse_meeting_note).collect())
            .unwrap_or_default(),
    })
}

// =============================================================================
// Get Copilot Interactions Node (Interaction Export API)
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct GetCopilotInteractionsNode {}

impl GetCopilotInteractionsNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for GetCopilotInteractionsNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_microsoft_copilot_get_interactions",
            "Get Copilot Interactions",
            "Get Microsoft 365 Copilot interaction history (prompts and responses)",
            "Data/Microsoft/Copilot",
        );
        node.add_icon("/flow/icons/copilot.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);
        node.add_input_pin(
            "provider",
            "Provider",
            "Microsoft Graph provider",
            VariableType::Struct,
        )
        .set_schema::<MicrosoftGraphProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());
        node.add_input_pin(
            "user_id",
            "User ID",
            "User ID to get interactions for",
            VariableType::String,
        );
        node.add_input_pin(
            "app_class_filter",
            "App Class Filter",
            "Filter by app (e.g., IPM.SkypeTeams.Message.Copilot.BizChat)",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));
        node.add_input_pin(
            "top",
            "Top",
            "Maximum number of results",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(100)));
        node.add_input_pin(
            "use_beta",
            "Use Beta API",
            "Use beta endpoint instead of v1.0",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_output_pin("exec_out", "Success", "", VariableType::Execution);
        node.add_output_pin("error", "Error", "", VariableType::Execution);
        node.add_output_pin("interactions", "Interactions", "", VariableType::Struct)
            .set_value_type(ValueType::Array)
            .set_schema::<Vec<CopilotInteraction>>();
        node.add_output_pin("count", "Count", "", VariableType::Integer);
        node.add_output_pin("error_message", "Error Message", "", VariableType::String);

        node.add_required_oauth_scopes(MICROSOFT_PROVIDER_ID, vec!["AiEnterpriseInteraction.Read"]);
        node.set_long_running(true);
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: MicrosoftGraphProvider = context.evaluate_pin("provider").await?;
        let user_id: String = context.evaluate_pin("user_id").await?;
        let app_class_filter: String = context
            .evaluate_pin("app_class_filter")
            .await
            .unwrap_or_default();
        let top: i64 = context.evaluate_pin("top").await.unwrap_or(100);
        let use_beta: bool = context.evaluate_pin("use_beta").await.unwrap_or(false);

        let api_version = if use_beta { "beta" } else { "v1.0" };
        let mut url = format!(
            "https://graph.microsoft.com/{}/copilot/users/{}/interactionHistory/getAllEnterpriseInteractions",
            api_version, user_id
        );

        let mut query_params = vec![format!("$top={}", top)];
        if !app_class_filter.is_empty() {
            query_params.push(format!("$filter=appClass eq '{}'", app_class_filter));
        }
        if !query_params.is_empty() {
            url = format!("{}?{}", url, query_params.join("&"));
        }

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let body: Value = resp.json().await?;
                let interactions: Vec<CopilotInteraction> = body["value"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(parse_interaction).collect())
                    .unwrap_or_default();
                let count = interactions.len() as i64;
                context
                    .set_pin_value("interactions", json!(interactions))
                    .await?;
                context.set_pin_value("count", json!(count)).await?;
                context.activate_exec_pin("exec_out").await?;
            }
            Ok(resp) => {
                let error = resp.text().await.unwrap_or_default();
                context.set_pin_value("error_message", json!(error)).await?;
                context.activate_exec_pin("error").await?;
            }
            Err(e) => {
                context
                    .set_pin_value("error_message", json!(e.to_string()))
                    .await?;
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}

// =============================================================================
// List Meeting Insights Node (Meeting Insights API)
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct ListMeetingInsightsNode {}

impl ListMeetingInsightsNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for ListMeetingInsightsNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_microsoft_copilot_list_meeting_insights",
            "List Meeting Insights",
            "Get AI-generated meeting notes and action items from Teams meetings",
            "Data/Microsoft/Copilot",
        );
        node.add_icon("/flow/icons/copilot.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);
        node.add_input_pin(
            "provider",
            "Provider",
            "Microsoft Graph provider",
            VariableType::Struct,
        )
        .set_schema::<MicrosoftGraphProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());
        node.add_input_pin(
            "meeting_id",
            "Meeting ID",
            "Online meeting ID",
            VariableType::String,
        );

        node.add_output_pin("exec_out", "Success", "", VariableType::Execution);
        node.add_output_pin("error", "Error", "", VariableType::Execution);
        node.add_output_pin("insights", "Insights", "", VariableType::Struct)
            .set_value_type(ValueType::Array)
            .set_schema::<Vec<MeetingInsight>>();
        node.add_output_pin("count", "Count", "", VariableType::Integer);
        node.add_output_pin("error_message", "Error Message", "", VariableType::String);

        node.add_required_oauth_scopes(
            MICROSOFT_PROVIDER_ID,
            vec!["OnlineMeetings.Read", "OnlineMeetingAiInsight.Read.All"],
        );
        node.set_long_running(true);
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: MicrosoftGraphProvider = context.evaluate_pin("provider").await?;
        let meeting_id: String = context.evaluate_pin("meeting_id").await?;

        let client = reqwest::Client::new();
        let response = client
            .get(&format!(
                "https://graph.microsoft.com/beta/me/onlineMeetings/{}/aiInsights",
                meeting_id
            ))
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let body: Value = resp.json().await?;
                let insights: Vec<MeetingInsight> = body["value"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(parse_meeting_insight).collect())
                    .unwrap_or_default();
                let count = insights.len() as i64;
                context.set_pin_value("insights", json!(insights)).await?;
                context.set_pin_value("count", json!(count)).await?;
                context.activate_exec_pin("exec_out").await?;
            }
            Ok(resp) => {
                let error = resp.text().await.unwrap_or_default();
                context.set_pin_value("error_message", json!(error)).await?;
                context.activate_exec_pin("error").await?;
            }
            Err(e) => {
                context
                    .set_pin_value("error_message", json!(e.to_string()))
                    .await?;
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}

// =============================================================================
// Get Meeting Insight Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct GetMeetingInsightNode {}

impl GetMeetingInsightNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for GetMeetingInsightNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_microsoft_copilot_get_meeting_insight",
            "Get Meeting Insight",
            "Get a specific AI insight from a Teams meeting",
            "Data/Microsoft/Copilot",
        );
        node.add_icon("/flow/icons/copilot.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);
        node.add_input_pin(
            "provider",
            "Provider",
            "Microsoft Graph provider",
            VariableType::Struct,
        )
        .set_schema::<MicrosoftGraphProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());
        node.add_input_pin(
            "meeting_id",
            "Meeting ID",
            "Online meeting ID",
            VariableType::String,
        );
        node.add_input_pin(
            "insight_id",
            "Insight ID",
            "AI insight ID",
            VariableType::String,
        );

        node.add_output_pin("exec_out", "Success", "", VariableType::Execution);
        node.add_output_pin("error", "Error", "", VariableType::Execution);
        node.add_output_pin("insight", "Insight", "", VariableType::Struct)
            .set_schema::<MeetingInsight>();
        node.add_output_pin("action_items", "Action Items", "", VariableType::Struct)
            .set_value_type(ValueType::Array)
            .set_schema::<Vec<ActionItem>>();
        node.add_output_pin("meeting_notes", "Meeting Notes", "", VariableType::Struct)
            .set_value_type(ValueType::Array)
            .set_schema::<Vec<MeetingNote>>();
        node.add_output_pin("error_message", "Error Message", "", VariableType::String);

        node.add_required_oauth_scopes(
            MICROSOFT_PROVIDER_ID,
            vec!["OnlineMeetings.Read", "OnlineMeetingAiInsight.Read.All"],
        );
        node.set_long_running(true);
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: MicrosoftGraphProvider = context.evaluate_pin("provider").await?;
        let meeting_id: String = context.evaluate_pin("meeting_id").await?;
        let insight_id: String = context.evaluate_pin("insight_id").await?;

        let client = reqwest::Client::new();
        let response = client
            .get(&format!(
                "https://graph.microsoft.com/beta/me/onlineMeetings/{}/aiInsights/{}",
                meeting_id, insight_id
            ))
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let body: Value = resp.json().await?;
                if let Some(insight) = parse_meeting_insight(&body) {
                    context
                        .set_pin_value("action_items", json!(insight.action_items.clone()))
                        .await?;
                    context
                        .set_pin_value("meeting_notes", json!(insight.meeting_notes.clone()))
                        .await?;
                    context.set_pin_value("insight", json!(insight)).await?;
                    context.activate_exec_pin("exec_out").await?;
                } else {
                    context
                        .set_pin_value("error_message", json!("Failed to parse response"))
                        .await?;
                    context.activate_exec_pin("error").await?;
                }
            }
            Ok(resp) => {
                let error = resp.text().await.unwrap_or_default();
                context.set_pin_value("error_message", json!(error)).await?;
                context.activate_exec_pin("error").await?;
            }
            Err(e) => {
                context
                    .set_pin_value("error_message", json!(e.to_string()))
                    .await?;
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}

// =============================================================================
// Copilot Search Node (Retrieval API - Semantic Search)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CopilotSearchResult {
    pub id: String,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub web_url: Option<String>,
    pub resource_type: Option<String>,
    pub relevance_score: Option<f64>,
}

fn parse_search_result(value: &Value) -> Option<CopilotSearchResult> {
    Some(CopilotSearchResult {
        id: value["id"].as_str()?.to_string(),
        title: value["resource"]["displayName"]
            .as_str()
            .map(String::from)
            .or_else(|| value["resource"]["name"].as_str().map(String::from)),
        summary: value["summary"].as_str().map(String::from),
        web_url: value["resource"]["webUrl"].as_str().map(String::from),
        resource_type: value["resource"]["@odata.type"].as_str().map(String::from),
        relevance_score: value["rank"].as_f64(),
    })
}

#[crate::register_node]
#[derive(Default)]
pub struct CopilotSearchNode {}

impl CopilotSearchNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for CopilotSearchNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_microsoft_copilot_search",
            "Copilot Search",
            "Search Microsoft 365 content using AI-powered semantic search",
            "Data/Microsoft/Copilot",
        );
        node.add_icon("/flow/icons/copilot.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);
        node.add_input_pin(
            "provider",
            "Provider",
            "Microsoft Graph provider",
            VariableType::Struct,
        )
        .set_schema::<MicrosoftGraphProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());
        node.add_input_pin(
            "query",
            "Query",
            "Natural language search query",
            VariableType::String,
        );
        node.add_input_pin(
            "entity_types",
            "Entity Types",
            "Comma-separated: driveItem,message,event,site,list,listItem",
            VariableType::String,
        )
        .set_default_value(Some(json!("driveItem")));
        node.add_input_pin(
            "size",
            "Size",
            "Maximum number of results",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(25)));

        node.add_output_pin("exec_out", "Success", "", VariableType::Execution);
        node.add_output_pin("error", "Error", "", VariableType::Execution);
        node.add_output_pin("results", "Results", "", VariableType::Struct)
            .set_value_type(ValueType::Array)
            .set_schema::<Vec<CopilotSearchResult>>();
        node.add_output_pin("count", "Count", "", VariableType::Integer);
        node.add_output_pin("error_message", "Error Message", "", VariableType::String);

        node.add_required_oauth_scopes(
            MICROSOFT_PROVIDER_ID,
            vec![
                "Files.Read.All",
                "Mail.Read",
                "Calendars.Read",
                "Sites.Read.All",
            ],
        );
        node.set_long_running(true);
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: MicrosoftGraphProvider = context.evaluate_pin("provider").await?;
        let query: String = context.evaluate_pin("query").await?;
        let entity_types: String = context
            .evaluate_pin("entity_types")
            .await
            .unwrap_or_else(|_| "driveItem".to_string());
        let size: i64 = context.evaluate_pin("size").await.unwrap_or(25);

        let entity_list: Vec<String> = entity_types
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let request_body = json!({
            "requests": [{
                "entityTypes": entity_list,
                "query": {
                    "queryString": query
                },
                "from": 0,
                "size": size
            }]
        });

        let client = reqwest::Client::new();
        let response = client
            .post("https://graph.microsoft.com/v1.0/search/query")
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let body: Value = resp.json().await?;
                let mut results: Vec<CopilotSearchResult> = Vec::new();

                if let Some(responses) = body["value"].as_array() {
                    for response in responses {
                        if let Some(hits_containers) = response["hitsContainers"].as_array() {
                            for container in hits_containers {
                                if let Some(hits) = container["hits"].as_array() {
                                    for hit in hits {
                                        if let Some(result) = parse_search_result(hit) {
                                            results.push(result);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                let count = results.len() as i64;
                context.set_pin_value("results", json!(results)).await?;
                context.set_pin_value("count", json!(count)).await?;
                context.activate_exec_pin("exec_out").await?;
            }
            Ok(resp) => {
                let error = resp.text().await.unwrap_or_default();
                context.set_pin_value("error_message", json!(error)).await?;
                context.activate_exec_pin("error").await?;
            }
            Err(e) => {
                context
                    .set_pin_value("error_message", json!(e.to_string()))
                    .await?;
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}

// =============================================================================
// Subscribe to Copilot Notifications Node (Change Notifications API)
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct SubscribeCopilotNotificationsNode {}

impl SubscribeCopilotNotificationsNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for SubscribeCopilotNotificationsNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_microsoft_copilot_subscribe_notifications",
            "Subscribe Copilot Notifications",
            "Subscribe to change notifications for Copilot interactions",
            "Data/Microsoft/Copilot",
        );
        node.add_icon("/flow/icons/copilot.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);
        node.add_input_pin(
            "provider",
            "Provider",
            "Microsoft Graph provider",
            VariableType::Struct,
        )
        .set_schema::<MicrosoftGraphProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());
        node.add_input_pin(
            "notification_url",
            "Notification URL",
            "Webhook URL to receive notifications",
            VariableType::String,
        );
        node.add_input_pin(
            "expiration_minutes",
            "Expiration (Minutes)",
            "Subscription expiration time",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(4230)));
        node.add_input_pin(
            "client_state",
            "Client State",
            "Optional client state for validation",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_output_pin("exec_out", "Success", "", VariableType::Execution);
        node.add_output_pin("error", "Error", "", VariableType::Execution);
        node.add_output_pin(
            "subscription_id",
            "Subscription ID",
            "",
            VariableType::String,
        );
        node.add_output_pin(
            "expiration_date_time",
            "Expiration DateTime",
            "",
            VariableType::String,
        );
        node.add_output_pin("error_message", "Error Message", "", VariableType::String);

        node.add_required_oauth_scopes(MICROSOFT_PROVIDER_ID, vec!["AiEnterpriseInteraction.Read"]);
        node.set_long_running(true);
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: MicrosoftGraphProvider = context.evaluate_pin("provider").await?;
        let notification_url: String = context.evaluate_pin("notification_url").await?;
        let expiration_minutes: i64 = context
            .evaluate_pin("expiration_minutes")
            .await
            .unwrap_or(4230);
        let client_state: String = context
            .evaluate_pin("client_state")
            .await
            .unwrap_or_default();

        // Calculate expiration datetime
        let expiration = chrono::Utc::now() + chrono::Duration::minutes(expiration_minutes);
        let expiration_str = expiration.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

        let mut request_body = json!({
            "changeType": "created",
            "notificationUrl": notification_url,
            "resource": "/copilot/users/getAllEnterpriseInteractions",
            "expirationDateTime": expiration_str
        });

        if !client_state.is_empty() {
            request_body["clientState"] = json!(client_state);
        }

        let client = reqwest::Client::new();
        let response = client
            .post("https://graph.microsoft.com/beta/subscriptions")
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let body: Value = resp.json().await?;
                let subscription_id = body["id"].as_str().unwrap_or("").to_string();
                let exp_dt = body["expirationDateTime"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                context
                    .set_pin_value("subscription_id", json!(subscription_id))
                    .await?;
                context
                    .set_pin_value("expiration_date_time", json!(exp_dt))
                    .await?;
                context.activate_exec_pin("exec_out").await?;
            }
            Ok(resp) => {
                let error = resp.text().await.unwrap_or_default();
                context.set_pin_value("error_message", json!(error)).await?;
                context.activate_exec_pin("error").await?;
            }
            Err(e) => {
                context
                    .set_pin_value("error_message", json!(e.to_string()))
                    .await?;
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}

// =============================================================================
// Get User Copilot Settings Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct GetUserCopilotSettingsNode {}

impl GetUserCopilotSettingsNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for GetUserCopilotSettingsNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_microsoft_copilot_get_user_settings",
            "Get User Copilot Settings",
            "Get the current user's Copilot settings and preferences",
            "Data/Microsoft/Copilot",
        );
        node.add_icon("/flow/icons/copilot.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);
        node.add_input_pin(
            "provider",
            "Provider",
            "Microsoft Graph provider",
            VariableType::Struct,
        )
        .set_schema::<MicrosoftGraphProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin("exec_out", "Success", "", VariableType::Execution);
        node.add_output_pin("error", "Error", "", VariableType::Execution);
        node.add_output_pin(
            "settings",
            "Settings",
            "Raw settings data",
            VariableType::Struct,
        );
        node.add_output_pin("error_message", "Error Message", "", VariableType::String);

        node.add_required_oauth_scopes(MICROSOFT_PROVIDER_ID, vec!["AiEnterpriseInteraction.Read"]);
        node.set_long_running(true);
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: MicrosoftGraphProvider = context.evaluate_pin("provider").await?;

        let client = reqwest::Client::new();
        let response = client
            .get("https://graph.microsoft.com/beta/me/settings/copilot")
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let body: Value = resp.json().await?;
                context.set_pin_value("settings", body).await?;
                context.activate_exec_pin("exec_out").await?;
            }
            Ok(resp) => {
                let error = resp.text().await.unwrap_or_default();
                context.set_pin_value("error_message", json!(error)).await?;
                context.activate_exec_pin("error").await?;
            }
            Err(e) => {
                context
                    .set_pin_value("error_message", json!(e.to_string()))
                    .await?;
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}

// =============================================================================
// Filter Copilot Interactions by Type Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct FilterCopilotInteractionsByTypeNode {}

impl FilterCopilotInteractionsByTypeNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for FilterCopilotInteractionsByTypeNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_microsoft_copilot_filter_interactions",
            "Filter Copilot Interactions",
            "Filter Copilot interactions by type (user prompts vs AI responses)",
            "Data/Microsoft/Copilot",
        );
        node.add_icon("/flow/icons/copilot.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);
        node.add_input_pin(
            "interactions",
            "Interactions",
            "Copilot interactions to filter",
            VariableType::Struct,
        )
        .set_value_type(ValueType::Array)
        .set_schema::<Vec<CopilotInteraction>>();
        node.add_input_pin(
            "interaction_type",
            "Interaction Type",
            "Type to filter by",
            VariableType::String,
        )
        .set_default_value(Some(json!("aiResponse")))
        .set_options(
            PinOptions::new()
                .set_valid_values(vec!["userPrompt".to_string(), "aiResponse".to_string()])
                .build(),
        );

        node.add_output_pin("exec_out", "Output", "", VariableType::Execution);
        node.add_output_pin("filtered", "Filtered", "", VariableType::Struct)
            .set_value_type(ValueType::Array)
            .set_schema::<Vec<CopilotInteraction>>();
        node.add_output_pin("count", "Count", "", VariableType::Integer);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let interactions: Vec<CopilotInteraction> = context.evaluate_pin("interactions").await?;
        let interaction_type: String = context
            .evaluate_pin("interaction_type")
            .await
            .unwrap_or_else(|_| "aiResponse".to_string());

        let filtered: Vec<CopilotInteraction> = interactions
            .into_iter()
            .filter(|i| i.interaction_type == interaction_type)
            .collect();

        let count = filtered.len() as i64;
        context.set_pin_value("filtered", json!(filtered)).await?;
        context.set_pin_value("count", json!(count)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
