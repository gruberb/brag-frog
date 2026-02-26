use async_trait::async_trait;
use chrono::{Datelike, NaiveDate};
use serde::Deserialize;

use super::{ConnectionStatus, SyncService, SyncedEntry};
use crate::kernel::error::AppError;

/// Syncs Google Calendar events via the Calendar API v3. The stored token is a
/// refresh token; each sync/test call exchanges it for a short-lived access token first.
pub struct GoogleCalendarSync {
    pub client_id: String,
    pub client_secret: String,
}

/// Top-level response from `GET calendars/primary/events`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventListResponse {
    #[serde(default)]
    items: Vec<CalendarEvent>,
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CalendarEvent {
    id: Option<String>,
    summary: Option<String>,
    start: Option<EventDateTime>,
    end: Option<EventDateTime>,
    #[serde(default)]
    attendees: Vec<Attendee>,
    /// True when the full attendee list was omitted (e.g., large events with
    /// hidden guest lists). The API only returns the user's own entry in this case.
    #[serde(default)]
    attendees_omitted: bool,
    html_link: Option<String>,
    location: Option<String>,
    recurring_event_id: Option<String>,
    conference_data: Option<ConferenceData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConferenceData {
    #[serde(default)]
    entry_points: Vec<ConferenceEntryPoint>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConferenceEntryPoint {
    entry_point_type: Option<String>,
    uri: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventDateTime {
    /// Present for all-day events (date only, no time).
    date: Option<String>,
    /// Present for timed events (RFC 3339).
    date_time: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Attendee {
    #[serde(rename = "self", default)]
    is_self: bool,
    response_status: Option<String>,
    display_name: Option<String>,
    email: Option<String>,
}

/// Response from `GET calendars/primary` (used for test_connection).
#[derive(Debug, Deserialize)]
struct CalendarMetadata {
    summary: Option<String>,
}



/// Extract the base recurring event ID from a Google Calendar event.
/// For recurring instances, `recurring_event_id` is the base ID.
/// For non-recurring events, the `id` itself is the base.
fn base_event_id(event: &CalendarEvent) -> Option<&str> {
    event.recurring_event_id.as_deref().or(event.id.as_deref())
}

/// Extract a date string (YYYY-MM-DD) from an RFC 3339 timestamp.
fn datetime_to_date(dt: &str) -> Option<String> {
    dt.split('T').next().map(|s| s.to_string())
}

/// Extract HH:MM from an RFC 3339 timestamp (e.g., "2025-03-10T14:30:00+01:00" → "14:30").
fn datetime_to_time(dt: &str) -> Option<String> {
    let time_part = dt.split('T').nth(1)?;
    // Take first 5 chars: "HH:MM"
    if time_part.len() >= 5 {
        Some(time_part[..5].to_string())
    } else {
        None
    }
}

/// Extract the human-readable `message` from a Google API JSON error response.
fn extract_api_error(body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    v["error"]["message"].as_str().map(|s| s.to_string())
}

#[async_trait]
impl SyncService for GoogleCalendarSync {
    async fn sync(
        &self,
        _client: &reqwest::Client,
        token: &str,
        config: &serde_json::Value,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<SyncedEntry>, AppError> {
        // Build exclusion set from config
        let excluded_event_ids: std::collections::HashSet<String> =
            crate::integrations::model::IntegrationConfig::excluded_calendar_event_ids(config)
                .into_iter()
                .collect();

        // Cap end_date to end of current ISO week — we need this week's future
        // meetings for dashboard/meeting-prep, but don't want to sync months ahead
        let today = chrono::Utc::now().date_naive();
        let days_to_sunday = 7 - today.weekday().num_days_from_monday(); // Mon=0..Sun=6 → 7..1
        let end_of_week = today + chrono::Duration::days(days_to_sunday as i64);
        let effective_end = end_date.min(end_of_week);

        // token is the refresh token -- exchange for access token
        let access_token = crate::identity::auth::refresh_access_token(
            &self.client_id,
            &self.client_secret,
            token,
        )
        .await?;

        let http = super::http_client()?;
        let mut entries: Vec<SyncedEntry> = Vec::new();
        let mut page_token: Option<String> = None;
        let mut total_fetched = 0usize;
        let max_events = 2500;

        let time_min = format!("{}T00:00:00Z", start_date);
        let time_max = format!("{}T23:59:59Z", effective_end);

        loop {
            let mut params = vec![
                ("timeMin", time_min.clone()),
                ("timeMax", time_max.clone()),
                ("singleEvents", "true".to_string()),
                ("orderBy", "startTime".to_string()),
                ("maxResults", "250".to_string()),
                ("conferenceDataVersion", "1".to_string()),
            ];
            if let Some(ref pt) = page_token {
                params.push(("pageToken", pt.clone()));
            }

            let resp = http
                .get("https://www.googleapis.com/calendar/v3/calendars/primary/events")
                .bearer_auth(&access_token)
                .query(&params)
                .send()
                .await?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                let message =
                    extract_api_error(&text).unwrap_or_else(|| format!("HTTP {}", status));
                return Err(AppError::Internal(format!("Calendar API: {}", message)));
            }

            let result: EventListResponse = resp.json().await?;

            for event in &result.items {
                // Skip all-day events (have `date` instead of `dateTime`)
                let start = match &event.start {
                    Some(s) => s,
                    None => continue,
                };
                if start.date.is_some() {
                    continue;
                }
                let start_dt = match &start.date_time {
                    Some(dt) => dt,
                    None => continue,
                };

                // Skip events with fewer than 2 attendees, unless the full list
                // was omitted by the API (large events with hidden guest lists).
                if !event.attendees_omitted && event.attendees.len() < 2 {
                    continue;
                }

                // Events returned by calendars/primary are already on the user's
                // calendar. Only skip events the user explicitly declined.
                let user_declined = event.attendees.iter().any(|a| {
                    a.is_self && a.response_status.as_deref() == Some("declined")
                });
                if user_declined {
                    continue;
                }

                // Skip excluded event series
                let base_id = match base_event_id(event) {
                    Some(id) => id,
                    None => continue,
                };
                if excluded_event_ids.contains(base_id) {
                    continue;
                }

                let event_id = match &event.id {
                    Some(id) => id,
                    None => continue,
                };

                let title = event.summary.as_deref().unwrap_or("(No title)");
                let date = match datetime_to_date(start_dt) {
                    Some(d) => d,
                    None => continue,
                };

                let start_time_hm = datetime_to_time(start_dt);
                let end_time_hm = event
                    .end
                    .as_ref()
                    .and_then(|e| e.date_time.as_deref())
                    .and_then(datetime_to_time);

                // source_url = calendar event link (for viewing in Google Calendar)
                // repository = video call link (Zoom/Meet) if available
                let video_link = event
                    .conference_data
                    .as_ref()
                    .and_then(|cd| {
                        cd.entry_points
                            .iter()
                            .find(|ep| ep.entry_point_type.as_deref() == Some("video"))
                            .and_then(|ep| ep.uri.clone())
                    })
                    .or_else(|| {
                        // Zoom/Meet links often live in the location field
                        event.location.as_ref().and_then(|loc| {
                            if loc.contains("zoom.us") || loc.contains("meet.google.com") {
                                loc.split_whitespace()
                                    .find(|s| s.starts_with("https://"))
                                    .map(|s| s.to_string())
                                    .or_else(|| Some(loc.clone()))
                            } else {
                                None
                            }
                        })
                    });

                // Extract attendee names/emails as collaborators
                let collaborators = {
                    let names: Vec<&str> = event
                        .attendees
                        .iter()
                        .filter(|a| !a.is_self)
                        .filter_map(|a| {
                            a.display_name
                                .as_deref()
                                .or(a.email.as_deref())
                        })
                        .collect();
                    if names.is_empty() {
                        None
                    } else {
                        Some(names.join(", "))
                    }
                };

                entries.push(SyncedEntry {
                    source: "google_calendar",
                    source_id: format!("calendar:{}", event_id),
                    source_url: event.html_link.clone(),
                    title: title.to_string(),
                    description: None,
                    entry_type: "meeting",
                    status: None,
                    repository: video_link,
                    occurred_at: date,
                    meeting_role: None,
                    recurring_group: event.recurring_event_id.clone(),
                    start_time: start_time_hm,
                    end_time: end_time_hm,
                    collaborators,
                });
            }

            total_fetched += result.items.len();
            page_token = result.next_page_token;

            if page_token.is_none() || total_fetched >= max_events {
                break;
            }
        }

        Ok(entries)
    }

    async fn test_connection(
        &self,
        _client: &reqwest::Client,
        token: &str,
        _config: &serde_json::Value,
    ) -> Result<ConnectionStatus, AppError> {
        let access_token = match crate::identity::auth::refresh_access_token(
            &self.client_id,
            &self.client_secret,
            token,
        )
        .await
        {
            Ok(t) => t,
            Err(e) => {
                return Ok(ConnectionStatus {
                    connected: false,
                    username: None,
                    error: Some(format!("Token refresh failed: {}", e)),
                });
            }
        };

        let http = super::http_client()?;

        let resp = http
            .get("https://www.googleapis.com/calendar/v3/calendars/primary?fields=summary")
            .bearer_auth(&access_token)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            return Ok(ConnectionStatus {
                connected: false,
                username: None,
                error: Some(format!("Calendar API returned {}", status)),
            });
        }

        let meta: CalendarMetadata = resp
            .json()
            .await
            .unwrap_or(CalendarMetadata { summary: None });

        Ok(ConnectionStatus {
            connected: true,
            username: Some(
                meta.summary
                    .unwrap_or_else(|| "Google Calendar".to_string()),
            ),
            error: None,
        })
    }
}
