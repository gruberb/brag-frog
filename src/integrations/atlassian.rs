use async_trait::async_trait;
use chrono::NaiveDate;

use super::confluence::ConfluenceSync;
use super::jira::JiraSync;
use super::{ConnectionStatus, SyncService, SyncedEntry};
use crate::kernel::error::AppError;

/// Composite sync that delegates to `JiraSync` + `ConfluenceSync` under a single Atlassian API token.
pub struct AtlassianSync;

#[async_trait]
impl SyncService for AtlassianSync {
    async fn sync(
        &self,
        client: &reqwest::Client,
        token: &str,
        config: &serde_json::Value,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<SyncedEntry>, AppError> {
        let mut entries = Vec::new();

        // Sync Jira issues
        match JiraSync
            .sync(client, token, config, start_date, end_date)
            .await
        {
            Ok(jira_entries) => {
                tracing::info!(count = jira_entries.len(), "Jira sync returned entries");
                entries.extend(jira_entries);
            }
            Err(e) => {
                tracing::error!(error = %e, "Jira sync failed within Atlassian sync");
                return Err(e);
            }
        }

        // Sync Confluence pages
        match ConfluenceSync
            .sync(client, token, config, start_date, end_date)
            .await
        {
            Ok(confluence_entries) => {
                tracing::info!(
                    count = confluence_entries.len(),
                    "Confluence sync returned entries"
                );
                entries.extend(confluence_entries);
            }
            Err(e) => {
                tracing::error!(error = %e, "Confluence sync failed within Atlassian sync");
                return Err(e);
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
        // Same Atlassian API token — test via Jira
        JiraSync.test_connection(client, token, config).await
    }
}
