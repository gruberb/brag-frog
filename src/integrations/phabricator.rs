use async_trait::async_trait;
use chrono::NaiveDate;

use super::{ConnectionStatus, SyncService, SyncedEntry};
use crate::kernel::error::AppError;

/// Syncs authored revisions from Phabricator's Conduit API (`differential.revision.search`).
pub struct PhabricatorSync;

#[async_trait]
impl SyncService for PhabricatorSync {
    async fn sync(
        &self,
        client: &reqwest::Client,
        token: &str,
        config: &serde_json::Value,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<SyncedEntry>, AppError> {
        let base_url = config["base_url"]
            .as_str()
            .filter(|s| !s.is_empty())
            .unwrap_or(
                &crate::integrations::services_config::get()
                    .phabricator
                    .default_base_url,
            );

        super::validate_base_url(base_url)?;

        // First get user PHID
        let user_phid = get_user_phid(client, token, base_url).await?;

        let start_ts = start_date
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        let end_ts = end_date
            .and_hms_opt(23, 59, 59)
            .unwrap()
            .and_utc()
            .timestamp();

        let public_only = config["public_only"].as_bool().unwrap_or(false);

        // Search authored revisions
        let mut form_params = vec![
            ("api.token", token.to_string()),
            ("constraints[authorPHIDs][0]", user_phid),
            ("constraints[createdStart]", start_ts.to_string()),
            ("constraints[createdEnd]", end_ts.to_string()),
            ("order", "newest".to_string()),
            ("limit", "100".to_string()),
        ];

        // In public-only mode, request policy attachments to filter by view policy
        if public_only {
            form_params.push(("attachments[policy]", "1".to_string()));
        }

        // Request project attachments for operator-configured project filtering
        let allowed_projects: Vec<String> = config["allowed_projects"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        if !allowed_projects.is_empty() {
            form_params.push(("attachments[projects]", "1".to_string()));
        }

        let resp = client
            .post(format!("{}/api/differential.revision.search", base_url))
            .form(&form_params)
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await?;
        let mut entries = Vec::new();

        if let Some(data) = body["result"]["data"].as_array() {
            for rev in data {
                // In public-only mode, skip revisions with restricted view policies
                if public_only {
                    let view_policy = rev["attachments"]["policy"]["view"]["value"]
                        .as_str()
                        .unwrap_or("");
                    // Exclude admin, no-one, and custom restricted policies (PHID-PLCY-*)
                    // Allow public, users, and project/team-based policies (PHID-PROJ-*)
                    if view_policy == "admin"
                        || view_policy == "no-one"
                        || view_policy.starts_with("PHID-PLCY-")
                    {
                        continue;
                    }
                }

                // Filter by allowed project tags if configured
                if !allowed_projects.is_empty() {
                    let project_slugs: Vec<&str> = rev["attachments"]["projects"]["projectPHIDs"]
                        .as_array()
                        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                        .unwrap_or_default();
                    // Check if any of the revision's projects match allowed list
                    // Note: Phabricator returns PHIDs, but project tags/slugs are in the
                    // revision's fields. We compare against the short code from the title prefix.
                    let slug_from_id = rev["fields"]["repositoryPHID"].as_str().unwrap_or("");
                    let has_match = project_slugs.iter().any(|phid| {
                        allowed_projects
                            .iter()
                            .any(|allowed| phid.contains(allowed))
                    }) || allowed_projects.iter().any(|allowed| {
                        // Also check if the revision title starts with the project tag
                        rev["fields"]["title"]
                            .as_str()
                            .unwrap_or("")
                            .to_uppercase()
                            .starts_with(&allowed.to_uppercase())
                    });
                    let _ = slug_from_id; // suppress unused warning
                    if !has_match {
                        continue;
                    }
                }

                let id = rev["id"].as_i64().unwrap_or(0);
                let fields = &rev["fields"];
                let title = fields["title"].as_str().unwrap_or("").to_string();
                let status = fields["status"]["name"].as_str().unwrap_or("").to_string();
                let created = fields["dateCreated"].as_i64().unwrap_or(0);

                let occurred_at = chrono::DateTime::from_timestamp(created, 0)
                    .map(|dt| dt.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| start_date.to_string());

                entries.push(SyncedEntry {
                    source: "phabricator",
                    source_id: format!("D{}", id),
                    source_url: Some(format!("{}/D{}", base_url, id)),
                    title,
                    description: None,
                    entry_type: "revision_authored",
                    status: Some(status),
                    repository: None,
                    occurred_at,
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
        let base_url = config["base_url"]
            .as_str()
            .filter(|s| !s.is_empty())
            .unwrap_or(
                &crate::integrations::services_config::get()
                    .phabricator
                    .default_base_url,
            );

        super::validate_base_url(base_url)?;

        let resp = client
            .post(format!("{}/api/user.whoami", base_url))
            .form(&[("api.token", token)])
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await?;

        if let Some(username) = body["result"]["userName"].as_str() {
            Ok(ConnectionStatus {
                connected: true,
                username: Some(username.to_string()),
                error: None,
            })
        } else {
            Ok(ConnectionStatus {
                connected: false,
                username: None,
                error: body["error_info"].as_str().map(|s| s.to_string()),
            })
        }
    }
}

// Resolves the authenticated user's PHID via `user.whoami` for use in revision queries.
async fn get_user_phid(
    client: &reqwest::Client,
    token: &str,
    base_url: &str,
) -> Result<String, AppError> {
    let resp = client
        .post(format!("{}/api/user.whoami", base_url))
        .form(&[("api.token", token)])
        .send()
        .await?;

    let body: serde_json::Value = resp.json().await?;
    body["result"]["phid"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::Internal("Could not get user PHID from Phabricator".to_string()))
}
