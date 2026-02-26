use async_trait::async_trait;
use chrono::NaiveDate;

use super::{ConnectionStatus, SyncService, SyncedEntry};
use crate::kernel::error::AppError;

/// Syncs bugs from Bugzilla's REST API: assigned (fixed/filed) and creator-filed bugs.
pub struct BugzillaSync;

// Queries the Bugzilla REST `/rest/bug` endpoint with the given params.
// Returns the `bugs` array from the JSON response.
async fn fetch_bugs(
    client: &reqwest::Client,
    base_url: &str,
    token: &str,
    params: &[(&str, &str)],
) -> Result<Vec<serde_json::Value>, AppError> {
    let mut request = client.get(format!("{}/rest/bug", base_url)).query(params);

    // Only add auth header when token is non-empty
    if !token.is_empty() {
        request = request.header("X-BUGZILLA-API-KEY", token);
    }

    let resp = request.send().await?;

    let body: serde_json::Value = resp.json().await?;

    Ok(body["bugs"].as_array().cloned().unwrap_or_default())
}

#[async_trait]
impl SyncService for BugzillaSync {
    async fn sync(
        &self,
        client: &reqwest::Client,
        token: &str,
        config: &serde_json::Value,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<SyncedEntry>, AppError> {
        let public_only = config["public_only"].as_bool().unwrap_or(false);
        let email = config["email"].as_str().unwrap_or("");
        let base_url = config["base_url"]
            .as_str()
            .filter(|s| !s.is_empty())
            .unwrap_or(
                &crate::integrations::services_config::get()
                    .bugzilla
                    .default_base_url,
            );

        super::validate_base_url(base_url)?;

        // In public-only mode, strip the token so Bugzilla only returns publicly visible bugs
        let token = if public_only { "" } else { token };

        let start_str = start_date.to_string();
        let end_str = end_date.to_string();

        let mut entries = Vec::new();

        // Collect allowed products filter from services.toml
        let allowed_products: Vec<String> = config["allowed_products"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let mut assigned_params: Vec<(&str, &str)> = vec![
            ("assigned_to", email),
            ("last_change_time", start_str.as_str()),
            ("creation_time_end", end_str.as_str()),
            (
                "include_fields",
                "id,summary,status,resolution,creation_time,last_change_time,product,component",
            ),
            ("limit", "200"),
        ];
        for product in &allowed_products {
            assigned_params.push(("product", product.as_str()));
        }

        // Fetch bugs assigned to user that were active during the phase
        let assigned_bugs = fetch_bugs(client, base_url, token, &assigned_params).await?;

        for bug in &assigned_bugs {
            let id = bug["id"].as_i64().unwrap_or(0);
            let summary = bug["summary"].as_str().unwrap_or("").to_string();
            let status = bug["status"].as_str().unwrap_or("").to_string();
            let resolution = bug["resolution"].as_str().unwrap_or("");
            let created = bug["creation_time"]
                .as_str()
                .unwrap_or("")
                .split('T')
                .next()
                .unwrap_or(&start_date.to_string())
                .to_string();
            let product = bug["product"].as_str().unwrap_or("").to_string();
            let component = bug["component"].as_str().unwrap_or("").to_string();

            let entry_type = if status == "RESOLVED" && resolution == "FIXED" {
                "bug_fixed"
            } else {
                "bug_filed"
            };

            entries.push(SyncedEntry {
                source: "bugzilla",
                source_id: format!("bug-{}", id),
                source_url: Some(format!("{}/show_bug.cgi?id={}", base_url, id)),
                title: summary,
                description: Some(format!("{} :: {}", product, component)),
                entry_type,
                status: Some(format!("{} {}", status, resolution).trim().to_string()),
                repository: None,
                occurred_at: created,
                meeting_role: None,
                recurring_group: None,
                start_time: None,
                end_time: None,
                collaborators: None,
            });
        }

        // Also fetch bugs filed by user during the phase
        let mut filed_params: Vec<(&str, &str)> = vec![
            ("creator", email),
            ("creation_time", start_str.as_str()),
            ("creation_time_end", end_str.as_str()),
            (
                "include_fields",
                "id,summary,status,resolution,creation_time,product,component",
            ),
            ("limit", "200"),
        ];
        for product in &allowed_products {
            filed_params.push(("product", product.as_str()));
        }

        let filed_bugs = fetch_bugs(client, base_url, token, &filed_params).await?;

        for bug in &filed_bugs {
            let id = bug["id"].as_i64().unwrap_or(0);
            let source_id = format!("bug-filed-{}", id);

            // Skip if already added from assigned bugs
            if entries.iter().any(|e| e.source_id == format!("bug-{}", id)) {
                continue;
            }

            let summary = bug["summary"].as_str().unwrap_or("").to_string();
            let created = bug["creation_time"]
                .as_str()
                .unwrap_or("")
                .split('T')
                .next()
                .unwrap_or(&start_date.to_string())
                .to_string();

            entries.push(SyncedEntry {
                source: "bugzilla",
                source_id,
                source_url: Some(format!("{}/show_bug.cgi?id={}", base_url, id)),
                title: format!("[Filed] {}", summary),
                description: None,
                entry_type: "bug_filed",
                status: None,
                repository: None,
                occurred_at: created,
                meeting_role: None,
                recurring_group: None,
                start_time: None,
                end_time: None,
                collaborators: None,
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
        let base_url = config["base_url"]
            .as_str()
            .filter(|s| !s.is_empty())
            .unwrap_or(
                &crate::integrations::services_config::get()
                    .bugzilla
                    .default_base_url,
            );

        super::validate_base_url(base_url)?;

        // Without a token, test with an unauthenticated request
        if token.is_empty() {
            let email = config["email"].as_str().unwrap_or("");
            if email.is_empty() {
                return Ok(ConnectionStatus {
                    connected: false,
                    username: None,
                    error: Some("Email is required for Bugzilla sync".to_string()),
                });
            }

            let resp = client
                .get(format!("{}/rest/bug", base_url))
                .query(&[("id", "1"), ("include_fields", "id")])
                .send()
                .await?;

            if resp.status().is_success() {
                return Ok(ConnectionStatus {
                    connected: true,
                    username: Some(email.to_string()),
                    error: None,
                });
            } else {
                return Ok(ConnectionStatus {
                    connected: false,
                    username: None,
                    error: Some(format!("HTTP {}", resp.status())),
                });
            }
        }

        let resp = client
            .get(format!("{}/rest/whoami", base_url))
            .header("X-BUGZILLA-API-KEY", token)
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await?;

        if let Some(name) = body["name"].as_str() {
            Ok(ConnectionStatus {
                connected: true,
                username: Some(name.to_string()),
                error: None,
            })
        } else {
            Ok(ConnectionStatus {
                connected: false,
                username: None,
                error: body["message"].as_str().map(|s| s.to_string()),
            })
        }
    }
}
