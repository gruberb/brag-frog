use super::{SyncService, atlassian, bugzilla, github, google_calendar, google_drive, phabricator};

/// Returns `true` for services that can sync without an API token.
pub fn service_requires_token(service: &str) -> bool {
    !matches!(service, "bugzilla")
}

/// Maps a service name to its `SyncService` implementation.
/// Google Drive/Calendar require OAuth credentials from `Config`; pass `None` to skip.
pub fn get_sync_service(
    service: &str,
    config: Option<&crate::kernel::config::Config>,
) -> Option<Box<dyn SyncService>> {
    match service {
        "github" => Some(Box::new(github::GitHubSync)),
        "phabricator" => Some(Box::new(phabricator::PhabricatorSync)),
        "bugzilla" => Some(Box::new(bugzilla::BugzillaSync)),
        "atlassian" => Some(Box::new(atlassian::AtlassianSync)),
        "google_drive" => config.map(|c| {
            Box::new(google_drive::GoogleDriveSync {
                client_id: c.google_client_id.clone(),
                client_secret: c.google_client_secret.clone(),
            }) as Box<dyn SyncService>
        }),
        "google_calendar" => config.map(|c| {
            Box::new(google_calendar::GoogleCalendarSync {
                client_id: c.google_client_id.clone(),
                client_secret: c.google_client_secret.clone(),
            }) as Box<dyn SyncService>
        }),
        _ => None,
    }
}
