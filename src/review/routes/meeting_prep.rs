use axum::{
    Form,
    extract::{Path, Query, State},
    response::Html,
};
use chrono::Local;

use crate::AppState;
use crate::ai::prompts::build_meeting_prep_prompt;
use crate::entries::model::BragEntry;
use crate::identity::auth::middleware::AuthUser;
use crate::identity::model::User;
use crate::okr::model::{Goal, KeyResult};
use crate::review::model::{BragPhase, MeetingPrepNote, Week};
use crate::review::routes::{get_ai_client, has_ai_for_user};
use crate::shared::error::AppError;

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
    #[serde(default)]
    pub key_result_id: Option<i64>,
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

    // Load goals + key results for KR selector
    let goals = Goal::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let key_results = KeyResult::list_active_for_user(&state.db, auth.user_id).await?;

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
    ctx.insert("goals", &goals);
    ctx.insert("key_results", &key_results);
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
        &auth.crypto,
    )
    .await?;

    // Update entry's key_result_id if provided (or clear it).
    // Ownership was already verified by find_by_id above; brag_entries has no user_id column.
    if let Some(kr_id) = input.key_result_id {
        let kr_val = if kr_id == 0 { None } else { Some(kr_id) };
        sqlx::query("UPDATE brag_entries SET key_result_id = ? WHERE id = ?")
            .bind(kr_val)
            .bind(entry_id)
            .execute(&state.db)
            .await?;
    }

    // Reload entry to get updated key_result_id
    let entry = BragEntry::find_by_id(&state.db, entry_id, auth.user_id, &auth.crypto)
        .await?
        .ok_or(AppError::NotFound("Entry not found".into()))?;

    // Load goals + key results for KR selector
    let goals = Goal::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let key_results = KeyResult::list_active_for_user(&state.db, auth.user_id).await?;

    let has_ai = has_ai_for_user(&state, auth.user_id).await;

    let mut ctx = tera::Context::new();
    ctx.insert("entry", &entry);
    ctx.insert("current_week", &current_week);
    ctx.insert("prep_note", &Some(prep_note));
    ctx.insert("goals", &goals);
    ctx.insert("key_results", &key_results);
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

    // Load goals + key results
    let goals = Goal::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let key_results = KeyResult::list_active_for_user(&state.db, auth.user_id).await?;

    // Find linked KR and goal
    let linked_kr = entry
        .key_result_id
        .and_then(|kr_id| key_results.iter().find(|kr| kr.id == kr_id));
    let linked_goal = linked_kr
        .and_then(|kr| kr.goal_id)
        .and_then(|gid| goals.iter().find(|g| g.id == gid));

    // Load recent entries linked to the same KR (for work context)
    let recent_entries = if let Some(kr) = linked_kr {
        let user_crypto = state.crypto.for_user(auth.user_id)?;
        let all_entries = BragEntry::list_for_phase(&state.db, phase.id, &user_crypto).await?;
        all_entries
            .into_iter()
            .filter(|e| e.key_result_id == Some(kr.id) && e.id != entry_id)
            .take(10)
            .collect()
    } else {
        Vec::new()
    };

    // Parse context snippets (newline-separated, max 5)
    let snippets: Vec<String> = input
        .context_snippets
        .as_deref()
        .unwrap_or("")
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .take(5)
        .collect();

    // Load existing prep note
    let prep_notes =
        MeetingPrepNote::list_for_week(&state.db, current_week.id, auth.user_id, &auth.crypto)
            .await?;
    let prep_note = prep_notes
        .into_iter()
        .find(|n| n.entry_id == Some(entry_id));

    let prompt = build_meeting_prep_prompt(
        &entry,
        linked_goal,
        linked_kr,
        &recent_entries,
        &snippets,
        prep_note.as_ref(),
    );

    let content = ai_client.generate(&prompt).await?;

    Ok(content)
}
