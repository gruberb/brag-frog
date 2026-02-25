use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use chrono::NaiveDate;

use super::{ConnectionStatus, SyncService, SyncedEntry};
use crate::kernel::error::AppError;

/// Syncs Jira issues via the v3 REST API using JQL filtered by assignee/creator and date range.
/// Maps issue types (bug, story, task, epic) to BragEntry entry types.
pub struct JiraSync;

#[async_trait]
impl SyncService for JiraSync {
    async fn sync(
        &self,
        client: &reqwest::Client,
        token: &str,
        config: &serde_json::Value,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<SyncedEntry>, AppError> {
        let email = config["email"].as_str().unwrap_or("");
        let base_url = config["base_url"]
            .as_str()
            .filter(|s| !s.is_empty())
            .unwrap_or(
                &crate::integrations::services_config::get()
                    .atlassian
                    .default_base_url,
            );

        super::validate_base_url(base_url)?;

        let auth = BASE64.encode(format!("{}:{}", email, token));

        // Escape single quotes in email to prevent JQL injection
        let safe_email = email.replace('\'', "''");
        let mut jql = format!(
            "(assignee = '{}' OR creator = '{}') AND updated >= '{}' AND updated <= '{}'",
            safe_email, safe_email, start_date, end_date
        );

        let public_only = config["public_only"].as_bool().unwrap_or(false);

        // In public-only mode, exclude security-sensitive projects via JQL.
        // Configured in services.toml as excluded_jira_projects.
        if public_only
            && let Some(excluded) = config["excluded_jira_projects"].as_array()
        {
            let projects: Vec<&str> = excluded.iter().filter_map(|v| v.as_str()).collect();
            if !projects.is_empty() {
                let project_list = projects
                    .iter()
                    .map(|p| format!("\"{}\"", p.replace('"', "\\\"")))
                    .collect::<Vec<_>>()
                    .join(", ");
                jql.push_str(&format!(" AND project not in ({})", project_list));
            }
        }

        // Operator-configured project filter from services.toml
        if let Some(allowed) = config["allowed_jira_projects"].as_array() {
            let projects: Vec<&str> = allowed.iter().filter_map(|v| v.as_str()).collect();
            if !projects.is_empty() {
                let project_list = projects
                    .iter()
                    .map(|p| format!("\"{}\"", p.replace('"', "\\\"")))
                    .collect::<Vec<_>>()
                    .join(", ");
                jql.push_str(&format!(" AND project in ({})", project_list));
            }
        }

        let resp = client
            .post(format!("{}/rest/api/3/search/jql", base_url))
            .header("Authorization", format!("Basic {}", auth))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "jql": jql,
                "maxResults": 100,
                "fields": ["summary", "status", "created", "updated", "issuetype", "project"]
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!("Jira API error: {}", body)));
        }

        let body: serde_json::Value = resp.json().await?;
        let mut entries = Vec::new();

        // Read sync toggles from config
        let sync_completed = config["sync_jira_completed"].as_bool().unwrap_or(true);
        let sync_in_progress = config["sync_jira_in_progress"].as_bool().unwrap_or(true);
        let sync_created = config["sync_jira_created"].as_bool().unwrap_or(false);

        if let Some(issues) = body["issues"].as_array() {
            for issue in issues {
                let key = issue["key"].as_str().unwrap_or("").to_string();
                let fields = &issue["fields"];
                let summary = fields["summary"].as_str().unwrap_or("").to_string();
                let status = fields["status"]["name"].as_str().unwrap_or("").to_string();
                let created = fields["created"]
                    .as_str()
                    .unwrap_or("")
                    .split('T')
                    .next()
                    .unwrap_or(&start_date.to_string())
                    .to_string();
                let project_name = fields["project"]["name"].as_str().unwrap_or("").to_string();
                let issue_type = fields["issuetype"]["name"]
                    .as_str()
                    .unwrap_or("")
                    .to_lowercase();

                // Use Jira's statusCategory to reliably determine issue state
                // Categories: "new" = To Do, "indeterminate" = In Progress, "done" = Done
                let status_category = fields["status"]["statusCategory"]["key"]
                    .as_str()
                    .unwrap_or("undefined")
                    .to_lowercase();

                let is_done = status_category == "done";
                let is_in_progress = status_category == "indeterminate";

                // Filter based on sync settings
                let include = sync_created
                    || (sync_completed && is_done)
                    || (sync_in_progress && is_in_progress);
                if !include {
                    continue;
                }

                let entry_type = match issue_type.as_str() {
                    "bug" => {
                        if is_done {
                            "bug_fixed"
                        } else {
                            "bug_filed"
                        }
                    }
                    "story" => "jira_story",
                    "task" | "sub-task" => "jira_task",
                    "epic" => "jira_epic",
                    _ => {
                        if is_done {
                            "jira_completed"
                        } else {
                            "jira_task"
                        }
                    }
                };

                entries.push(SyncedEntry {
                    source: "jira",
                    source_id: format!("jira-{}", key),
                    source_url: Some(format!("{}/browse/{}", base_url, key)),
                    title: format!("[{}] {}", key, summary),
                    description: Some(format!("Project: {}", project_name)),
                    entry_type,
                    status: Some(status),
                    repository: None,
                    occurred_at: created,
                    meeting_role: None,
                    recurring_group: None,
                    start_time: None,
                    end_time: None,
                });
            }
        }

        Ok(entries)
    }

    async fn test_connection(
        &self,
        client: &reqwest::Client,
        token: &str,
        config: &serde_json::Value,
    ) -> Result<ConnectionStatus, AppError> {
        let email = config["email"].as_str().unwrap_or("");
        let base_url = config["base_url"]
            .as_str()
            .filter(|s| !s.is_empty())
            .unwrap_or(
                &crate::integrations::services_config::get()
                    .atlassian
                    .default_base_url,
            );

        super::validate_base_url(base_url)?;

        let auth = BASE64.encode(format!("{}:{}", email, token));

        let resp = client
            .get(format!("{}/rest/api/3/myself", base_url))
            .header("Authorization", format!("Basic {}", auth))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Ok(ConnectionStatus {
                connected: false,
                username: None,
                error: Some(format!("HTTP {}", resp.status())),
            });
        }

        let body: serde_json::Value = resp.json().await?;

        Ok(ConnectionStatus {
            connected: true,
            username: body["displayName"].as_str().map(|s| s.to_string()),
            error: None,
        })
    }
}
