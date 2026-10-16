use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use chrono::NaiveDate;

use super::{ConnectionStatus, SyncService, SyncedEntry};
use crate::kernel::error::AppError;

/// Syncs pages created by the user from Confluence's REST API via CQL search.
pub struct ConfluenceSync;

#[async_trait]
impl SyncService for ConfluenceSync {
    async fn sync(
        &self,
        client: &reqwest::Client,
        token: &str,
        config: &serde_json::Value,
        start_date: NaiveDate,
        _end_date: NaiveDate,
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

        let public_only = config["public_only"].as_bool().unwrap_or(false);

        // Escape single quotes in email to prevent CQL injection
        let safe_email = email.replace('\'', "''");
        let mut cql = format!("creator = '{}' AND created >= '{}'", safe_email, start_date);

        // In public-only mode, the API token's permissions already limit what's returned.
        // No additional CQL filtering needed — space.type is categorization, not security.
        let _ = public_only;

        // Operator-configured space filter from services.toml
        if let Some(allowed) = config["allowed_confluence_spaces"].as_array() {
            let spaces: Vec<&str> = allowed.iter().filter_map(|v| v.as_str()).collect();
            if !spaces.is_empty() {
                let space_list = spaces
                    .iter()
                    .map(|s| format!("\"{}\"", s.replace('"', "\\\"")))
                    .collect::<Vec<_>>()
                    .join(", ");
                cql.push_str(&format!(" AND space in ({})", space_list));
            }
        }

        let resp = client
            .get(format!("{}/wiki/rest/api/content/search", base_url))
            .header("Authorization", format!("Basic {}", auth))
            .query(&[
                ("cql", cql.as_str()),
                ("limit", "100"),
                ("expand", "space,version"),
            ])
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "Confluence API error: {}",
                body
            )));
        }

        let body: serde_json::Value = resp.json().await?;
        let mut entries = Vec::new();

        if let Some(results) = body["results"].as_array() {
            for page in results {
                let id = page["id"].as_str().unwrap_or("").to_string();
                let title = page["title"].as_str().unwrap_or("").to_string();
                let space_name = page["space"]["name"].as_str().unwrap_or("").to_string();
                let created = page["version"]["when"]
                    .as_str()
                    .unwrap_or("")
                    .split('T')
                    .next()
                    .unwrap_or(&start_date.to_string())
                    .to_string();

                let web_link = page["_links"]["webui"]
                    .as_str()
                    .map(|link| format!("{}/wiki{}", base_url, link));

                entries.push(SyncedEntry {
                    source: "confluence",
                    source_id: format!("confluence-{}", id),
                    source_url: web_link,
                    title,
                    description: Some(format!("Space: {}", space_name)),
                    entry_type: "confluence_page",
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
            .get(format!("{}/wiki/rest/api/user/current", base_url))
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
