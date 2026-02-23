use async_trait::async_trait;
use chrono::NaiveDate;
use serde::Deserialize;

use super::{ConnectionStatus, SyncService, SyncedEntry};
use crate::shared::error::AppError;

/// Syncs PRs from GitHub via the GraphQL API: authored, merged, and reviewed.
/// Requires a token (Classic PAT, zero scopes is sufficient for public repos).
/// No PR body content is stored — only title, URL, state, repo, and dates.
pub struct GitHubSync;

/// Resolved user identity for a GitHub sync operation.
struct GitHubUser<'a> {
    username: &'a str,
    orgs_str: &'a str,
}

// Envelope for GitHub's GraphQL JSON response.
#[derive(Debug, Deserialize)]
struct GraphQLResponse {
    data: Option<serde_json::Value>,
    errors: Option<Vec<serde_json::Value>>,
}

// Discriminant for the three PR search queries we issue.
#[derive(Debug)]
enum GitHubQueryType {
    Authored,
    Merged,
    Reviewed,
}

#[async_trait]
impl SyncService for GitHubSync {
    async fn sync(
        &self,
        client: &reqwest::Client,
        token: &str,
        config: &serde_json::Value,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<SyncedEntry>, AppError> {
        let user_orgs_str = config["org"].as_str().unwrap_or("");
        let username = config["username"].as_str().unwrap_or("");

        // If operator configured allowed_orgs, use those instead of (or merged with) user orgs
        let orgs_str: String;
        if let Some(allowed) = config["allowed_orgs"].as_array() {
            let allowed_orgs: Vec<&str> = allowed.iter().filter_map(|v| v.as_str()).collect();
            if !allowed_orgs.is_empty() {
                // Operator restriction: only sync from these orgs
                orgs_str = allowed_orgs.join(", ");
            } else {
                orgs_str = user_orgs_str.to_string();
            }
        } else {
            orgs_str = user_orgs_str.to_string();
        }
        let orgs_str = orgs_str.as_str();

        if username.is_empty() {
            // Auto-detect username via token
            if let Ok(status) = self.test_connection(client, token, config).await
                && let Some(login) = status.username
            {
                tracing::info!(login = %login, "Auto-detected GitHub username");
                return self
                    .sync_with_username(
                        client, token, config, &GitHubUser { username: &login, orgs_str }, start_date, end_date,
                    )
                    .await;
            }
            return Err(AppError::BadRequest(
                "GitHub username is required. Set it in the integration settings.".to_string(),
            ));
        }

        self.sync_with_username(
            client, token, config, &GitHubUser { username, orgs_str }, start_date, end_date,
        )
        .await
    }

    async fn test_connection(
        &self,
        client: &reqwest::Client,
        token: &str,
        _config: &serde_json::Value,
    ) -> Result<ConnectionStatus, AppError> {
        let query = r#"{ "query": "query { viewer { login } }" }"#;

        let resp = client
            .post("https://api.github.com/graphql")
            .header("Authorization", format!("bearer {}", token))
            .header("User-Agent", "brag-frog")
            .header("Content-Type", "application/json")
            .body(query)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Ok(ConnectionStatus {
                connected: false,
                username: None,
                error: Some(format!("HTTP {}", resp.status())),
            });
        }

        let body: GraphQLResponse = resp.json().await?;

        if let Some(data) = body.data {
            let login = data["viewer"]["login"].as_str().map(|s| s.to_string());
            Ok(ConnectionStatus {
                connected: true,
                username: login,
                error: None,
            })
        } else {
            Ok(ConnectionStatus {
                connected: false,
                username: None,
                error: body
                    .errors
                    .map(|e| format!("{:?}", e))
                    .or(Some("Unknown error".to_string())),
            })
        }
    }
}

impl GitHubSync {
    async fn sync_with_username(
        &self,
        client: &reqwest::Client,
        token: &str,
        config: &serde_json::Value,
        user: &GitHubUser<'_>,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<SyncedEntry>, AppError> {
        let public_only = config["public_only"].as_bool().unwrap_or(false);

        // Read sync toggles from config (default: all ON)
        let sync_authored = config["sync_pr_authored"].as_bool().unwrap_or(true);
        let sync_merged = config["sync_pr_merged"].as_bool().unwrap_or(true);
        let sync_reviewed = config["sync_pr_reviewed"].as_bool().unwrap_or(true);
        let sync_pr_development = config["sync_pr_development"].as_bool().unwrap_or(true);

        // Split comma-separated orgs, trimming whitespace
        let orgs: Vec<&str> = user.orgs_str
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        // Collect which query types are enabled
        let mut query_types = Vec::new();
        if sync_authored {
            query_types.push(GitHubQueryType::Authored);
        }
        if sync_merged {
            query_types.push(GitHubQueryType::Merged);
        }
        if sync_reviewed {
            query_types.push(GitHubQueryType::Reviewed);
        }

        let mut entries = Vec::new();

        for query_type in &query_types {
            if orgs.is_empty() {
                let prs = fetch_prs(&FetchPrsParams {
                    client,
                    token,
                    username: user.username,
                    org: "",
                    start_date,
                    end_date,
                    query_type,
                    public_only,
                    sync_pr_development,
                })
                .await?;
                entries.extend(prs);
            } else {
                for org in &orgs {
                    let prs = fetch_prs(&FetchPrsParams {
                        client,
                        token,
                        username: user.username,
                        org,
                        start_date,
                        end_date,
                        query_type,
                        public_only,
                        sync_pr_development,
                    })
                    .await?;
                    entries.extend(prs);
                }
            }
        }

        // Deduplicate by source_id (a PR could appear in multiple org searches)
        entries.sort_by(|a, b| a.source_id.cmp(&b.source_id));
        entries.dedup_by(|a, b| a.source_id == b.source_id);

        Ok(entries)
    }
}

// Escapes backslashes and double quotes for safe interpolation into a
// GraphQL search query string embedded in a JSON literal.
fn escape_graphql_search(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Parameters for a single GitHub PR search query.
struct FetchPrsParams<'a> {
    client: &'a reqwest::Client,
    token: &'a str,
    username: &'a str,
    org: &'a str,
    start_date: NaiveDate,
    end_date: NaiveDate,
    query_type: &'a GitHubQueryType,
    public_only: bool,
    sync_pr_development: bool,
}

// Executes a single GitHub GraphQL search for PRs matching `query_type`,
// scoped to `username` and optionally an `org`, within the date range.
// Returns up to 100 results (GitHub search API limit per page).
//
// Security: the query deliberately omits the `body` field to prevent
// PR descriptions (which may contain security-sensitive content) from
// being stored in the database.
async fn fetch_prs(p: &FetchPrsParams<'_>) -> Result<Vec<SyncedEntry>, AppError> {
    let safe_username = escape_graphql_search(p.username);
    let safe_org = escape_graphql_search(p.org);

    let org_filter = if p.org.is_empty() {
        String::new()
    } else {
        format!("org:{} ", safe_org)
    };

    let visibility_filter = if p.public_only { "is:public " } else { "" };

    let search_query = match p.query_type {
        GitHubQueryType::Authored => format!(
            "is:pr {}{}author:{} created:{}..{}",
            visibility_filter, org_filter, safe_username, p.start_date, p.end_date
        ),
        GitHubQueryType::Merged => format!(
            "is:pr {}{}author:{} merged:{}..{}",
            visibility_filter, org_filter, safe_username, p.start_date, p.end_date
        ),
        GitHubQueryType::Reviewed => format!(
            "is:pr {}{}reviewed-by:{} -author:{} updated:{}..{}",
            visibility_filter, org_filter, safe_username, safe_username, p.start_date, p.end_date
        ),
    };

    tracing::info!(query = %search_query, "GitHub search query");

    // NOTE: `body` is intentionally excluded from the PullRequest fragment.
    // PR descriptions may contain security-sensitive information (vulnerability
    // details, private code snippets, security review discussion) that should
    // not be stored in Brag Frog.
    // Include commits connection for Authored queries when development tracking is enabled
    let commits_fragment = if p.sync_pr_development && matches!(p.query_type, GitHubQueryType::Authored)
    {
        r#"commits(last: 100) {
                                nodes { commit { committedDate } }
                            }"#
    } else {
        ""
    };

    let graphql_query = serde_json::json!({
        "query": format!(r#"
            query {{
                search(query: "{}", type: ISSUE, first: 100) {{
                    issueCount
                    nodes {{
                        ... on PullRequest {{
                            number
                            title
                            url
                            state
                            createdAt
                            updatedAt
                            mergedAt
                            repository {{
                                nameWithOwner
                            }}
                            {}
                        }}
                    }}
                }}
            }}
        "#, search_query, commits_fragment)
    });

    let resp = p.client
        .post("https://api.github.com/graphql")
        .header("Authorization", format!("bearer {}", p.token))
        .header("User-Agent", "brag-frog")
        .json(&graphql_query)
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!("GitHub API error: {}", body)));
    }

    let raw_body = resp.text().await.unwrap_or_default();
    tracing::debug!(response = %raw_body, "GitHub GraphQL raw response");
    let body: GraphQLResponse = serde_json::from_str(&raw_body)
        .map_err(|e| AppError::Internal(format!("Failed to parse GitHub response: {}", e)))?;
    let mut entries = Vec::new();

    if let Some(data) = body.data {
        let total = data["search"]["issueCount"].as_i64().unwrap_or(0);
        tracing::info!(total, query_type = ?p.query_type, "GitHub search returned results");

        if let Some(nodes) = data["search"]["nodes"].as_array() {
            for node in nodes {
                // Skip empty nodes (Issues matched by type:ISSUE but filtered
                // by the PullRequest fragment return as {})
                let number = match node["number"].as_i64() {
                    Some(n) if n > 0 => n,
                    _ => continue,
                };
                let title = node["title"].as_str().unwrap_or("").to_string();
                if title.is_empty() {
                    continue;
                }

                let url = node["url"].as_str().unwrap_or("").to_string();
                let state = node["state"].as_str().unwrap_or("OPEN").to_string();
                let repo = node["repository"]["nameWithOwner"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                let created_at = node["createdAt"].as_str().unwrap_or("").to_string();
                let updated_at = node["updatedAt"].as_str().unwrap_or("").to_string();
                let merged_at = node["mergedAt"].as_str().unwrap_or("").to_string();

                match p.query_type {
                    GitHubQueryType::Authored => {
                        let occurred_at = parse_date(&created_at, &p.start_date);

                        entries.push(SyncedEntry {
                            source: "github",
                            source_id: format!("{}#PR{}", repo, number),
                            source_url: Some(url.clone()),
                            title: title.clone(),
                            description: None,
                            entry_type: "pr_authored",
                            status: Some(state.clone()),
                            repository: Some(repo.clone()),
                            occurred_at,
                            meeting_role: None,
                            recurring_group: None,
                            start_time: None,
                            end_time: None,
                        });

                        // Generate per-day development entries from commit dates
                        if p.sync_pr_development
                            && let Some(commit_nodes) = node["commits"]["nodes"].as_array() {
                                let mut dev_dates = std::collections::BTreeSet::new();
                                for cn in commit_nodes {
                                    if let Some(date_str) = cn["commit"]["committedDate"].as_str() {
                                        let date = parse_date(date_str, &p.start_date);
                                        // Filter to phase range
                                        if date >= p.start_date.to_string()
                                            && date <= p.end_date.to_string()
                                        {
                                            dev_dates.insert(date);
                                        }
                                    }
                                }
                                for date in dev_dates {
                                    entries.push(SyncedEntry {
                                        source: "github",
                                        source_id: format!("{}#PR{}:dev:{}", repo, number, date),
                                        source_url: Some(url.clone()),
                                        title: title.clone(),
                                        description: None,
                                        entry_type: "pr_development",
                                        status: Some(state.clone()),
                                        repository: Some(repo.clone()),
                                        occurred_at: date,
                                        meeting_role: None,
                                        recurring_group: None,
                                        start_time: None,
                                        end_time: None,
                                    });
                                }
                            }
                    }
                    GitHubQueryType::Merged => {
                        if merged_at.is_empty() {
                            continue;
                        }
                        let occurred_at = parse_date(&merged_at, &p.start_date);

                        entries.push(SyncedEntry {
                            source: "github",
                            source_id: format!("{}#PR{}:merged", repo, number),
                            source_url: Some(url),
                            title,
                            description: None,
                            entry_type: "pr_merged",
                            status: Some(state),
                            repository: Some(repo),
                            occurred_at,
                            meeting_role: None,
                            recurring_group: None,
                            start_time: None,
                            end_time: None,
                        });
                    }
                    GitHubQueryType::Reviewed => {
                        let occurred_at = parse_date(&updated_at, &p.start_date);

                        entries.push(SyncedEntry {
                            source: "github",
                            source_id: format!("{}#PR{}", repo, number),
                            source_url: Some(url),
                            title,
                            description: None,
                            entry_type: "pr_reviewed",
                            status: Some(state),
                            repository: Some(repo),
                            occurred_at,
                            meeting_role: None,
                            recurring_group: None,
                            start_time: None,
                            end_time: None,
                        });
                    }
                }
            }
        }
    } else if let Some(errors) = body.errors {
        tracing::error!(?errors, "GitHub GraphQL errors");
    }

    Ok(entries)
}

/// Extracts the `YYYY-MM-DD` date from an ISO 8601 timestamp, falling back to start_date.
fn parse_date(iso_timestamp: &str, fallback: &NaiveDate) -> String {
    iso_timestamp
        .split('T')
        .next()
        .unwrap_or(&fallback.to_string())
        .to_string()
}
