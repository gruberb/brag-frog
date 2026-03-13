use axum::{
    Form,
    extract::{Path, Query, State},
    response::Html,
};
use chrono::{Duration, Local};

use crate::AppState;
use crate::ai::prompts::build_meeting_prep_prompt;
use crate::ai::{get_ai_client, has_ai_for_user};
use crate::worklog::model::BragEntry;
use crate::identity::auth::middleware::AuthUser;
use crate::identity::model::User;
use crate::objectives::model::{DepartmentGoal, Priority};
use crate::cycle::model::{BragPhase, MeetingPrepNote, Week, WeeklyFocus};
use crate::reflections::model::WeeklyCheckin;
use crate::review::model::AiDocument;
use crate::kernel::error::AppError;

/// Query params for the panel GET.
#[derive(serde::Deserialize, Default)]
pub struct PanelQuery {
    #[serde(default)]
    pub edit: Option<String>,
}

/// Form for saving a meeting prep note via the panel.
#[derive(serde::Deserialize)]
pub struct SavePanelForm {
    pub week_id: i64,
    pub notes: Option<String>,
    pub doc_urls: Option<String>,
    pub meeting_goal: Option<String>,
    #[serde(default)]
    pub priority_id: Option<i64>,
}

/// GET /meeting-prep/panel/{entry_id} — panel content for a single meeting's prep notes.
pub async fn meeting_prep_panel(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(entry_id): Path<i64>,
    Query(params): Query<PanelQuery>,
) -> Result<Html<String>, AppError> {
    let _user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let now = Local::now().naive_local().date();
    let current_week = Week::find_or_create_for_date(&state.db, phase.id, now).await?;

    let entry = BragEntry::find_by_id(&state.db, entry_id, auth.user_id, &auth.crypto)
        .await?
        .ok_or(AppError::NotFound("Entry not found".into()))?;

    // Load existing prep note for this entry
    let prep_notes =
        MeetingPrepNote::list_for_week(&state.db, current_week.id, auth.user_id, &auth.crypto)
            .await?;
    let prep_note = prep_notes
        .into_iter()
        .find(|n| n.entry_id == Some(entry_id));

    // Load dept goals + priorities for priority selector
    let dept_goals = DepartmentGoal::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let priorities = Priority::list_active_for_user(&state.db, auth.user_id, &auth.crypto).await?;

    // Show edit mode if explicitly requested or if no notes exist yet
    let force_edit = params.edit.is_some();
    let has_notes = prep_note
        .as_ref()
        .map(|n| n.notes.as_ref().is_some_and(|s| !s.is_empty()))
        .unwrap_or(false);
    let edit_mode = force_edit || !has_notes;

    let has_ai = has_ai_for_user(&state, auth.user_id).await;

    let mut ctx = tera::Context::new();
    ctx.insert("entry", &entry);
    ctx.insert("current_week", &current_week);
    ctx.insert("prep_note", &prep_note);
    ctx.insert("dept_goals", &dept_goals);
    ctx.insert("priorities", &priorities);
    ctx.insert("edit_mode", &edit_mode);
    ctx.insert("has_ai", &has_ai);

    let html = state.templates.render("panels/meeting_prep.html", &ctx)?;
    Ok(Html(html))
}

/// POST /meeting-prep/panel/{entry_id} — saves prep note and returns updated panel (read mode).
pub async fn save_meeting_prep_panel(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(entry_id): Path<i64>,
    Form(input): Form<SavePanelForm>,
) -> Result<Html<String>, AppError> {
    let _user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let now = Local::now().naive_local().date();
    let current_week = Week::find_or_create_for_date(&state.db, phase.id, now).await?;

    // Save the prep note
    let prep_note = MeetingPrepNote::upsert(
        &state.db,
        auth.user_id,
        input.week_id,
        entry_id,
        input.notes.as_deref().filter(|s| !s.is_empty()),
        input.doc_urls.as_deref().filter(|s| !s.is_empty()),
        input.meeting_goal.as_deref().filter(|s| !s.is_empty()),
        &auth.crypto,
    )
    .await?;

    // Copy prep notes into the entry's description so they appear in the logbook.
    if let Some(notes) = input.notes.as_deref().filter(|s| !s.is_empty()) {
        let enc = auth.crypto.encrypt(notes)?;
        sqlx::query("UPDATE brag_entries SET description = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(&enc)
            .bind(entry_id)
            .execute(&state.db)
            .await?;
    }

    // Update entry's priority_id if provided (or clear it).
    // Ownership was already verified by find_by_id above; brag_entries has no user_id column.
    if let Some(pri_id) = input.priority_id {
        let pri_val = if pri_id == 0 { None } else { Some(pri_id) };
        sqlx::query("UPDATE brag_entries SET priority_id = ? WHERE id = ?")
            .bind(pri_val)
            .bind(entry_id)
            .execute(&state.db)
            .await?;
    }

    // Reload entry to get updated priority_id
    let entry = BragEntry::find_by_id(&state.db, entry_id, auth.user_id, &auth.crypto)
        .await?
        .ok_or(AppError::NotFound("Entry not found".into()))?;

    // Load dept goals + priorities for priority selector
    let dept_goals = DepartmentGoal::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let priorities = Priority::list_active_for_user(&state.db, auth.user_id, &auth.crypto).await?;

    let has_ai = has_ai_for_user(&state, auth.user_id).await;

    let mut ctx = tera::Context::new();
    ctx.insert("entry", &entry);
    ctx.insert("current_week", &current_week);
    ctx.insert("prep_note", &Some(prep_note));
    ctx.insert("dept_goals", &dept_goals);
    ctx.insert("priorities", &priorities);
    ctx.insert("edit_mode", &false);
    ctx.insert("has_ai", &has_ai);

    let html = state.templates.render("panels/meeting_prep.html", &ctx)?;
    Ok(Html(html))
}

/// Form for the AI draft request.
#[derive(serde::Deserialize)]
pub struct AiDraftForm {
    #[serde(default)]
    pub context_snippets: Option<String>,
    #[serde(default)]
    pub meeting_goal: Option<String>,
}

/// POST /meeting-prep/panel/{entry_id}/ai-draft — generates AI prep notes and returns plain text.
pub async fn ai_draft_meeting_prep(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(entry_id): Path<i64>,
    Form(input): Form<AiDraftForm>,
) -> Result<String, AppError> {
    let ai_client = get_ai_client(&state, auth.user_id).await?;

    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let now = Local::now().naive_local().date();
    let current_week = Week::find_or_create_for_date(&state.db, phase.id, now).await?;

    let entry = BragEntry::find_by_id(&state.db, entry_id, auth.user_id, &auth.crypto)
        .await?
        .ok_or(AppError::NotFound("Entry not found".into()))?;

    // Load dept goals + priorities
    let dept_goals = DepartmentGoal::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let priorities = Priority::list_active_for_user(&state.db, auth.user_id, &auth.crypto).await?;

    // Find linked priority and dept goal
    let linked_priority = entry
        .priority_id
        .and_then(|pri_id| priorities.iter().find(|p| p.id == pri_id));
    let linked_dept_goal = linked_priority
        .and_then(|p| p.department_goal_id)
        .and_then(|gid| dept_goals.iter().find(|g| g.id == gid));

    // Load recent entries across all priorities (last 3 weeks)
    let three_weeks_ago = (now - Duration::days(21)).format("%Y-%m-%d").to_string();
    let today_str = now.format("%Y-%m-%d").to_string();
    let user_crypto = state.crypto.for_user(auth.user_id)?;
    let all_recent = BragEntry::list_for_phase_in_range(
        &state.db,
        phase.id,
        &three_weeks_ago,
        &today_str,
        &user_crypto,
    )
    .await?;

    // Split into priority-specific entries and broader recent work
    let (recent_entries, other_recent_entries): (Vec<BragEntry>, Vec<BragEntry>) =
        if let Some(pri) = linked_priority {
            let (mut matched, rest): (Vec<_>, Vec<_>) = all_recent
                .into_iter()
                .filter(|e| e.id != entry_id)
                .partition(|e| e.priority_id == Some(pri.id));
            matched.truncate(10);
            (matched, rest)
        } else {
            (Vec::new(), all_recent.into_iter().filter(|e| e.id != entry_id).collect())
        };

    // Load check-ins for current and previous week
    let prev_week = Week::find_or_create_for_date(&state.db, phase.id, now - Duration::days(7)).await?;
    let current_checkin = WeeklyCheckin::find_for_week(&state.db, current_week.id, auth.user_id, &auth.crypto).await?;
    let prev_checkin = WeeklyCheckin::find_for_week(&state.db, prev_week.id, auth.user_id, &auth.crypto).await?;
    let checkins: Vec<&WeeklyCheckin> = [current_checkin.as_ref(), prev_checkin.as_ref()]
        .into_iter()
        .flatten()
        .collect();

    // Load current week's focus items
    let focus_items = WeeklyFocus::list_for_week(&state.db, current_week.id, auth.user_id, &auth.crypto).await?;

    let context_text = input
        .context_snippets
        .as_deref()
        .unwrap_or("")
        .trim();

    let meeting_goal = input
        .meeting_goal
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());

    // Load existing prep note
    let prep_notes =
        MeetingPrepNote::list_for_week(&state.db, current_week.id, auth.user_id, &auth.crypto)
            .await?;
    let prep_note = prep_notes
        .into_iter()
        .find(|n| n.entry_id == Some(entry_id));

    // Load prior meeting preps from the same recurring series (last 2)
    let prior_preps = if let Some(ref rg) = entry.recurring_group {
        AiDocument::list_for_recurring_group(&state.db, auth.user_id, rg, &auth.crypto)
            .await
            .unwrap_or_default()
            .into_iter()
            .take(2)
            .collect()
    } else {
        Vec::new()
    };

    let prompt = build_meeting_prep_prompt(
        &entry,
        linked_dept_goal,
        linked_priority,
        &recent_entries,
        &other_recent_entries,
        &checkins,
        &focus_items,
        context_text,
        prep_note.as_ref(),
        meeting_goal,
        &prior_preps,
    );

    let content = ai_client.generate(&prompt).await?;

    Ok(content)
}
