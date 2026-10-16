use axum::extract::State;
use axum::response::Html;
use chrono::Local;

use crate::AppState;
use crate::cycle::model::{BragPhase, MeetingPrepNote, Week};
use crate::cycle::service::dashboard::{build_meeting_days, compute_focus_days, filter_active_work};
use crate::identity::auth::middleware::AuthUser;
use crate::identity::model::User;
use crate::integrations::model::IntegrationConfig;
use crate::kernel::error::AppError;
use crate::objectives::model::{DepartmentGoal, Priority};
use crate::worklog::model::BragEntry;

/// Dashboard page. Quick capture, meetings, active work, priorities,
/// focus-time widget. AI report surfaces (Last Week summary, Latest
/// Updates) live on `/reports`.
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

    // This week's entries — used for Active Work filtering.
    let week_entries = BragEntry::list_for_phase_in_range(
        &state.db,
        phase.id,
        &current_week.start_date,
        &current_week.end_date,
        &auth.crypto,
    )
    .await?;

    // Calendar meetings for the week — already sorted by date + start_time.
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

    // Department goals + priorities for the quick-capture dropdown and sidebar.
    let dept_goals = DepartmentGoal::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let priorities = Priority::list_for_phase(&state.db, phase.id, &auth.crypto).await?;

    // Only show active priorities in the dashboard sidebar widget
    let dashboard_priorities: Vec<Priority> = priorities
        .iter()
        .filter(|p| p.status == "active")
        .cloned()
        .collect();

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
    ctx.insert("known_teams", &known_teams);
    ctx.insert("known_collaborators", &known_collaborators);
    ctx.insert("has_calendar", &has_calendar);
    ctx.insert("has_code_review", &has_code_review);
    ctx.insert("has_tickets", &has_tickets);
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
