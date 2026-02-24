use axum::{
    Form,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue},
    response::Html,
};
use chrono::{Local, NaiveDate};

use crate::AppState;
use crate::entries::model::{BragEntry, CreateEntry, EntryType};
use crate::identity::auth::middleware::AuthUser;
use crate::okr::model::{Goal, Initiative, KeyResult};
use crate::review::model::{BragPhase, Week};
use crate::shared::error::AppError;
use crate::shared::serde_helpers::deserialize_optional_i64;

/// Builds a Tera context with entry, key results, goals, and entry type options.
async fn build_entry_context(
    state: &AppState,
    user_id: i64,
    entry: &BragEntry,
) -> Result<tera::Context, AppError> {
    let user_crypto = state.crypto.for_user(user_id)?;
    let key_results = KeyResult::list_active_for_user(&state.db, user_id).await?;
    let goals = Goal::list_for_active_phase(&state.db, user_id, &user_crypto).await?;

    let phase = BragPhase::get_active(&state.db, user_id).await?;
    let (known_teams, initiatives) = if let Some(ref p) = phase {
        let teams = BragEntry::distinct_teams_for_phase(&state.db, p.id, &user_crypto).await?;
        let inits = Initiative::list_for_phase(&state.db, p.id, &user_crypto).await?;
        (teams, inits)
    } else {
        (Vec::new(), Vec::new())
    };

    let mut ctx = tera::Context::new();
    ctx.insert("entry", entry);
    ctx.insert("key_results", &key_results);
    ctx.insert("goals", &goals);
    ctx.insert("initiatives", &initiatives);
    ctx.insert("known_teams", &known_teams);
    ctx.insert("entry_types", &EntryType::as_json_options());
    ctx.insert("manual_entry_types", &EntryType::as_manual_json_options());
    ctx.insert("grouped_entry_types", &EntryType::as_grouped_json_options());
    ctx.insert(
        "manual_grouped_entry_types",
        &EntryType::as_manual_grouped_json_options(),
    );
    Ok(ctx)
}

/// Renders a single entry card fragment with key result/goal context for HTMX swap.
async fn render_entry_card(
    state: &AppState,
    user_id: i64,
    entry: &BragEntry,
) -> Result<Html<String>, AppError> {
    let ctx = build_entry_context(state, user_id, entry).await?;
    let html = state.templates.render("components/entry_card.html", &ctx)?;
    Ok(Html(html))
}

/// HTMX handler: updates an entry. Reassigns to the correct week if the date changed.
pub async fn update_entry(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(input): Form<crate::entries::model::UpdateEntry>,
) -> Result<Html<String>, AppError> {
    let entry = super::service::update_with_week_reassignment(
        &state.db,
        id,
        auth.user_id,
        &input,
        &auth.crypto,
    )
    .await?;

    render_entry_card(&state, auth.user_id, &entry).await
}

/// HTMX handler: deletes an entry. Synced entries are soft-deleted; manual entries are hard-deleted.
pub async fn delete_entry(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Html<String>, AppError> {
    let entry = BragEntry::find_by_id(&state.db, id, auth.user_id, &auth.crypto)
        .await?
        .ok_or(AppError::NotFound("Entry not found".to_string()))?;

    if entry.source == "manual" {
        BragEntry::delete(&state.db, id, auth.user_id).await?;
    } else {
        BragEntry::soft_delete(&state.db, id, auth.user_id).await?;
    }

    Ok(Html(String::new()))
}

/// HTMX handler: returns the read-only entry card fragment (cancels an edit).
pub async fn view_entry(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Html<String>, AppError> {
    let entry = BragEntry::find_by_id(&state.db, id, auth.user_id, &auth.crypto)
        .await?
        .ok_or(AppError::NotFound("Entry not found".to_string()))?;

    render_entry_card(&state, auth.user_id, &entry).await
}

/// Form payload for the quick-add entry bar (minimal fields, auto-resolves week).
#[derive(serde::Deserialize)]
pub struct QuickCreateEntry {
    pub title: String,
    pub entry_type: String,
    #[serde(default)]
    pub occurred_at: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub key_result_id: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub initiative_id: Option<i64>,
    #[serde(default)]
    pub teams: Option<String>,
    #[serde(default)]
    pub collaborators: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
}

/// HTMX handler: creates an entry from the quick-add bar, auto-resolving the week from the date.
pub async fn quick_create_entry(
    auth: AuthUser,
    State(state): State<AppState>,
    Form(input): Form<QuickCreateEntry>,
) -> Result<Html<String>, AppError> {
    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No active phase".to_string()))?;

    let occurred_at = input.occurred_at.clone().unwrap_or_else(|| {
        Local::now()
            .naive_local()
            .date()
            .format("%Y-%m-%d")
            .to_string()
    });

    let date = NaiveDate::parse_from_str(&occurred_at, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("Invalid date format".to_string()))?;

    // Validate date falls within the active review cycle
    let phase_start = NaiveDate::parse_from_str(&phase.start_date, "%Y-%m-%d")
        .map_err(|_| AppError::Internal("Invalid phase start date".to_string()))?;
    let phase_end = NaiveDate::parse_from_str(&phase.end_date, "%Y-%m-%d")
        .map_err(|_| AppError::Internal("Invalid phase end date".to_string()))?;
    if date < phase_start || date > phase_end {
        return Err(AppError::BadRequest(format!(
            "Date must be within your review cycle ({} to {})",
            phase.start_date, phase.end_date
        )));
    }

    let week = Week::find_or_create_for_date(&state.db, phase.id, date).await?;

    let teams = input
        .teams
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(String::from);
    let collaborators = input
        .collaborators
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(String::from);
    let source_url = input
        .source_url
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(String::from);

    let create_input = CreateEntry {
        week_id: week.id,
        key_result_id: input.key_result_id,
        initiative_id: input.initiative_id,
        title: input.title,
        description: None,
        entry_type: input.entry_type,
        occurred_at,
        teams,
        collaborators,
        source_url,
    };

    let entry = BragEntry::create(&state.db, &create_input, auth.user_id, &auth.crypto).await?;

    // Return a brief flash message instead of the full entry card
    let label = crate::entries::model::EntryType::display_name(&entry.entry_type);
    let safe_title = entry.title.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    Ok(Html(format!(
        r#"<div class="quick-entry-flash" onanimationend="this.remove()">Logged: {safe_title} ({label})</div>"#,
    )))
}

/// HTMX handler: returns entry detail content for the slide-over panel.
pub async fn entry_panel(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Html<String>, AppError> {
    let entry = BragEntry::find_by_id(&state.db, id, auth.user_id, &auth.crypto)
        .await?
        .ok_or(AppError::NotFound("Entry not found".to_string()))?;

    let ctx = build_entry_context(&state, auth.user_id, &entry).await?;
    let html = state.templates.render("panels/entry_detail.html", &ctx)?;
    Ok(Html(html))
}

/// HTMX handler: classify a meeting entry by setting its meeting_role.
#[derive(serde::Deserialize)]
pub struct ClassifyEntryForm {
    pub meeting_role: String,
    #[serde(default)]
    pub teams: Option<String>,
    #[serde(default)]
    pub save_rule: Option<bool>,
}

pub async fn classify_entry(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(input): Form<ClassifyEntryForm>,
) -> Result<Html<String>, AppError> {
    let entry = super::service::classify_entry(
        &state.db,
        id,
        auth.user_id,
        &input.meeting_role,
        input.teams.as_deref(),
        input.save_rule.unwrap_or(false),
        &auth.crypto,
    )
    .await?;

    render_entry_card(&state, auth.user_id, &entry).await
}

/// HTMX handler: excludes a Google Drive file from future syncs and soft-deletes all its entries.
pub async fn exclude_drive_file(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<(HeaderMap, Html<String>), AppError> {
    let entry = BragEntry::find_by_id(&state.db, id, auth.user_id, &auth.crypto)
        .await?
        .ok_or(AppError::NotFound("Entry not found".to_string()))?;

    super::service::exclude_drive_file(&state.db, auth.user_id, &entry).await?;

    let mut headers = HeaderMap::new();
    headers.insert("HX-Redirect", HeaderValue::from_static("/logbook"));
    Ok((headers, Html(String::new())))
}

/// HTMX handler: excludes a Google Calendar event series from future syncs and soft-deletes all its entries.
pub async fn exclude_calendar_event(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<(HeaderMap, Html<String>), AppError> {
    let entry = BragEntry::find_by_id(&state.db, id, auth.user_id, &auth.crypto)
        .await?
        .ok_or(AppError::NotFound("Entry not found".to_string()))?;

    super::service::exclude_calendar_event(&state.db, auth.user_id, &entry).await?;

    let mut headers = HeaderMap::new();
    headers.insert("HX-Redirect", HeaderValue::from_static("/logbook"));
    Ok((headers, Html(String::new())))
}
