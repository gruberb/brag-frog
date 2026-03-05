use axum::{
    Form,
    extract::{Path, State},
    response::Html,
};

use crate::AppState;
use crate::worklog::model::BragEntry;
use crate::identity::auth::middleware::AuthUser;
use crate::identity::model::User;
use crate::kernel::render::html_escape;
use crate::cycle::model::{
    BragPhase, ContributionExample, CreateContributionExample, UpdateContributionExample,
};
use crate::kernel::error::AppError;

/// Renders the contribution examples page for the active phase.
pub async fn contribution_examples_page(
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
            ctx.insert("current_page", "review");
            let html = state.templates.render("pages/no_phase.html", &ctx)?;
            return Ok(Html(html));
        }
    };

    let examples =
        ContributionExample::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let entries =
        BragEntry::list_for_phase(&state.db, phase.id, &auth.crypto).await?;

    // Build linked entry IDs per example
    let mut example_entries: Vec<serde_json::Value> = Vec::new();
    for ex in &examples {
        let linked_ids = ContributionExample::linked_entry_ids(&state.db, ex.id).await?;
        let linked: Vec<&BragEntry> = entries.iter().filter(|e| linked_ids.contains(&e.id)).collect();
        example_entries.push(serde_json::json!({
            "example_id": ex.id,
            "linked_entries": linked,
        }));
    }

    let has_ai = super::has_ai_for_user(&state, auth.user_id).await;

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("examples", &examples);
    ctx.insert("entries", &entries);
    ctx.insert("example_entries", &example_entries);
    ctx.insert("has_ai", &has_ai);
    ctx.insert("current_page", "review");

    let html = state
        .templates
        .render("pages/contribution_examples.html", &ctx)?;
    Ok(Html(html))
}

pub async fn create_contribution_example(
    auth: AuthUser,
    State(state): State<AppState>,
    Form(input): Form<CreateContributionExample>,
) -> Result<Html<String>, AppError> {
    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No active phase".to_string()))?;

    let example = ContributionExample::create(
        &state.db,
        phase.id,
        auth.user_id,
        &input,
        &auth.crypto,
    )
    .await?;

    let mut ctx = tera::Context::new();
    ctx.insert("example", &example);
    let html = state
        .templates
        .render("components/contribution_example_card.html", &ctx)?;
    Ok(Html(html))
}

pub async fn update_contribution_example(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(input): Form<UpdateContributionExample>,
) -> Result<Html<String>, AppError> {
    let example =
        ContributionExample::update(&state.db, id, auth.user_id, &input, &auth.crypto).await?;

    let mut ctx = tera::Context::new();
    ctx.insert("example", &example);
    let html = state
        .templates
        .render("components/contribution_example_card.html", &ctx)?;
    Ok(Html(html))
}

pub async fn delete_contribution_example(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Html<String>, AppError> {
    ContributionExample::delete(&state.db, id, auth.user_id).await?;
    Ok(Html(String::new()))
}

/// Links an entry to a contribution example. Returns the updated linked entries chips.
pub async fn link_entry_to_example(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((example_id, entry_id)): Path<(i64, i64)>,
) -> Result<Html<String>, AppError> {
    // Verify ownership
    let _example = ContributionExample::find_by_id(&state.db, example_id, auth.user_id, &auth.crypto)
        .await?
        .ok_or_else(|| AppError::NotFound("Example not found".to_string()))?;

    ContributionExample::link_entry(&state.db, example_id, entry_id).await?;

    render_linked_entries_chips(&state, example_id, &auth).await
}

/// Unlinks an entry from a contribution example. Returns the updated linked entries chips.
pub async fn unlink_entry_from_example(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((example_id, entry_id)): Path<(i64, i64)>,
) -> Result<Html<String>, AppError> {
    let _example = ContributionExample::find_by_id(&state.db, example_id, auth.user_id, &auth.crypto)
        .await?
        .ok_or_else(|| AppError::NotFound("Example not found".to_string()))?;

    ContributionExample::unlink_entry(&state.db, example_id, entry_id).await?;

    render_linked_entries_chips(&state, example_id, &auth).await
}

/// Renders the linked entries chips HTML fragment for a contribution example.
async fn render_linked_entries_chips(
    state: &AppState,
    example_id: i64,
    auth: &AuthUser,
) -> Result<Html<String>, AppError> {
    let linked_ids = ContributionExample::linked_entry_ids(&state.db, example_id).await?;

    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No active phase".to_string()))?;

    let entries = BragEntry::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let linked: Vec<&BragEntry> = entries.iter().filter(|e| linked_ids.contains(&e.id)).collect();

    let mut html = String::new();
    for entry in &linked {
        html.push_str(&format!(
            "<span class=\"badge\" style=\"display:inline-flex;align-items:center;gap:4px;margin:2px;\">{title}<button class=\"filter-pill-remove\" hx-delete=\"/contribution-examples/{eid}/entries/{entid}\" hx-target=\"#linked-entries-{eid}\" hx-swap=\"innerHTML\">&times;</button></span>",
            title = html_escape(&entry.title),
            eid = example_id,
            entid = entry.id,
        ));
    }

    Ok(Html(html))
}

