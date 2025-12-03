use super::{JiraIssue, parse_jira_issue};
use crate::data::atlassian::provider::{ATLASSIAN_PROVIDER_ID, AtlassianProvider};
use flow_like::{
    flow::{
        execution::{LogLevel, context::ExecutionContext},
        node::{Node, NodeLogic, NodeScores},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{Value, async_trait, json::json, reqwest};

#[crate::register_node]
#[derive(Default)]
pub struct CreateJiraIssueNode {}

impl CreateJiraIssueNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for CreateJiraIssueNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_atlassian_jira_create_issue",
            "Create Jira Issue",
            "Create a new Jira issue",
            "Data/Atlassian/Jira",
        );
        node.add_icon("/flow/icons/plus-circle.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Trigger the creation",
            VariableType::Execution,
        );

        node.add_input_pin(
            "provider",
            "Provider",
            "Atlassian provider (from Atlassian node)",
            VariableType::Struct,
        )
        .set_schema::<AtlassianProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "project_key",
            "Project Key",
            "The project key (e.g., PROJ)",
            VariableType::String,
        );

        node.add_input_pin(
            "issue_type",
            "Issue Type",
            "The issue type name (e.g., Bug, Story, Task)",
            VariableType::String,
        )
        .set_default_value(Some(json!("Task")));

        node.add_input_pin(
            "summary",
            "Summary",
            "Issue summary/title",
            VariableType::String,
        );

        node.add_input_pin(
            "description",
            "Description",
            "Issue description (plain text)",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "priority",
            "Priority",
            "Issue priority name (e.g., Highest, High, Medium, Low, Lowest)",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "assignee_id",
            "Assignee ID",
            "Account ID of the assignee (leave empty for unassigned)",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "labels",
            "Labels",
            "Comma-separated list of labels",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "parent_key",
            "Parent Key",
            "Parent issue key for subtasks (e.g., PROJ-123)",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_output_pin(
            "exec_out",
            "Success",
            "Triggered when issue is created successfully",
            VariableType::Execution,
        );

        node.add_output_pin(
            "error",
            "Error",
            "Triggered when an error occurs",
            VariableType::Execution,
        );

        node.add_output_pin(
            "issue",
            "Issue",
            "The created Jira issue",
            VariableType::Struct,
        )
        .set_schema::<JiraIssue>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "issue_key",
            "Issue Key",
            "The key of the created issue (e.g., PROJ-123)",
            VariableType::String,
        );

        node.add_required_oauth_scopes(ATLASSIAN_PROVIDER_ID, vec!["write:jira-work"]);
        node.set_scores(
            NodeScores::new()
                .set_privacy(6)
                .set_security(8)
                .set_performance(7)
                .set_governance(7)
                .set_reliability(8)
                .set_cost(7)
                .build(),
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: AtlassianProvider = context.evaluate_pin("provider").await?;
        let project_key: String = context.evaluate_pin("project_key").await?;
        let issue_type: String = context.evaluate_pin("issue_type").await?;
        let summary: String = context.evaluate_pin("summary").await?;
        let description: String = context.evaluate_pin("description").await?;
        let priority: String = context.evaluate_pin("priority").await?;
        let assignee_id: String = context.evaluate_pin("assignee_id").await?;
        let labels: String = context.evaluate_pin("labels").await?;
        let parent_key: String = context.evaluate_pin("parent_key").await?;

        if project_key.is_empty() {
            context.log_message("Project key is required", LogLevel::Error);
            context.activate_exec_pin("error").await?;
            return Ok(());
        }

        if summary.is_empty() {
            context.log_message("Summary is required", LogLevel::Error);
            context.activate_exec_pin("error").await?;
            return Ok(());
        }

        let client = reqwest::Client::new();
        let url = provider.jira_api_url("/issue");

        // Build the issue fields
        let mut fields = json!({
            "project": {
                "key": project_key
            },
            "summary": summary,
            "issuetype": {
                "name": issue_type
            }
        });

        // Add description (use ADF format for cloud)
        if !description.is_empty() {
            if provider.is_cloud {
                fields["description"] = json!({
                    "type": "doc",
                    "version": 1,
                    "content": [
                        {
                            "type": "paragraph",
                            "content": [
                                {
                                    "type": "text",
                                    "text": description
                                }
                            ]
                        }
                    ]
                });
            } else {
                fields["description"] = json!(description);
            }
        }

        if !priority.is_empty() {
            fields["priority"] = json!({ "name": priority });
        }

        if !assignee_id.is_empty() {
            if provider.is_cloud {
                fields["assignee"] = json!({ "accountId": assignee_id });
            } else {
                fields["assignee"] = json!({ "name": assignee_id });
            }
        }

        if !labels.is_empty() {
            let label_list: Vec<&str> = labels.split(',').map(|s| s.trim()).collect();
            fields["labels"] = json!(label_list);
        }

        if !parent_key.is_empty() {
            fields["parent"] = json!({ "key": parent_key });
        }

        let body = json!({ "fields": fields });

        context.log_message(
            &format!("Creating Jira issue in project {}", project_key),
            LogLevel::Debug,
        );

        let response = client
            .post(&url)
            .header("Authorization", provider.auth_header())
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&body)
            .send()
            .await;

        let response = match response {
            Ok(r) => r,
            Err(e) => {
                context.log_message(&format!("Request failed: {}", e), LogLevel::Error);
                context.activate_exec_pin("error").await?;
                return Ok(());
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            context.log_message(
                &format!("Jira API error {}: {}", status, error_text),
                LogLevel::Error,
            );
            context.activate_exec_pin("error").await?;
            return Ok(());
        }

        let data: Value = match response.json().await {
            Ok(d) => d,
            Err(e) => {
                context.log_message(&format!("Failed to parse response: {}", e), LogLevel::Error);
                context.activate_exec_pin("error").await?;
                return Ok(());
            }
        };

        let issue_key = data
            .get("key")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        context.log_message(&format!("Created issue: {}", issue_key), LogLevel::Debug);

        // Fetch the full issue details
        let issue = fetch_created_issue(&provider, &issue_key, context).await?;

        context.set_pin_value("issue", json!(issue)).await?;
        context.set_pin_value("issue_key", json!(issue_key)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}

async fn fetch_created_issue(
    provider: &AtlassianProvider,
    issue_key: &str,
    context: &mut ExecutionContext,
) -> flow_like_types::Result<Option<JiraIssue>> {
    let client = reqwest::Client::new();
    let url = provider.jira_api_url(&format!("/issue/{}", issue_key));

    let response = client
        .get(&url)
        .header("Authorization", provider.auth_header())
        .header("Accept", "application/json")
        .send()
        .await;

    let response = match response {
        Ok(r) => r,
        Err(e) => {
            context.log_message(
                &format!("Failed to fetch created issue details: {}", e),
                LogLevel::Warn,
            );
            return Ok(None);
        }
    };

    if !response.status().is_success() {
        return Ok(None);
    }

    let data: Value = match response.json().await {
        Ok(d) => d,
        Err(_) => return Ok(None),
    };

    Ok(parse_jira_issue(&data, &provider.base_url))
}
