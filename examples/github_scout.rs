//! A read-only GitHub scout over stdio, instrumented in one line. Built for the
//! demo: ask an agent what people want from a repository and watch the calls,
//! their intent and their failures arrive in PostHog.
//!
//! ```sh
//! POSTHOG_PROJECT_TOKEN=phc_... cargo run --example github_scout
//! ```
//!
//! It only ever sends GET requests. Without `GITHUB_TOKEN` GitHub allows 60
//! requests an hour (10 a minute for search); a read-only token raises that.

use std::sync::Arc;

use reqwest::StatusCode;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig};
use rmcp::schemars::{self, JsonSchema};
use rmcp::{ServerHandler, ServiceExt, tool, tool_handler, tool_router};
use serde::Deserialize;
use serde_json::Value;

const GITHUB_API: &str = "https://api.github.com";
const MAX_BODY_CHARACTERS: usize = 1500;
const MAX_RESULTS: &str = "10";

#[derive(Deserialize, JsonSchema)]
struct SearchArgs {
    /// Repository as owner/name, for example PostHog/posthog-rs
    repo: String,
    /// Words to search for in titles and descriptions
    query: String,
    /// open, closed, or omit for both
    state: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
struct IssueArgs {
    /// Repository as owner/name, for example PostHog/posthog-rs
    repo: String,
    /// Issue or pull request number
    number: u64,
}

#[derive(Deserialize, JsonSchema)]
struct PullRequestArgs {
    /// Repository as owner/name, for example PostHog/posthog-rs
    repo: String,
    /// open, closed, or all. Defaults to open
    state: Option<String>,
}

struct Scout {
    http: reqwest::Client,
    token: Option<String>,
}

impl Scout {
    fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
            token: std::env::var("GITHUB_TOKEN")
                .ok()
                .filter(|token| !token.is_empty()),
        }
    }

    async fn get(&self, path: &str, query: &[(&str, String)]) -> Result<Value, String> {
        let mut request = self
            .http
            .get(format!("{GITHUB_API}{path}"))
            .query(query)
            .header("User-Agent", "rmcp-posthog-github-scout")
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        let response = request
            .send()
            .await
            .map_err(|error| format!("GitHub is unreachable: {error}"))?;
        match response.status() {
            StatusCode::OK => response
                .json()
                .await
                .map_err(|error| format!("GitHub sent an unreadable response: {error}")),
            StatusCode::NOT_FOUND => Err(format!("GitHub has nothing at {path} (404)")),
            StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS => {
                Err("GitHub rate limit reached; set GITHUB_TOKEN for a higher limit".to_string())
            }
            status => Err(format!("GitHub answered {status} for {path}")),
        }
    }
}

fn checked_repo(repo: &str) -> Result<&str, String> {
    let is_owner_and_name = repo.split('/').filter(|part| !part.is_empty()).count() == 2;
    if is_owner_and_name {
        Ok(repo.trim_matches('/'))
    } else {
        Err(format!(
            "\"{repo}\" is not a repository; use owner/name, like PostHog/posthog-rs"
        ))
    }
}

fn text_result(outcome: Result<String, String>) -> CallToolResult {
    match outcome {
        Ok(text) => CallToolResult::success(vec![ContentBlock::text(text)]),
        Err(message) => CallToolResult::error(vec![ContentBlock::text(message)]),
    }
}

fn field<'a>(item: &'a Value, key: &str) -> &'a str {
    item.get(key).and_then(Value::as_str).unwrap_or("")
}

fn number(item: &Value, key: &str) -> u64 {
    item.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn kind(item: &Value) -> &'static str {
    if item.get("pull_request").is_some() {
        "pull request"
    } else {
        "issue"
    }
}

fn summary_line(item: &Value) -> String {
    format!(
        "#{} [{}] {} ({}, {} comments) {}",
        number(item, "number"),
        field(item, "state"),
        field(item, "title"),
        kind(item),
        number(item, "comments"),
        field(item, "html_url"),
    )
}

fn shortened(body: &str) -> String {
    if body.chars().count() <= MAX_BODY_CHARACTERS {
        return body.to_string();
    }
    let kept: String = body.chars().take(MAX_BODY_CHARACTERS).collect();
    format!("{kept}\n[... description shortened]")
}

#[tool_router]
impl Scout {
    #[tool(description = "Search a public GitHub repository's issues and pull requests")]
    async fn search_issues(&self, Parameters(args): Parameters<SearchArgs>) -> CallToolResult {
        text_result(self.search(args).await)
    }

    #[tool(description = "Read one GitHub issue or pull request, including its description")]
    async fn get_issue(&self, Parameters(args): Parameters<IssueArgs>) -> CallToolResult {
        text_result(self.issue(args).await)
    }

    #[tool(description = "List a public GitHub repository's most recently updated pull requests")]
    async fn list_pull_requests(
        &self,
        Parameters(args): Parameters<PullRequestArgs>,
    ) -> CallToolResult {
        text_result(self.pull_requests(args).await)
    }
}

impl Scout {
    async fn search(&self, args: SearchArgs) -> Result<String, String> {
        let repo = checked_repo(&args.repo)?;
        let mut query = format!("repo:{repo} {}", args.query);
        if let Some(state) = args.state.filter(|state| !state.is_empty()) {
            query += &format!(" state:{state}");
        }
        let params = [("q", query), ("per_page", MAX_RESULTS.to_string())];
        let found = self.get("/search/issues", &params).await?;
        let items = found
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if items.is_empty() {
            return Ok(format!("Nothing in {repo} matches \"{}\"", args.query));
        }
        let lines: Vec<String> = items.iter().map(summary_line).collect();
        let total = number(&found, "total_count");
        Ok(format!(
            "{total} results in {repo} for \"{}\":\n{}",
            args.query,
            lines.join("\n")
        ))
    }

    async fn issue(&self, args: IssueArgs) -> Result<String, String> {
        let repo = checked_repo(&args.repo)?;
        let issue = self
            .get(&format!("/repos/{repo}/issues/{}", args.number), &[])
            .await?;
        let author = issue
            .pointer("/user/login")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        Ok(format!(
            "{} #{}: {}\nstate: {} | author: {} | comments: {} | opened: {}\n{}\n\n{}",
            kind(&issue),
            args.number,
            field(&issue, "title"),
            field(&issue, "state"),
            author,
            number(&issue, "comments"),
            field(&issue, "created_at"),
            field(&issue, "html_url"),
            shortened(field(&issue, "body")),
        ))
    }

    async fn pull_requests(&self, args: PullRequestArgs) -> Result<String, String> {
        let repo = checked_repo(&args.repo)?;
        let state = args
            .state
            .filter(|state| !state.is_empty())
            .unwrap_or_else(|| "open".to_string());
        let params = [
            ("state", state.clone()),
            ("sort", "updated".to_string()),
            ("direction", "desc".to_string()),
            ("per_page", MAX_RESULTS.to_string()),
        ];
        let listed = self.get(&format!("/repos/{repo}/pulls"), &params).await?;
        let pulls = listed.as_array().cloned().unwrap_or_default();
        if pulls.is_empty() {
            return Ok(format!("{repo} has no {state} pull requests"));
        }
        let lines: Vec<String> = pulls.iter().map(summary_line).collect();
        Ok(format!(
            "Recently updated {state} pull requests in {repo}:\n{}",
            lines.join("\n")
        ))
    }
}

#[tool_handler]
impl ServerHandler for Scout {
    fn get_info(&self) -> ServerConfig {
        let capabilities = ServerCapabilities::builder().enable_tools().build();
        ServerConfig::new(capabilities)
            .with_server_info(Implementation::new("github-scout", "0.1.0"))
    }
}

async fn posthog_client() -> posthog_rs::Client {
    let token = std::env::var("POSTHOG_PROJECT_TOKEN").unwrap_or_default();
    let mut options = posthog_rs::ClientOptionsBuilder::default();
    // A server's IP says nothing about its users; posthog-node skips the lookup too.
    options.api_key(token).disable_geoip(true);
    if let Ok(host) = std::env::var("POSTHOG_HOST") {
        options.host(host);
    }
    let options = options.build().expect("valid PostHog options");
    posthog_rs::client(options).await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let posthog = Arc::new(posthog_client().await);

    let server = rmcp_posthog::instrument(Scout::new(), posthog.clone());

    let running = server.serve(rmcp::transport::stdio()).await?;
    running.waiting().await?;
    posthog.shutdown().await;
    Ok(())
}
