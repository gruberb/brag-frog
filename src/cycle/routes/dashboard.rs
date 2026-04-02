use axum::extract::State;
use axum::response::Html;
use axum::{Form, extract::Path, response::IntoResponse};
use chrono::Local;

use crate::AppState;
use crate::worklog::model::BragEntry;
use crate::cycle::service::dashboard::{build_meeting_days, compute_focus_days, filter_active_work};
use crate::identity::auth::middleware::AuthUser;
use crate::identity::model::User;
use crate::objectives::model::{DepartmentGoal, Priority};
use crate::cycle::model::{BragPhase, MeetingPrepNote, Week};
use crate::kernel::error::AppError;
use crate::kernel::render::hx_redirect;
use crate::integrations::model::IntegrationConfig;

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
    let meeting_days = build_meeting_days(&meetings, &today_str);

    // Load prep notes for this week
    let prep_notes =
        MeetingPrepNote::list_for_week(&state.db, current_week.id, auth.user_id, &auth.crypto)
            .await?;
    let prep_map: std::collections::HashMap<i64, &MeetingPrepNote> = prep_notes
        .iter()
        .filter_map(|n| n.entry_id.map(|eid| (eid, n)))
        .collect();

    let active_work = filter_active_work(&week_entries);

    // Department goals + priorities for OKR snapshot
    let dept_goals = DepartmentGoal::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let priorities = Priority::list_for_phase(&state.db, phase.id, &auth.crypto).await?;

    // Only show active priorities in the dashboard sidebar widget
    let dashboard_priorities: Vec<Priority> = priorities
        .iter()
        .filter(|p| p.status == "active")
        .cloned()
        .collect();

    // Weekly focus items (up to 3)
    let focus_items = crate::cycle::model::WeeklyFocus::list_for_week(
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
            crate::cycle::model::WeeklyFocusEntry::list_for_focus(&state.db, focus.id).await?;
        focus_entry_ids.push(entries.iter().map(|e| e.entry_id).collect());
    }

    // Carryover: incomplete focus items from previous week
    let carryover_items = crate::cycle::model::WeeklyFocus::list_incomplete_for_previous_week(
        &state.db,
        auth.user_id,
        current_week.id,
        &auth.crypto,
    )
    .await
    .unwrap_or_default();

    // AI availability for "What did I do last week?" button
    let has_ai = crate::ai::helpers::has_ai_for_user(&state, auth.user_id).await;

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

    // Unresolved blocker count for priorities sidebar
    let unresolved_blockers =
        crate::objectives::model::PriorityUpdate::count_unresolved_blockers(
            &state.db,
            auth.user_id,
        )
        .await
        .unwrap_or(0);

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
    ctx.insert("dashboard_priorities", &dashboard_priorities);
    ctx.insert("focus_items", &focus_items);
    ctx.insert("focus_entry_ids", &focus_entry_ids);
    ctx.insert("picker_entries", &picker_entries);
    ctx.insert("known_teams", &known_teams);
    ctx.insert("known_collaborators", &known_collaborators);
    ctx.insert("has_calendar", &has_calendar);
    ctx.insert("has_code_review", &has_code_review);
    ctx.insert("has_tickets", &has_tickets);
    ctx.insert("carryover_items", &carryover_items);
    ctx.insert("has_ai", &has_ai);
    ctx.insert("unresolved_blockers", &unresolved_blockers);
    ctx.insert("focus_days", &focus_days);
    ctx.insert("current_page", "dashboard");
    ctx.insert("today", &today_str);
    ctx.insert(
        "manual_entry_types",
        &crate::worklog::model::EntryType::as_manual_json_options(),
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
    /// Planning notes / task breakdown for this focus item.
    #[serde(default)]
    pub notes: Option<String>,
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
        crate::cycle::model::WeeklyFocus::count_for_week(&state.db, week_id, auth.user_id).await?;
    if count >= 3 {
        return Ok(hx_redirect("/dashboard"));
    }

    let (linked_type, linked_id) = parse_linked_ref(input.linked_ref.as_deref());

    let focus = crate::cycle::model::WeeklyFocus::create(
        &state.db,
        &crate::cycle::model::CreateFocusParams {
            week_id,
            user_id: auth.user_id,
            sort_order: count,
            title: &input.title,
            linked_type: linked_type.as_deref(),
            linked_id,
            link_1: None,
            link_2: None,
            link_3: None,
            notes: input.notes.as_deref(),
        },
        &auth.crypto,
    )
    .await?;

    let eids = input.parsed_entry_ids();
    if !eids.is_empty() {
        crate::cycle::model::WeeklyFocusEntry::set_entries(&state.db, focus.id, &eids).await?;
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

    crate::cycle::model::WeeklyFocus::update(
        &state.db,
        focus_id,
        auth.user_id,
        &crate::cycle::model::UpdateFocusParams {
            title: &input.title,
            linked_type: linked_type.as_deref(),
            linked_id,
            link_1: None,
            link_2: None,
            link_3: None,
            notes: input.notes.as_deref(),
        },
        &auth.crypto,
    )
    .await?;

    let eids = input.parsed_entry_ids();
    crate::cycle::model::WeeklyFocusEntry::set_entries(&state.db, focus_id, &eids).await?;

    Ok(hx_redirect("/dashboard"))
}

/// DELETE /focus/{focus_id} — delete a focus item.
pub async fn delete_focus(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(focus_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    crate::cycle::model::WeeklyFocus::delete(&state.db, focus_id, auth.user_id).await?;
    Ok(hx_redirect("/dashboard"))
}

/// POST /focus/{focus_id}/toggle — toggle completion status.
pub async fn toggle_focus_complete(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(focus_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    crate::cycle::model::WeeklyFocus::toggle_completed(&state.db, focus_id, auth.user_id).await?;
    Ok(hx_redirect("/dashboard"))
}

/// POST /dashboard/last-week-summary — AI-generate a summary of last week's work.
pub async fn last_week_summary(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let phase = crate::cycle::model::BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::BadRequest("No active phase".into()))?;

    let now = Local::now().naive_local().date();
    let current_week =
        crate::cycle::model::Week::find_or_create_for_date(&state.db, phase.id, now).await?;

    // Previous week date range: go back 7 days from current week start
    let prev_start = chrono::NaiveDate::parse_from_str(&current_week.start_date, "%Y-%m-%d")
        .map(|d| d - chrono::Duration::days(7))
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|_| current_week.start_date.clone());
    let prev_end = chrono::NaiveDate::parse_from_str(&current_week.start_date, "%Y-%m-%d")
        .map(|d| d - chrono::Duration::days(1))
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|_| current_week.start_date.clone());

    // Fetch previous week's data
    let entries = BragEntry::list_for_phase_in_range(
        &state.db,
        phase.id,
        &prev_start,
        &prev_end,
        &auth.crypto,
    )
    .await?;

    // Previous week's focus items (find by date)
    let prev_week = crate::cycle::model::Week::find_or_create_for_date(
        &state.db,
        phase.id,
        chrono::NaiveDate::parse_from_str(&prev_start, "%Y-%m-%d")
            .unwrap_or(now - chrono::Duration::days(7)),
    )
    .await?;

    let focus_items = crate::cycle::model::WeeklyFocus::list_for_week(
        &state.db,
        prev_week.id,
        auth.user_id,
        &auth.crypto,
    )
    .await?;

    let priorities =
        crate::objectives::model::Priority::list_for_phase(&state.db, phase.id, &auth.crypto)
            .await?;

    let checkin = crate::reflections::model::WeeklyCheckin::find_for_week(
        &state.db,
        prev_week.id,
        auth.user_id,
        &auth.crypto,
    )
    .await?;

    let prompt = crate::ai::prompts::build_last_week_summary_prompt(
        &entries,
        &focus_items,
        &priorities,
        checkin.as_ref(),
        &prev_start,
        &prev_end,
    );

    let ai = crate::ai::helpers::get_ai_client(&state, auth.user_id).await?;
    let generated = ai.generate(&prompt).await?;

    let mut ctx = tera::Context::new();
    ctx.insert("summary", &generated);
    ctx.insert("week_start", &prev_start);
    ctx.insert("week_end", &prev_end);

    let html = state
        .templates
        .render("components/last_week_summary.html", &ctx)?;
    Ok(Html(html))
}
