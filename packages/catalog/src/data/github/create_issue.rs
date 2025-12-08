use super::{
    list_issues::{GitHubIssue, parse_issue},
    provider::{GITHUB_PROVIDER_ID, GitHubProvider},
};
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
pub struct CreateGitHubIssueNode {}

impl CreateGitHubIssueNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for CreateGitHubIssueNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_github_create_issue",
            "Create Issue",
            "Create a new issue in a repository",
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
        node.add_input_pin("title", "Title", "Issue title", VariableType::String);

        node.add_input_pin(
            "body",
            "Body",
            "Issue body (Markdown supported)",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "labels",
            "Labels",
            "Comma-separated list of label names",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "assignees",
            "Assignees",
            "Comma-separated list of usernames to assign",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "milestone",
            "Milestone",
            "Milestone number to associate with the issue",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(0)));

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

        node.add_output_pin("issue", "Issue", "Created issue", VariableType::Struct)
            .set_schema::<GitHubIssue>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "issue_number",
            "Issue Number",
            "The number of the created issue",
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
        let title: String = context.evaluate_pin("title").await?;
        let body: String = context.evaluate_pin("body").await.unwrap_or_default();
        let labels: String = context.evaluate_pin("labels").await.unwrap_or_default();
        let assignees: String = context.evaluate_pin("assignees").await.unwrap_or_default();
        let milestone: i64 = context.evaluate_pin("milestone").await.unwrap_or(0);

        if owner.is_empty() || repo.is_empty() || title.is_empty() {
            context.log_message("Owner, repository, and title are required", LogLevel::Error);
            context.activate_exec_pin("error").await?;
            return Ok(());
        }

        let url = provider.api_url(&format!("/repos/{}/{}/issues", owner, repo));

        let mut request_body = json!({
            "title": title
        });

        if !body.is_empty() {
            request_body["body"] = json!(body);
        }

        if !labels.is_empty() {
            let label_list: Vec<&str> = labels.split(',').map(|s| s.trim()).collect();
            request_body["labels"] = json!(label_list);
        }

        if !assignees.is_empty() {
            let assignee_list: Vec<&str> = assignees.split(',').map(|s| s.trim()).collect();
            request_body["assignees"] = json!(assignee_list);
        }

        if milestone > 0 {
            request_body["milestone"] = json!(milestone);
        }

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

                let issue_json: Value = resp
                    .json()
                    .await
                    .map_err(|e| flow_like_types::anyhow!("Failed to parse response: {}", e))?;

                if let Some(issue) = parse_issue(&issue_json) {
                    context.log_message(
                        &format!("Created issue #{}: {}", issue.number, issue.title),
                        LogLevel::Info,
                    );
                    context
                        .set_pin_value("issue_number", json!(issue.number))
                        .await?;
                    context.set_pin_value("issue", json!(issue)).await?;
                    context.activate_exec_pin("exec_out").await?;
                } else {
                    context.log_message("Failed to parse created issue", LogLevel::Error);
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
