use super::common;

use async_trait::async_trait;
use chrono::NaiveDate;

use brag_frog::entries::model::BragEntry;
use brag_frog::shared::crypto::Crypto;
use brag_frog::shared::error::AppError;
use brag_frog::sync::model::{IntegrationConfig, SyncLog};
use brag_frog::sync::{ConnectionStatus, SyncService, SyncedEntry};

/// Mock sync service that returns a configurable list of entries.
struct MockSyncService {
    entries: Vec<SyncedEntry>,
}

impl MockSyncService {
    fn new(entries: Vec<SyncedEntry>) -> Box<Self> {
        Box::new(Self { entries })
    }
}

#[async_trait]
impl SyncService for MockSyncService {
    async fn sync(
        &self,
        _client: &reqwest::Client,
        _token: &str,
        _config: &serde_json::Value,
        _start_date: NaiveDate,
        _end_date: NaiveDate,
    ) -> Result<Vec<SyncedEntry>, AppError> {
        Ok(self.entries.clone())
    }

    async fn test_connection(
        &self,
        _client: &reqwest::Client,
        _token: &str,
        _config: &serde_json::Value,
    ) -> Result<ConnectionStatus, AppError> {
        Ok(ConnectionStatus {
            connected: true,
            username: Some("mock-user".to_string()),
            error: None,
        })
    }
}

/// Mock sync service that always returns an error.
struct FailingSyncService {
    error_msg: String,
}

#[async_trait]
impl SyncService for FailingSyncService {
    async fn sync(
        &self,
        _client: &reqwest::Client,
        _token: &str,
        _config: &serde_json::Value,
        _start_date: NaiveDate,
        _end_date: NaiveDate,
    ) -> Result<Vec<SyncedEntry>, AppError> {
        Err(AppError::Internal(self.error_msg.clone()))
    }

    async fn test_connection(
        &self,
        _client: &reqwest::Client,
        _token: &str,
        _config: &serde_json::Value,
    ) -> Result<ConnectionStatus, AppError> {
        Ok(ConnectionStatus {
            connected: false,
            username: None,
            error: Some(self.error_msg.clone()),
        })
    }
}

fn make_entry(
    source_id: &str,
    title: &str,
    entry_type: &'static str,
    occurred_at: &str,
) -> SyncedEntry {
    SyncedEntry {
        source: "github",
        source_id: source_id.to_string(),
        source_url: Some(format!("https://github.com/test/{}", source_id)),
        title: title.to_string(),
        description: None,
        entry_type,
        status: None,
        repository: Some("test-repo".to_string()),
        occurred_at: occurred_at.to_string(),
        meeting_role: None,
        recurring_group: None,
        start_time: None,
        end_time: None,
    }
}

/// Set up a user + phase + integration, returning (pool, crypto, user_id, phase_id).
async fn setup_sync_test() -> (sqlx::SqlitePool, Crypto, i64, i64) {
    let pool = common::test_pool().await;
    let crypto = common::test_crypto();
    let user_id = common::create_test_user(&pool).await;
    let phase_id = common::create_test_phase(&pool, user_id).await;

    // Create an enabled integration with an encrypted token
    let user_crypto = crypto.for_user(user_id).unwrap();
    let encrypted_token = user_crypto.encrypt("fake-token-12345").unwrap();
    IntegrationConfig::upsert(&pool, user_id, "github", true, Some(&encrypted_token), None)
        .await
        .unwrap();

    (pool, crypto, user_id, phase_id)
}

// ── Basic sync ──

#[tokio::test]
async fn test_sync_creates_entries() {
    let (pool, crypto, user_id, phase_id) = setup_sync_test().await;
    let http_client = reqwest::Client::new();

    let mock = MockSyncService::new(vec![
        make_entry("gh-1", "Fix login bug", "pr_authored", "2025-03-10"),
        make_entry("gh-2", "Add tests", "pr_authored", "2025-03-11"),
        make_entry("gh-3", "Review docs", "pr_reviewed", "2025-03-12"),
    ]);

    let result = brag_frog::sync::run_sync_with_service(
        &pool,
        &crypto,
        &http_client,
        user_id,
        "github",
        mock,
    )
    .await
    .unwrap();

    assert_eq!(result.log.entries_created, 3);
    assert_eq!(result.log.entries_fetched, 3);
    assert_eq!(result.log.status, "success");

    // Verify entries in DB
    let user_crypto = crypto.for_user(user_id).unwrap();
    let entries = BragEntry::list_for_phase(&pool, phase_id, &user_crypto)
        .await
        .unwrap();
    assert_eq!(entries.len(), 3);
}

#[tokio::test]
async fn test_sync_empty_result() {
    let (pool, crypto, user_id, _phase_id) = setup_sync_test().await;
    let http_client = reqwest::Client::new();

    let mock = MockSyncService::new(vec![]);
    let result = brag_frog::sync::run_sync_with_service(
        &pool,
        &crypto,
        &http_client,
        user_id,
        "github",
        mock,
    )
    .await
    .unwrap();

    assert_eq!(result.log.entries_created, 0);
    assert_eq!(result.log.entries_fetched, 0);
    assert_eq!(result.log.status, "success");
}

#[tokio::test]
async fn test_sync_creates_correct_week() {
    let (pool, crypto, user_id, phase_id) = setup_sync_test().await;
    let http_client = reqwest::Client::new();

    // 2025-03-10 is a Monday, ISO week 11
    let mock = MockSyncService::new(vec![make_entry(
        "gh-week",
        "Week test",
        "pr_authored",
        "2025-03-12",
    )]);

    brag_frog::sync::run_sync_with_service(&pool, &crypto, &http_client, user_id, "github", mock)
        .await
        .unwrap();

    // Check that a week was created for ISO week 11
    let weeks = brag_frog::review::model::Week::list_for_phase(&pool, phase_id)
        .await
        .unwrap();
    assert!(!weeks.is_empty());
    let week = weeks.iter().find(|w| w.iso_week == 11 && w.year == 2025);
    assert!(week.is_some(), "Should create week 11 of 2025");
}

// ── Upsert/dedup ──

#[tokio::test]
async fn test_sync_upserts_on_duplicate_source_id() {
    let (pool, crypto, user_id, phase_id) = setup_sync_test().await;
    let http_client = reqwest::Client::new();

    // First sync
    let mock1 = MockSyncService::new(vec![make_entry(
        "gh-dup",
        "Original Title",
        "pr_authored",
        "2025-03-10",
    )]);
    brag_frog::sync::run_sync_with_service(&pool, &crypto, &http_client, user_id, "github", mock1)
        .await
        .unwrap();

    // Second sync with updated title
    let mock2 = MockSyncService::new(vec![make_entry(
        "gh-dup",
        "Updated Title",
        "pr_authored",
        "2025-03-10",
    )]);
    let result2 = brag_frog::sync::run_sync_with_service(
        &pool,
        &crypto,
        &http_client,
        user_id,
        "github",
        mock2,
    )
    .await
    .unwrap();

    // Should be updated, not created
    assert_eq!(result2.log.entries_created, 0);
    assert_eq!(result2.log.entries_updated, 1);

    // Only 1 entry in DB with updated title
    let user_crypto = crypto.for_user(user_id).unwrap();
    let entries = BragEntry::list_for_phase(&pool, phase_id, &user_crypto)
        .await
        .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].title, "Updated Title");
}

#[tokio::test]
async fn test_sync_preserves_user_key_result() {
    let (pool, crypto, user_id, _phase_id) = setup_sync_test().await;
    let http_client = reqwest::Client::new();

    // First sync
    let mock1 = MockSyncService::new(vec![make_entry(
        "gh-kr",
        "PR with KR",
        "pr_authored",
        "2025-03-10",
    )]);
    brag_frog::sync::run_sync_with_service(&pool, &crypto, &http_client, user_id, "github", mock1)
        .await
        .unwrap();

    // Manually assign a key result
    let kr = common::create_test_key_result(&pool, user_id, "My KR", None).await;
    let user_crypto = crypto.for_user(user_id).unwrap();
    let entry = BragEntry::find_by_source(&pool, "github", "gh-kr", user_id, &user_crypto)
        .await
        .unwrap()
        .unwrap();
    sqlx::query("UPDATE brag_entries SET key_result_id = ? WHERE id = ?")
        .bind(kr.id)
        .bind(entry.id)
        .execute(&pool)
        .await
        .unwrap();

    // Re-sync
    let mock2 = MockSyncService::new(vec![make_entry(
        "gh-kr",
        "PR with KR updated",
        "pr_authored",
        "2025-03-10",
    )]);
    brag_frog::sync::run_sync_with_service(&pool, &crypto, &http_client, user_id, "github", mock2)
        .await
        .unwrap();

    // key_result_id should be preserved
    let found = BragEntry::find_by_id(&pool, entry.id, user_id, &user_crypto)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        found.key_result_id,
        Some(kr.id),
        "key_result_id should be preserved"
    );
}

// ── Entry types ──

#[tokio::test]
async fn test_sync_meeting_type() {
    let (pool, crypto, user_id, phase_id) = setup_sync_test().await;
    let http_client = reqwest::Client::new();

    let mock = MockSyncService::new(vec![SyncedEntry {
        source: "google_calendar",
        source_id: "cal-1".to_string(),
        source_url: None,
        title: "Team Standup".to_string(),
        description: None,
        entry_type: "meeting",
        status: None,
        repository: None,
        occurred_at: "2025-03-10".to_string(),
        meeting_role: None,
        recurring_group: None,
        start_time: None,
        end_time: None,
    }]);

    // Need google_calendar integration instead
    let user_crypto = crypto.for_user(user_id).unwrap();
    let encrypted_token = user_crypto.encrypt("fake-cal-token").unwrap();
    IntegrationConfig::upsert(
        &pool,
        user_id,
        "google_calendar",
        true,
        Some(&encrypted_token),
        None,
    )
    .await
    .unwrap();

    brag_frog::sync::run_sync_with_service(
        &pool,
        &crypto,
        &http_client,
        user_id,
        "google_calendar",
        mock,
    )
    .await
    .unwrap();

    let entries = BragEntry::list_for_phase(&pool, phase_id, &user_crypto)
        .await
        .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].entry_type, "meeting");
}

#[tokio::test]
async fn test_sync_drive_types() {
    let (pool, crypto, user_id, phase_id) = setup_sync_test().await;
    let http_client = reqwest::Client::new();

    let user_crypto = crypto.for_user(user_id).unwrap();
    let encrypted_token = user_crypto.encrypt("fake-drive-token").unwrap();
    IntegrationConfig::upsert(
        &pool,
        user_id,
        "google_drive",
        true,
        Some(&encrypted_token),
        None,
    )
    .await
    .unwrap();

    let mock = MockSyncService::new(vec![
        SyncedEntry {
            source: "google_drive",
            source_id: "drive:abc:created:2025-03-10".to_string(),
            source_url: None,
            title: "Design Doc".to_string(),
            description: None,
            entry_type: "drive_created",
            status: None,
            repository: None,
            occurred_at: "2025-03-10".to_string(),
            meeting_role: None,
            recurring_group: None,
            start_time: None,
            end_time: None,
        },
        SyncedEntry {
            source: "google_drive",
            source_id: "drive:def:edited:2025-03-11".to_string(),
            source_url: None,
            title: "Spec Update".to_string(),
            description: None,
            entry_type: "drive_edited",
            status: None,
            repository: None,
            occurred_at: "2025-03-11".to_string(),
            meeting_role: None,
            recurring_group: None,
            start_time: None,
            end_time: None,
        },
    ]);

    brag_frog::sync::run_sync_with_service(
        &pool,
        &crypto,
        &http_client,
        user_id,
        "google_drive",
        mock,
    )
    .await
    .unwrap();

    let entries = BragEntry::list_for_phase(&pool, phase_id, &user_crypto)
        .await
        .unwrap();
    assert_eq!(entries.len(), 2);
    let types: Vec<&str> = entries.iter().map(|e| e.entry_type.as_str()).collect();
    assert!(types.contains(&"drive_created"));
    assert!(types.contains(&"drive_edited"));
}

// ── SyncLog ──

#[tokio::test]
async fn test_sync_log_recorded() {
    let (pool, crypto, user_id, _phase_id) = setup_sync_test().await;
    let http_client = reqwest::Client::new();

    let mock = MockSyncService::new(vec![
        make_entry("gh-log-1", "Entry 1", "pr_authored", "2025-03-10"),
        make_entry("gh-log-2", "Entry 2", "pr_authored", "2025-03-11"),
    ]);

    brag_frog::sync::run_sync_with_service(&pool, &crypto, &http_client, user_id, "github", mock)
        .await
        .unwrap();

    let logs = SyncLog::recent_for_user(&pool, user_id, 10).await.unwrap();
    assert!(!logs.is_empty());
    let log = &logs[0];
    assert_eq!(log.service, "github");
    assert_eq!(log.status, "success");
    assert_eq!(log.entries_created, 2);
    assert_eq!(log.entries_fetched, 2);
}

#[tokio::test]
async fn test_sync_log_records_error() {
    let (pool, crypto, user_id, _phase_id) = setup_sync_test().await;
    let http_client = reqwest::Client::new();

    let mock: Box<dyn SyncService> = Box::new(FailingSyncService {
        error_msg: "API rate limited".to_string(),
    });

    let result = brag_frog::sync::run_sync_with_service(
        &pool,
        &crypto,
        &http_client,
        user_id,
        "github",
        mock,
    )
    .await;

    assert!(result.is_err());

    let logs = SyncLog::recent_for_user(&pool, user_id, 10).await.unwrap();
    assert!(!logs.is_empty());
    let log = &logs[0];
    assert_eq!(log.status, "error");
    assert!(
        log.error_message
            .as_deref()
            .unwrap()
            .contains("API rate limited")
    );
}

// ── Integration config ──

#[tokio::test]
async fn test_sync_requires_enabled_integration() {
    let pool = common::test_pool().await;
    let crypto = common::test_crypto();
    let user_id = common::create_test_user(&pool).await;
    let _phase_id = common::create_test_phase(&pool, user_id).await;
    let http_client = reqwest::Client::new();

    // No integration configured at all
    let mock = MockSyncService::new(vec![]);
    let result = brag_frog::sync::run_sync_with_service(
        &pool,
        &crypto,
        &http_client,
        user_id,
        "github",
        mock,
    )
    .await;

    assert!(result.is_err());
    let err = match result {
        Err(e) => format!("{}", e),
        Ok(_) => panic!("Expected error"),
    };
    assert!(err.contains("not configured") || err.contains("Not found"));
}

#[tokio::test]
async fn test_sync_rejects_disabled_integration() {
    let pool = common::test_pool().await;
    let crypto = common::test_crypto();
    let user_id = common::create_test_user(&pool).await;
    let _phase_id = common::create_test_phase(&pool, user_id).await;
    let http_client = reqwest::Client::new();

    // Create a disabled integration
    let user_crypto = crypto.for_user(user_id).unwrap();
    let encrypted_token = user_crypto.encrypt("some-token").unwrap();
    IntegrationConfig::upsert(
        &pool,
        user_id,
        "github",
        false,
        Some(&encrypted_token),
        None,
    )
    .await
    .unwrap();

    let mock = MockSyncService::new(vec![]);
    let result = brag_frog::sync::run_sync_with_service(
        &pool,
        &crypto,
        &http_client,
        user_id,
        "github",
        mock,
    )
    .await;

    assert!(result.is_err());
    let err = match result {
        Err(e) => format!("{}", e),
        Ok(_) => panic!("Expected error"),
    };
    assert!(err.contains("not enabled"));
}

#[tokio::test]
async fn test_sync_with_configured_integration() {
    let (pool, crypto, user_id, _phase_id) = setup_sync_test().await;
    let http_client = reqwest::Client::new();

    let mock = MockSyncService::new(vec![make_entry(
        "gh-cfg",
        "Configured sync",
        "pr_authored",
        "2025-03-10",
    )]);

    let result = brag_frog::sync::run_sync_with_service(
        &pool,
        &crypto,
        &http_client,
        user_id,
        "github",
        mock,
    )
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().log.entries_created, 1);
}
