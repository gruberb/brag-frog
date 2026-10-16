use std::sync::Arc;
use dashmap::DashMap;

/// In-memory sync status for a single user, updated by background sync tasks.
pub struct UserSyncStatus {
    pub is_syncing: bool,
    pub current_service: Option<String>,
    pub last_synced_at: Option<String>,
    pub last_error: Option<String>,
    pub services_remaining: usize,
    pub services_total: usize,
}

/// Concurrent map of user_id -> live sync status. Shared across all handlers.
pub type SyncStatusMap = Arc<DashMap<i64, UserSyncStatus>>;

pub fn new_sync_status_map() -> SyncStatusMap {
    Arc::new(DashMap::new())
}
