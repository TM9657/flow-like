use super::{
    list_issues::{GitHubIssue, GitHubIssueUser, parse_issue},
    provider::{GITHUB_PROVIDER_ID, GitHubProvider},
};
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GitHubIssueComment {
    pub id: i64,
    pub body: String,
    pub html_url: String,
    pub user: GitHubIssueUser,
    pub created_at: String,
    pub updated_at: String,
}

fn parse_comment(comment: &Value) -> Option<GitHubIssueComment> {
    let user = &comment["user"];
    Some(GitHubIssueComment {
        id: comment["id"].as_i64()?,
        body: comment["body"].as_str()?.to_string(),
        html_url: comment["html_url"].as_str()?.to_string(),
        user: GitHubIssueUser {
            id: user["id"].as_i64()?,
            login: user["login"].as_str()?.to_string(),
            avatar_url: user["avatar_url"].as_str()?.to_string(),
            html_url: user["html_url"].as_str()?.to_string(),
        },
        created_at: comment["created_at"].as_str()?.to_string(),
        updated_at: comment["updated_at"].as_str()?.to_string(),
    })
}

// =============================================================================
// Get Issue Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct GetGitHubIssueNode {}

impl GetGitHubIssueNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for GetGitHubIssueNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_github_get_issue",
            "Get Issue",
            "Get details about a specific issue",
            "Data/GitHub",
        );
        node.add_icon("/flow/icons/github.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);

        node.add_input_pin(
            "provider",
            "Provider",
            "GitHub provider",
            VariableType::Struct,
        )
        .set_schema::<GitHubProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin("owner", "Owner", "Repository owner", VariableType::String);
        node.add_input_pin(
            "repo",
            "Repository",
            "Repository name",
            VariableType::String,
        );
        node.add_input_pin(
            "issue_number",
            "Issue Number",
            "Issue number",
            VariableType::Integer,
        );

        node.add_output_pin(
            "exec_out",
            "Success",
            "Triggered on success",
            VariableType::Execution,
        );

        node.add_output_pin(
            "error",
            "Error",
            "Triggered on error",
            VariableType::Execution,
        );

        node.add_output_pin("issue", "Issue", "Issue details", VariableType::Struct)
            .set_schema::<GitHubIssue>();

        node.add_output_pin("title", "Title", "Issue title", VariableType::String);
        node.add_output_pin("body", "Body", "Issue body", VariableType::String);
        node.add_output_pin(
            "state",
            "State",
            "Issue state (open/closed)",
            VariableType::String,
        );

        node.add_required_oauth_scopes(GITHUB_PROVIDER_ID, vec!["repo"]);

        node.set_scores(
            NodeScores::new()
                .set_privacy(7)
                .set_security(8)
                .set_performance(9)
                .set_governance(7)
                .set_reliability(9)
                .set_cost(9)
                .build(),
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: GitHubProvider = context.evaluate_pin("provider").await?;
        let owner: String = context.evaluate_pin("owner").await?;
        let repo: String = context.evaluate_pin("repo").await?;
        let issue_number: i64 = context.evaluate_pin("issue_number").await?;

        if owner.is_empty() || repo.is_empty() {
            context.log_message("Owner and repository are required", LogLevel::Error);
            context.activate_exec_pin("error").await?;
            return Ok(());
        }

        let url = provider.api_url(&format!(
            "/repos/{}/{}/issues/{}",
            owner, repo, issue_number
        ));

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "flow-like")
            .send()
            .await;

        match response {
            Ok(resp) => {
                if !resp.status().is_success() {
                    let status = resp.status();
                    let error_text = resp.text().await.unwrap_or_default();
                    context.log_message(
                        &format!("GitHub API error {}: {}", status, error_text),
                        LogLevel::Error,
                    );
                    context.activate_exec_pin("error").await?;
                    return Ok(());
                }

                let issue_json: Value = resp
                    .json()
                    .await
                    .map_err(|e| flow_like_types::anyhow!("Failed to parse response: {}", e))?;

                if let Some(issue) = parse_issue(&issue_json) {
                    context
                        .set_pin_value("title", json!(issue.title.clone()))
                        .await?;
                    context
                        .set_pin_value("body", json!(issue.body.clone().unwrap_or_default()))
                        .await?;
                    context
                        .set_pin_value("state", json!(issue.state.clone()))
                        .await?;
                    context.set_pin_value("issue", json!(issue)).await?;
                    context.activate_exec_pin("exec_out").await?;
                } else {
                    context.log_message("Failed to parse issue", LogLevel::Error);
                    context.activate_exec_pin("error").await?;
                }
            }
            Err(e) => {
                context.log_message(&format!("Network error: {}", e), LogLevel::Error);
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}

// =============================================================================
// Update Issue Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct UpdateGitHubIssueNode {}

impl UpdateGitHubIssueNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for UpdateGitHubIssueNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_github_update_issue",
            "Update Issue",
            "Update an existing issue",
            "Data/GitHub",
        );
        node.add_icon("/flow/icons/github.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);

        node.add_input_pin(
            "provider",
            "Provider",
            "GitHub provider",
            VariableType::Struct,
        )
        .set_schema::<GitHubProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin("owner", "Owner", "Repository owner", VariableType::String);
        node.add_input_pin(
            "repo",
            "Repository",
            "Repository name",
            VariableType::String,
        );
        node.add_input_pin(
            "issue_number",
            "Issue Number",
            "Issue number to update",
            VariableType::Integer,
        );

        node.add_input_pin(
            "title",
            "Title",
            "New title (leave empty to keep current)",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "body",
            "Body",
            "New body (leave empty to keep current)",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "state",
            "State",
            "New state: open or closed (leave empty to keep current)",
            VariableType::String,
        )
        .set_default_value(Some(json!("")))
        .set_options(
            PinOptions::new()
                .set_valid_values(vec![
                    "".to_string(),
                    "open".to_string(),
                    "closed".to_string(),
                ])
                .build(),
        );

        node.add_input_pin(
            "labels",
            "Labels",
            "Comma-separated list of label names (replaces all labels)",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "assignees",
            "Assignees",
            "Comma-separated list of usernames (replaces all assignees)",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_output_pin(
            "exec_out",
            "Success",
            "Triggered on success",
            VariableType::Execution,
        );

        node.add_output_pin(
            "error",
            "Error",
            "Triggered on error",
            VariableType::Execution,
        );

        node.add_output_pin("issue", "Issue", "Updated issue", VariableType::Struct)
            .set_schema::<GitHubIssue>();

        node.add_required_oauth_scopes(GITHUB_PROVIDER_ID, vec!["repo"]);

        node.set_scores(
            NodeScores::new()
                .set_privacy(6)
                .set_security(7)
                .set_performance(8)
                .set_governance(6)
                .set_reliability(9)
                .set_cost(8)
                .build(),
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: GitHubProvider = context.evaluate_pin("provider").await?;
        let owner: String = context.evaluate_pin("owner").await?;
        let repo: String = context.evaluate_pin("repo").await?;
        let issue_number: i64 = context.evaluate_pin("issue_number").await?;
        let title: String = context.evaluate_pin("title").await.unwrap_or_default();
        let body: String = context.evaluate_pin("body").await.unwrap_or_default();
        let state: String = context.evaluate_pin("state").await.unwrap_or_default();
        let labels: String = context.evaluate_pin("labels").await.unwrap_or_default();
        let assignees: String = context.evaluate_pin("assignees").await.unwrap_or_default();

        if owner.is_empty() || repo.is_empty() {
            context.log_message("Owner and repository are required", LogLevel::Error);
            context.activate_exec_pin("error").await?;
            return Ok(());
        }

        let url = provider.api_url(&format!(
            "/repos/{}/{}/issues/{}",
            owner, repo, issue_number
        ));

        let mut request_body = json!({});

        if !title.is_empty() {
            request_body["title"] = json!(title);
        }
        if !body.is_empty() {
            request_body["body"] = json!(body);
        }
        if !state.is_empty() {
            request_body["state"] = json!(state);
        }
        if !labels.is_empty() {
            let label_list: Vec<&str> = labels.split(',').map(|s| s.trim()).collect();
            request_body["labels"] = json!(label_list);
        }
        if !assignees.is_empty() {
            let assignee_list: Vec<&str> = assignees.split(',').map(|s| s.trim()).collect();
            request_body["assignees"] = json!(assignee_list);
        }

        let client = reqwest::Client::new();
        let response = client
            .patch(&url)
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "flow-like")
            .json(&request_body)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if !resp.status().is_success() {
                    let status = resp.status();
                    let error_text = resp.text().await.unwrap_or_default();
                    context.log_message(
                        &format!("GitHub API error {}: {}", status, error_text),
                        LogLevel::Error,
                    );
                    context.activate_exec_pin("error").await?;
                    return Ok(());
                }

                let issue_json: Value = resp
                    .json()
                    .await
                    .map_err(|e| flow_like_types::anyhow!("Failed to parse response: {}", e))?;

                if let Some(issue) = parse_issue(&issue_json) {
                    context.log_message(
                        &format!("Updated issue #{}: {}", issue.number, issue.title),
                        LogLevel::Info,
                    );
                    context.set_pin_value("issue", json!(issue)).await?;
                    context.activate_exec_pin("exec_out").await?;
                } else {
                    context.log_message("Failed to parse updated issue", LogLevel::Error);
                    context.activate_exec_pin("error").await?;
                }
            }
            Err(e) => {
                context.log_message(&format!("Network error: {}", e), LogLevel::Error);
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}

// =============================================================================
// Add Issue Comment Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct AddGitHubIssueCommentNode {}

impl AddGitHubIssueCommentNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for AddGitHubIssueCommentNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_github_add_issue_comment",
            "Add Issue Comment",
            "Add a comment to an issue or pull request",
            "Data/GitHub",
        );
        node.add_icon("/flow/icons/github.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);

        node.add_input_pin(
            "provider",
            "Provider",
            "GitHub provider",
            VariableType::Struct,
        )
        .set_schema::<GitHubProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin("owner", "Owner", "Repository owner", VariableType::String);
        node.add_input_pin(
            "repo",
            "Repository",
            "Repository name",
            VariableType::String,
        );
        node.add_input_pin(
            "issue_number",
            "Issue/PR Number",
            "Issue or PR number",
            VariableType::Integer,
        );
        node.add_input_pin(
            "body",
            "Body",
            "Comment body (Markdown supported)",
            VariableType::String,
        );

        node.add_output_pin(
            "exec_out",
            "Success",
            "Triggered on success",
            VariableType::Execution,
        );

        node.add_output_pin(
            "error",
            "Error",
            "Triggered on error",
            VariableType::Execution,
        );

        node.add_output_pin(
            "comment",
            "Comment",
            "Created comment",
            VariableType::Struct,
        )
        .set_schema::<GitHubIssueComment>();

        node.add_output_pin(
            "comment_id",
            "Comment ID",
            "ID of the created comment",
            VariableType::Integer,
        );

        node.add_required_oauth_scopes(GITHUB_PROVIDER_ID, vec!["repo"]);

        node.set_scores(
            NodeScores::new()
                .set_privacy(6)
                .set_security(7)
                .set_performance(8)
                .set_governance(6)
                .set_reliability(9)
                .set_cost(8)
                .build(),
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: GitHubProvider = context.evaluate_pin("provider").await?;
        let owner: String = context.evaluate_pin("owner").await?;
        let repo: String = context.evaluate_pin("repo").await?;
        let issue_number: i64 = context.evaluate_pin("issue_number").await?;
        let body: String = context.evaluate_pin("body").await?;

        if owner.is_empty() || repo.is_empty() || body.is_empty() {
            context.log_message(
                "Owner, repository, and comment body are required",
                LogLevel::Error,
            );
            context.activate_exec_pin("error").await?;
            return Ok(());
        }

        let url = provider.api_url(&format!(
            "/repos/{}/{}/issues/{}/comments",
            owner, repo, issue_number
        ));

        let request_body = json!({
            "body": body
        });

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "flow-like")
            .json(&request_body)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if !resp.status().is_success() {
                    let status = resp.status();
                    let error_text = resp.text().await.unwrap_or_default();
                    context.log_message(
                        &format!("GitHub API error {}: {}", status, error_text),
                        LogLevel::Error,
                    );
                    context.activate_exec_pin("error").await?;
                    return Ok(());
                }

                let comment_json: Value = resp
                    .json()
                    .await
                    .map_err(|e| flow_like_types::anyhow!("Failed to parse response: {}", e))?;

                if let Some(comment) = parse_comment(&comment_json) {
                    context.log_message(
                        &format!("Added comment to issue #{}", issue_number),
                        LogLevel::Info,
                    );
                    context
                        .set_pin_value("comment_id", json!(comment.id))
                        .await?;
                    context.set_pin_value("comment", json!(comment)).await?;
                    context.activate_exec_pin("exec_out").await?;
                } else {
                    context.log_message("Failed to parse created comment", LogLevel::Error);
                    context.activate_exec_pin("error").await?;
                }
            }
            Err(e) => {
                context.log_message(&format!("Network error: {}", e), LogLevel::Error);
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}

// =============================================================================
// List Issue Comments Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct ListGitHubIssueCommentsNode {}

impl ListGitHubIssueCommentsNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for ListGitHubIssueCommentsNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_github_list_issue_comments",
            "List Issue Comments",
            "List comments on an issue or pull request",
            "Data/GitHub",
        );
        node.add_icon("/flow/icons/github.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);

        node.add_input_pin(
            "provider",
            "Provider",
            "GitHub provider",
            VariableType::Struct,
        )
        .set_schema::<GitHubProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin("owner", "Owner", "Repository owner", VariableType::String);
        node.add_input_pin(
            "repo",
            "Repository",
            "Repository name",
            VariableType::String,
        );
        node.add_input_pin(
            "issue_number",
            "Issue/PR Number",
            "Issue or PR number",
            VariableType::Integer,
        );

        node.add_input_pin(
            "per_page",
            "Per Page",
            "Results per page (max 100)",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(30)));

        node.add_input_pin("page", "Page", "Page number", VariableType::Integer)
            .set_default_value(Some(json!(1)));

        node.add_output_pin(
            "exec_out",
            "Success",
            "Triggered on success",
            VariableType::Execution,
        );

        node.add_output_pin(
            "error",
            "Error",
            "Triggered on error",
            VariableType::Execution,
        );

        node.add_output_pin(
            "comments",
            "Comments",
            "Array of comments",
            VariableType::Struct,
        )
        .set_value_type(ValueType::Array)
        .set_schema::<GitHubIssueComment>();

        node.add_output_pin(
            "count",
            "Count",
            "Number of comments returned",
            VariableType::Integer,
        );

        node.add_required_oauth_scopes(GITHUB_PROVIDER_ID, vec!["repo"]);

        node.set_scores(
            NodeScores::new()
                .set_privacy(7)
                .set_security(8)
                .set_performance(8)
                .set_governance(7)
                .set_reliability(9)
                .set_cost(8)
                .build(),
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: GitHubProvider = context.evaluate_pin("provider").await?;
        let owner: String = context.evaluate_pin("owner").await?;
        let repo: String = context.evaluate_pin("repo").await?;
        let issue_number: i64 = context.evaluate_pin("issue_number").await?;
        let per_page: i64 = context.evaluate_pin("per_page").await.unwrap_or(30);
        let page: i64 = context.evaluate_pin("page").await.unwrap_or(1);

        if owner.is_empty() || repo.is_empty() {
            context.log_message("Owner and repository are required", LogLevel::Error);
            context.activate_exec_pin("error").await?;
            return Ok(());
        }

        let url = provider.api_url(&format!(
            "/repos/{}/{}/issues/{}/comments?per_page={}&page={}",
            owner,
            repo,
            issue_number,
            per_page.clamp(1, 100),
            page.max(1)
        ));

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "flow-like")
            .send()
            .await;

        match response {
            Ok(resp) => {
                if !resp.status().is_success() {
                    let status = resp.status();
                    let error_text = resp.text().await.unwrap_or_default();
                    context.log_message(
                        &format!("GitHub API error {}: {}", status, error_text),
                        LogLevel::Error,
                    );
                    context.activate_exec_pin("error").await?;
                    return Ok(());
                }

                let comments_json: Vec<Value> = resp
                    .json()
                    .await
                    .map_err(|e| flow_like_types::anyhow!("Failed to parse response: {}", e))?;

                let comments: Vec<GitHubIssueComment> =
                    comments_json.iter().filter_map(parse_comment).collect();

                let count = comments.len() as i64;

                context.log_message(&format!("Found {} comments", count), LogLevel::Info);
                context.set_pin_value("comments", json!(comments)).await?;
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
