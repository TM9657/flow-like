use super::provider::{GITHUB_PROVIDER_ID, GitHubProvider};
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
pub struct GitHubRepository {
    pub id: i64,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub html_url: String,
    pub clone_url: String,
    pub ssh_url: String,
    pub private: bool,
    pub fork: bool,
    pub archived: bool,
    pub default_branch: String,
    pub language: Option<String>,
    pub stargazers_count: i64,
    pub forks_count: i64,
    pub open_issues_count: i64,
    pub created_at: String,
    pub updated_at: String,
    pub pushed_at: Option<String>,
}

fn parse_repo(repo: &Value) -> Option<GitHubRepository> {
    Some(GitHubRepository {
        id: repo["id"].as_i64()?,
        name: repo["name"].as_str()?.to_string(),
        full_name: repo["full_name"].as_str()?.to_string(),
        description: repo["description"].as_str().map(String::from),
        html_url: repo["html_url"].as_str()?.to_string(),
        clone_url: repo["clone_url"].as_str().unwrap_or_default().to_string(),
        ssh_url: repo["ssh_url"].as_str().unwrap_or_default().to_string(),
        private: repo["private"].as_bool().unwrap_or(false),
        fork: repo["fork"].as_bool().unwrap_or(false),
        archived: repo["archived"].as_bool().unwrap_or(false),
        default_branch: repo["default_branch"]
            .as_str()
            .unwrap_or("main")
            .to_string(),
        language: repo["language"].as_str().map(String::from),
        stargazers_count: repo["stargazers_count"].as_i64().unwrap_or(0),
        forks_count: repo["forks_count"].as_i64().unwrap_or(0),
        open_issues_count: repo["open_issues_count"].as_i64().unwrap_or(0),
        created_at: repo["created_at"].as_str()?.to_string(),
        updated_at: repo["updated_at"].as_str()?.to_string(),
        pushed_at: repo["pushed_at"].as_str().map(String::from),
    })
}

#[crate::register_node]
#[derive(Default)]
pub struct ListGitHubReposNode {}

impl ListGitHubReposNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for ListGitHubReposNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_github_list_repos",
            "List Repositories",
            "List repositories for the authenticated user or a specified organization",
            "Data/GitHub",
        );
        node.add_icon("/flow/icons/github.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Trigger the repository listing",
            VariableType::Execution,
        );

        node.add_input_pin(
            "provider",
            "Provider",
            "GitHub provider",
            VariableType::Struct,
        )
        .set_schema::<GitHubProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "org",
            "Organization",
            "Optional organization name. If empty, lists user's repos",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "type",
            "Type",
            "Type of repos: all, owner, public, private, member (for user repos)",
            VariableType::String,
        )
        .set_default_value(Some(json!("all")));

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
            "repos",
            "Repositories",
            "Array of repositories",
            VariableType::Struct,
        )
        .set_value_type(ValueType::Array)
        .set_schema::<Vec<GitHubRepository>>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "count",
            "Count",
            "Number of repos returned",
            VariableType::Integer,
        );

        node.add_required_oauth_scopes(GITHUB_PROVIDER_ID, vec!["repo"]);
        node.set_scores(
            NodeScores::new()
                .set_privacy(6)
                .set_security(8)
                .set_performance(7)
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
        let org: String = context.evaluate_pin("org").await.unwrap_or_default();
        let repo_type: String = context
            .evaluate_pin("type")
            .await
            .unwrap_or_else(|_| "all".to_string());
        let per_page: i64 = context.evaluate_pin("per_page").await.unwrap_or(30);
        let page: i64 = context.evaluate_pin("page").await.unwrap_or(1);

        let url = if org.is_empty() {
            provider.api_url(&format!(
                "/user/repos?type={}&per_page={}&page={}",
                repo_type,
                per_page.clamp(1, 100),
                page.max(1)
            ))
        } else {
            provider.api_url(&format!(
                "/orgs/{}/repos?per_page={}&page={}",
                org,
                per_page.clamp(1, 100),
                page.max(1)
            ))
        };

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

                let repos_json: Vec<Value> = resp
                    .json()
                    .await
                    .map_err(|e| flow_like_types::anyhow!("Failed to parse response: {}", e))?;

                let repos: Vec<GitHubRepository> =
                    repos_json.iter().filter_map(parse_repo).collect();

                let count = repos.len() as i64;

                context.log_message(&format!("Found {} repositories", count), LogLevel::Info);
                context.set_pin_value("repos", json!(repos)).await?;
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
