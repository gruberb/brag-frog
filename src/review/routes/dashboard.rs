use axum::extract::State;
use axum::response::Html;
use axum::{Form, extract::Path, response::IntoResponse};
use chrono::{Local, NaiveDate};
use serde::Serialize;

use crate::AppState;
use crate::entries::model::BragEntry;
use crate::identity::auth::middleware::AuthUser;
use crate::identity::model::User;
use crate::goals::model::{DepartmentGoal, Priority};
use crate::review::model::{BragPhase, MeetingPrepNote, Week};
use crate::shared::error::AppError;
use crate::shared::render::hx_redirect;
use crate::sync::model::IntegrationConfig;

/// A contiguous free block in a workday.
#[derive(Debug, Serialize)]
pub struct FocusBlock {
    pub hours: String, // "4h" or "1.5h"
    pub start: String, // "08:00"
    pub end: String,   // "12:00"
}

/// A single day in the week calendar widget.
#[derive(Debug, Serialize)]
pub struct FocusDay {
    pub day_abbr: String,
    pub date: String,
    pub full_date: String, // "YYYY-MM-DD"
    pub day_name: String,  // "Monday" etc.
    pub meetings: i32,
    pub focus_minutes: i32,
    pub blocks: Vec<FocusBlock>, // top-2 focus blocks
    pub is_best: bool,
}

/// A day header for grouping meetings.
#[derive(Debug, Serialize)]
pub struct MeetingDay {
    pub date: String,  // "YYYY-MM-DD"
    pub label: String, // "Monday, Feb 23"
    pub is_today: bool,
    pub is_past: bool,
}

/// Parse "HH:MM" to minutes since midnight.
fn hhmm_to_minutes(s: &str) -> Option<i32> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() == 2 {
        let h = parts[0].parse::<i32>().ok()?;
        let m = parts[1].parse::<i32>().ok()?;
        Some(h * 60 + m)
    } else {
        None
    }
}

/// Format minutes as "Xh" or "X.Yh".
fn format_minutes(mins: i32) -> String {
    if mins <= 0 {
        return "0h".to_string();
    }
    let hours = mins / 60;
    let remainder = mins % 60;
    if remainder == 0 {
        format!("{}h", hours)
    } else {
        format!("{:.1}h", mins as f64 / 60.0)
    }
}

/// Format minutes since midnight as a compact time string.
/// Whole hours: "8", "13". Partial hours: "8:30", "13:50".
fn minutes_to_hhmm(mins: i32) -> String {
    let h = mins / 60;
    let m = mins % 60;
    if m == 0 {
        format!("{}", h)
    } else {
        format!("{}:{:02}", h, m)
    }
}

/// Compute focus blocks for each weekday (Mon-Fri) of the given week.
fn compute_focus_days(
    week_start: &str,
    meetings: &[&BragEntry],
    work_start: &str,
    work_end: &str,
) -> Vec<FocusDay> {
    let work_start_min = hhmm_to_minutes(work_start).unwrap_or(9 * 60);
    let work_end_min = hhmm_to_minutes(work_end).unwrap_or(17 * 60);

    let start_date = NaiveDate::parse_from_str(week_start, "%Y-%m-%d")
        .unwrap_or_else(|_| Local::now().date_naive());

    let day_abbrs = ["Mo", "Tu", "We", "Th", "Fr"];
    let day_names = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday"];
    let mut days: Vec<FocusDay> = Vec::with_capacity(5);

    for i in 0..5 {
        let date = start_date + chrono::Duration::days(i);
        let date_str = date.format("%Y-%m-%d").to_string();

        // Collect meeting intervals for this day
        let mut intervals: Vec<(i32, i32)> = meetings
            .iter()
            .filter(|m| m.occurred_at == date_str)
            .filter_map(|m| {
                let start = m.start_time.as_deref().and_then(hhmm_to_minutes)?;
                let end = m.end_time.as_deref().and_then(hhmm_to_minutes)?;
                Some((start.max(work_start_min), end.min(work_end_min)))
            })
            .filter(|(s, e)| s < e)
            .collect();

        intervals.sort_by_key(|&(s, _)| s);

        // Count meetings (all, not just timed ones)
        let meeting_count = meetings
            .iter()
            .filter(|m| m.occurred_at == date_str)
            .count() as i32;

        // Collect ALL free blocks between meetings
        let mut free_blocks: Vec<(i32, i32)> = Vec::new();
        let mut prev_end = work_start_min;
        for (s, e) in &intervals {
            let gap = s - prev_end;
            if gap > 0 {
                free_blocks.push((prev_end, *s));
            }
            if *e > prev_end {
                prev_end = *e;
            }
        }
        // Gap after last meeting to work end
        if work_end_min > prev_end {
            free_blocks.push((prev_end, work_end_min));
        }

        // Only keep blocks >= 2 hours, then sort by duration descending, take top 2
        free_blocks.retain(|(s, e)| (e - s) >= 120);
        free_blocks.sort_by(|a, b| (b.1 - b.0).cmp(&(a.1 - a.0)));
        let top_blocks: Vec<FocusBlock> = free_blocks
            .iter()
            .take(2)
            .map(|(s, e)| FocusBlock {
                hours: format_minutes(e - s),
                start: minutes_to_hhmm(*s),
                end: minutes_to_hhmm(*e),
            })
            .collect();

        // Total focus = work hours minus meeting time
        let meeting_mins: i32 = intervals.iter().map(|(s, e)| e - s).sum();
        let focus_mins = (work_end_min - work_start_min) - meeting_mins;

        days.push(FocusDay {
            day_abbr: day_abbrs[i as usize].to_string(),
            date: date.format("%d").to_string(),
            full_date: date_str,
            day_name: day_names[i as usize].to_string(),
            meetings: meeting_count,
            focus_minutes: focus_mins.max(0),
            blocks: top_blocks,
            is_best: false,
        });
    }

    // Mark top 3 days by focus_minutes as "best"
    let mut sorted_indices: Vec<usize> = (0..days.len()).collect();
    sorted_indices.sort_by(|a, b| days[*b].focus_minutes.cmp(&days[*a].focus_minutes));
    for &idx in sorted_indices.iter().take(3) {
        if days[idx].focus_minutes > 0 {
            days[idx].is_best = true;
        }
    }

    days
}

/// Dashboard page. Shows quick capture, weekly focus, meetings,
/// OKR snapshot, check-in status, active work, and calendar widget.
pub async fn dashboard(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let phase = match BragPhase::get_active(&state.db, auth.user_id).await? {
        Some(p) => p,
        None => {
            let mut ctx = tera::Context::new();
            ctx.insert("user", &user);
            ctx.insert("current_page", "dashboard");
            let html = state.templates.render("pages/no_phase.html", &ctx)?;
            return Ok(Html(html));
        }
    };

    let now = Local::now().naive_local().date();
    let current_week = Week::find_or_create_for_date(&state.db, phase.id, now).await?;

    // This week's entries
    let week_entries = BragEntry::list_for_phase_in_range(
        &state.db,
        phase.id,
        &current_week.start_date,
        &current_week.end_date,
        &auth.crypto,
    )
    .await?;

    // Calendar meetings for the week — includes soft-deleted (excluded) entries,
    // excludes manually-created meetings. Already sorted by date + start_time.
    let today_str = now.format("%Y-%m-%d").to_string();
    let calendar_meetings = BragEntry::list_calendar_meetings_in_range(
        &state.db,
        phase.id,
        &current_week.start_date,
        &current_week.end_date,
        &auth.crypto,
    )
    .await?;
    let meetings: Vec<&BragEntry> = calendar_meetings.iter().collect();

    // Group meetings by day (ascending: Mon→Fri)
    let mut seen_dates = std::collections::BTreeSet::new();
    for m in &meetings {
        seen_dates.insert(m.occurred_at.clone());
    }
    let meeting_days: Vec<MeetingDay> = seen_dates
        .into_iter()
        .map(|d| {
            let label = NaiveDate::parse_from_str(&d, "%Y-%m-%d")
                .map(|nd| nd.format("%A, %b %e").to_string())
                .unwrap_or_else(|_| d.clone());
            MeetingDay {
                is_today: d == today_str,
                is_past: d < today_str,
                label,
                date: d,
            }
        })
        .collect();

    // Load prep notes for this week
    let prep_notes =
        MeetingPrepNote::list_for_week(&state.db, current_week.id, auth.user_id, &auth.crypto)
            .await?;
    let prep_map: std::collections::HashMap<i64, &MeetingPrepNote> = prep_notes
        .iter()
        .filter_map(|n| n.entry_id.map(|eid| (eid, n)))
        .collect();

    // Active work: open PRs, revisions, bugs, Jira tickets
    let active_work: Vec<&BragEntry> = week_entries
        .iter()
        .filter(|e| {
            match e.entry_type.as_str() {
                // PRs/revisions that aren't merged/closed
                "pr_authored" | "pr_reviewed" | "revision_authored" => !matches!(
                    e.status.as_deref(),
                    Some("MERGED") | Some("closed") | Some("merged")
                ),
                // Bugs
                "bug_filed" | "bug_fixed" => true,
                // Jira items that aren't done
                "jira_task" | "jira_story" | "jira_epic" => !matches!(
                    e.status.as_deref(),
                    Some("Done") | Some("done") | Some("Closed") | Some("closed")
                ),
                _ => false,
            }
        })
        .collect();

    // Department goals + priorities for OKR snapshot
    let dept_goals = DepartmentGoal::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let priorities = Priority::list_active_for_user(&state.db, auth.user_id, &auth.crypto).await?;

    // Check-in status for this week
    let checkin = crate::review::model::WeeklyCheckin::find_for_week(
        &state.db,
        current_week.id,
        auth.user_id,
        &auth.crypto,
    )
    .await?;
    let checkin_done = checkin.is_some();
    let checkin_energy = checkin.as_ref().and_then(|c| c.energy_level);
    let checkin_productivity = checkin.as_ref().and_then(|c| c.productivity_rating);

    // Weekly focus items (up to 3)
    let focus_items = crate::review::model::WeeklyFocus::list_for_week(
        &state.db,
        current_week.id,
        auth.user_id,
        &auth.crypto,
    )
    .await?;

    // Load linked entry IDs for each focus item
    let mut focus_entry_ids: Vec<Vec<i64>> = Vec::new();
    for focus in &focus_items {
        let entries =
            crate::review::model::WeeklyFocusEntry::list_for_focus(&state.db, focus.id).await?;
        focus_entry_ids.push(entries.iter().map(|e| e.entry_id).collect());
    }

    // All non-deleted entries from the entire phase (for focus entry picker)
    let all_phase_entries = BragEntry::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let picker_entries: Vec<&BragEntry> = all_phase_entries.iter().collect();

    // Known teams and collaborators for the full capture form
    let known_teams =
        BragEntry::distinct_teams_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let known_collaborators =
        BragEntry::distinct_collaborators_for_phase(&state.db, phase.id, &auth.crypto).await?;

    // Integration status booleans
    let configs = IntegrationConfig::list_for_user(&state.db, auth.user_id).await?;
    let has_calendar = configs
        .iter()
        .any(|c| c.service == "google_calendar" && c.encrypted_token.is_some());
    let has_code_review = configs.iter().any(|c| {
        (c.service == "github" || c.service == "phabricator") && c.encrypted_token.is_some()
    });
    let has_tickets = configs.iter().any(|c| {
        (c.service == "bugzilla" || c.service == "atlassian") && c.encrypted_token.is_some()
    });

    // Week calendar widget with focus blocks
    let work_start = user.work_start_time.as_deref().unwrap_or("09:00");
    let work_end = user.work_end_time.as_deref().unwrap_or("17:00");
    let focus_days = compute_focus_days(&current_week.start_date, &meetings, work_start, work_end);

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("current_week", &current_week);
    ctx.insert("meetings", &meetings);
    ctx.insert("meeting_days", &meeting_days);
    ctx.insert("prep_map", &prep_map);
    ctx.insert("active_work", &active_work);
    ctx.insert("dept_goals", &dept_goals);
    ctx.insert("priorities", &priorities);
    ctx.insert("focus_items", &focus_items);
    ctx.insert("focus_entry_ids", &focus_entry_ids);
    ctx.insert("picker_entries", &picker_entries);
    ctx.insert("known_teams", &known_teams);
    ctx.insert("known_collaborators", &known_collaborators);
    ctx.insert("has_calendar", &has_calendar);
    ctx.insert("has_code_review", &has_code_review);
    ctx.insert("has_tickets", &has_tickets);
    ctx.insert("checkin_done", &checkin_done);
    ctx.insert("checkin_energy", &checkin_energy);
    ctx.insert("checkin_productivity", &checkin_productivity);
    ctx.insert("focus_days", &focus_days);
    ctx.insert("current_page", "dashboard");
    ctx.insert("today", &today_str);
    ctx.insert(
        "manual_entry_types",
        &crate::entries::model::EntryType::as_manual_json_options(),
    );

    let html = state.templates.render("pages/dashboard.html", &ctx)?;
    Ok(Html(html))
}

/// Form for creating/updating a focus item.
#[derive(Debug, serde::Deserialize)]
pub struct FocusForm {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub linked_ref: Option<String>, // "priority:123", "dept_goal:5", or ""
    /// Comma-separated entry IDs (from hidden input built by JS).
    #[serde(default)]
    pub entry_ids: Option<String>,
}

impl FocusForm {
    fn parsed_entry_ids(&self) -> Vec<i64> {
        self.entry_ids
            .as_deref()
            .unwrap_or("")
            .split(',')
            .filter_map(|s| s.trim().parse::<i64>().ok())
            .collect()
    }
}

/// Parse "priority:123" or "dept_goal:5" into (type, id).
fn parse_linked_ref(s: Option<&str>) -> (Option<String>, Option<i64>) {
    match s {
        Some(r) if !r.is_empty() => {
            let parts: Vec<&str> = r.splitn(2, ':').collect();
            if parts.len() == 2
                && let Ok(id) = parts[1].parse::<i64>() {
                    return (Some(parts[0].to_string()), Some(id));
                }
            (None, None)
        }
        _ => (None, None),
    }
}

/// POST /focus/{week_id} — create a new focus item.
pub async fn create_focus(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(week_id): Path<i64>,
    Form(input): Form<FocusForm>,
) -> Result<impl IntoResponse, AppError> {
    let count =
        crate::review::model::WeeklyFocus::count_for_week(&state.db, week_id, auth.user_id).await?;
    if count >= 3 {
        return Ok(hx_redirect("/dashboard"));
    }

    let (linked_type, linked_id) = parse_linked_ref(input.linked_ref.as_deref());

    let focus = crate::review::model::WeeklyFocus::create(
        &state.db,
        &crate::review::model::CreateFocusParams {
            week_id,
            user_id: auth.user_id,
            sort_order: count,
            title: &input.title,
            linked_type: linked_type.as_deref(),
            linked_id,
            link_1: None,
            link_2: None,
            link_3: None,
        },
        &auth.crypto,
    )
    .await?;

    let eids = input.parsed_entry_ids();
    if !eids.is_empty() {
        crate::review::model::WeeklyFocusEntry::set_entries(&state.db, focus.id, &eids).await?;
    }

    Ok(hx_redirect("/dashboard"))
}

/// PUT /focus/{focus_id} — update an existing focus item.
pub async fn update_focus(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(focus_id): Path<i64>,
    Form(input): Form<FocusForm>,
) -> Result<impl IntoResponse, AppError> {
    let (linked_type, linked_id) = parse_linked_ref(input.linked_ref.as_deref());

    crate::review::model::WeeklyFocus::update(
        &state.db,
        focus_id,
        &crate::review::model::UpdateFocusParams {
            title: &input.title,
            linked_type: linked_type.as_deref(),
            linked_id,
            link_1: None,
            link_2: None,
            link_3: None,
        },
        &auth.crypto,
    )
    .await?;

    let eids = input.parsed_entry_ids();
    crate::review::model::WeeklyFocusEntry::set_entries(&state.db, focus_id, &eids).await?;

    Ok(hx_redirect("/dashboard"))
}

/// DELETE /focus/{focus_id} — delete a focus item.
pub async fn delete_focus(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(focus_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    crate::review::model::WeeklyFocus::delete(&state.db, focus_id).await?;
    Ok(hx_redirect("/dashboard"))
}
