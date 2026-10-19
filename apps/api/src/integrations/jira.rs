use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use chrono::{DateTime, FixedOffset, NaiveDate};

use super::{ConnectionStatus, SyncService, SyncedEntry};
use crate::kernel::error::AppError;

/// Search pages fetched before a sync gives up. 50 pages of 100 issues is far above any
/// cycle; the limit only stops a server that keeps handing out page tokens. Hitting it is an
/// error rather than a partial result, because a hard sync soft-deletes entries it did not see.
const MAX_SEARCH_PAGES: usize = 50;

const SEARCH_FIELDS: &[&str] = &[
    "summary", "status", "created", "updated", "issuetype", "project", "assignee", "creator",
];

/// Syncs Jira issues via the v3 REST API using JQL filtered by assignee/creator and date range.
/// Maps issue types (bug, story, task, epic) to BragEntry entry types, and adds one
/// `jira_progress` entry per day on which the user changed an issue's status.
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

        // Read sync toggles from config
        let sync_completed = config["sync_jira_completed"].as_bool().unwrap_or(true);
        let sync_in_progress = config["sync_jira_in_progress"].as_bool().unwrap_or(true);
        let sync_created = config["sync_jira_created"].as_bool().unwrap_or(false);
        let sync_progress = config["sync_jira_progress"].as_bool().unwrap_or(true);

        // Changelog authors are matched by accountId: Jira hides emailAddress for users
        // with a private profile, so an email match would silently drop their changes.
        let account_id = if sync_progress {
            Some(fetch_account_id(client, base_url, &auth).await?)
        } else {
            None
        };

        let issues = search_issues(client, base_url, &auth, &jql, sync_progress).await?;
        let mut entries = Vec::new();

        for issue in &issues {
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
            let source_url = format!("{}/browse/{}", base_url, key);

            // Use Jira's statusCategory to reliably determine issue state
            // Categories: "new" = To Do, "indeterminate" = In Progress, "done" = Done
            let status_category = fields["status"]["statusCategory"]["key"]
                .as_str()
                .unwrap_or("undefined")
                .to_lowercase();

            let is_done = status_category == "done";
            let is_in_progress = status_category == "indeterminate";

            // Extract collaborator: assignee or creator (whichever isn't the syncing user)
            let assignee_email = fields["assignee"]["emailAddress"].as_str().unwrap_or("");
            let assignee_name = fields["assignee"]["displayName"].as_str().unwrap_or("");
            let creator_email = fields["creator"]["emailAddress"].as_str().unwrap_or("");
            let creator_name = fields["creator"]["displayName"].as_str().unwrap_or("");

            let collaborator = if !assignee_email.is_empty() && assignee_email != email {
                Some(if !assignee_name.is_empty() { assignee_name } else { assignee_email }.to_string())
            } else if !creator_email.is_empty() && creator_email != email {
                Some(if !creator_name.is_empty() { creator_name } else { creator_email }.to_string())
            } else {
                None
            };

            // Progress entries are independent of the current-status filters below: moving an
            // issue is work done that day even if it later went back to To Do.
            if let Some(account_id) = account_id.as_deref() {
                for (day, to_status) in status_changes_by_day(issue, account_id, start_date, end_date) {
                    entries.push(SyncedEntry {
                        source: "jira",
                        source_id: format!("jira-{}:status:{}", key, day),
                        source_url: Some(source_url.clone()),
                        title: format!("[{}] {} \u{2192} {}", key, summary, to_status),
                        description: Some(format!("Project: {}", project_name)),
                        entry_type: "jira_progress",
                        status: Some(to_status),
                        repository: None,
                        occurred_at: day.to_string(),
                        meeting_role: None,
                        recurring_group: None,
                        start_time: None,
                        end_time: None,
                        collaborators: collaborator.clone(),
                    });
                }
            }

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
                source_url: Some(source_url),
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
                collaborators: collaborator,
            });
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

/// Returns the accountId of the user the token belongs to.
async fn fetch_account_id(
    client: &reqwest::Client,
    base_url: &str,
    auth: &str,
) -> Result<String, AppError> {
    let resp = client
        .get(format!("{}/rest/api/3/myself", base_url))
        .header("Authorization", format!("Basic {}", auth))
        .send()
        .await?;
    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!("Jira API error: {}", body)));
    }
    let body: serde_json::Value = resp.json().await?;
    body["accountId"]
        .as_str()
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .ok_or_else(|| AppError::Internal("Jira /myself returned no accountId".to_string()))
}

/// Runs `jql` through every result page. With `with_changelog`, each issue carries its
/// recent history (Jira includes up to the latest 100 changes per issue in search results).
async fn search_issues(
    client: &reqwest::Client,
    base_url: &str,
    auth: &str,
    jql: &str,
    with_changelog: bool,
) -> Result<Vec<serde_json::Value>, AppError> {
    let mut issues = Vec::new();
    let mut page_token: Option<String> = None;

    for _ in 0..MAX_SEARCH_PAGES {
        let mut body = serde_json::json!({
            "jql": jql,
            "maxResults": 100,
            "fields": SEARCH_FIELDS,
        });
        if with_changelog {
            body["expand"] = "changelog".into();
        }
        if let Some(token) = &page_token {
            body["nextPageToken"] = token.as_str().into();
        }

        let resp = client
            .post(format!("{}/rest/api/3/search/jql", base_url))
            .header("Authorization", format!("Basic {}", auth))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!("Jira API error: {}", body)));
        }

        let mut page: serde_json::Value = resp.json().await?;
        if let serde_json::Value::Array(batch) = page["issues"].take() {
            issues.extend(batch);
        }

        match page["nextPageToken"].as_str() {
            Some(token) if !page["isLast"].as_bool().unwrap_or(false) => {
                page_token = Some(token.to_string());
            }
            _ => return Ok(issues),
        }
    }

    Err(AppError::Internal(format!(
        "Jira search returned more than {} pages; narrow the cycle or project filter",
        MAX_SEARCH_PAGES
    )))
}

/// Days in `[start, end]` on which `account_id` changed the issue's status, each paired with
/// the last status set that day. Several moves on one day (To Do, In Progress, In Review)
/// collapse into one entry so a quick click-through does not flood the logbook.
///
/// Days come from the timestamp's own offset, which Jira renders in the user's profile
/// timezone, matching how `created` is turned into a date.
fn status_changes_by_day(
    issue: &serde_json::Value,
    account_id: &str,
    start: NaiveDate,
    end: NaiveDate,
) -> Vec<(NaiveDate, String)> {
    let mut latest: std::collections::BTreeMap<NaiveDate, (DateTime<FixedOffset>, String)> =
        std::collections::BTreeMap::new();

    let histories = issue["changelog"]["histories"].as_array().map(Vec::as_slice).unwrap_or_default();
    for history in histories {
        if history["author"]["accountId"].as_str() != Some(account_id) {
            continue;
        }
        let Some(at) = history["created"]
            .as_str()
            .and_then(|s| DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f%z").ok())
        else {
            continue;
        };
        let day = at.date_naive();
        if day < start || day > end {
            continue;
        }
        let items = history["items"].as_array().map(Vec::as_slice).unwrap_or_default();
        for item in items.iter().filter(|i| i["field"] == "status") {
            let Some(to_status) = item["toString"].as_str() else {
                continue;
            };
            let slot = latest.entry(day).or_insert_with(|| (at, to_status.to_string()));
            if at >= slot.0 {
                *slot = (at, to_status.to_string());
            }
        }
    }

    latest.into_iter().map(|(day, (_, status))| (day, status)).collect()
}

#[cfg(test)]
mod tests {
    use super::status_changes_by_day;
    use chrono::NaiveDate;
    use serde_json::json;

    const ME: &str = "acct-me";

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn change(author: &str, at: &str, field: &str, to: &str) -> serde_json::Value {
        json!({
            "author": {"accountId": author},
            "created": at,
            "items": [{"field": field, "toString": to}],
        })
    }

    fn issue(histories: Vec<serde_json::Value>) -> serde_json::Value {
        json!({"changelog": {"histories": histories}})
    }

    fn days(issue: &serde_json::Value) -> Vec<(NaiveDate, String)> {
        status_changes_by_day(issue, ME, date("2026-07-01"), date("2026-12-31"))
    }

    #[test]
    fn one_entry_per_day_with_the_last_status() {
        // Histories are deliberately out of order: Jira does not promise an order.
        let issue = issue(vec![
            change(ME, "2026-10-15T13:07:16.289-0400", "status", "In Review"),
            change(ME, "2026-10-15T09:00:00.000-0400", "status", "In Progress"),
            change(ME, "2026-08-03T10:00:00.000-0400", "status", "In Progress"),
        ]);
        assert_eq!(
            days(&issue),
            vec![
                (date("2026-08-03"), "In Progress".to_string()),
                (date("2026-10-15"), "In Review".to_string()),
            ]
        );
    }

    #[test]
    fn ignores_other_authors_other_fields_and_days_outside_the_cycle() {
        let issue = issue(vec![
            change("acct-someone-else", "2026-09-01T10:00:00.000-0400", "status", "Done"),
            change(ME, "2026-09-02T10:00:00.000-0400", "assignee", "Bastian"),
            change(ME, "2026-06-30T10:00:00.000-0400", "status", "In Progress"),
            change(ME, "not a timestamp", "status", "Done"),
        ]);
        assert!(days(&issue).is_empty());
    }

    #[test]
    fn day_follows_the_timestamp_offset() {
        // 23:30 in Halifax is already the next day in UTC; the user's day is what counts.
        let issue = issue(vec![change(ME, "2026-10-15T23:30:00.000-0300", "status", "Done")]);
        assert_eq!(days(&issue), vec![(date("2026-10-15"), "Done".to_string())]);
    }

    #[test]
    fn issue_without_changelog_has_no_progress() {
        assert!(days(&json!({"key": "DISCO-1"})).is_empty());
    }
}
