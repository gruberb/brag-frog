use axum::{
    Form,
    extract::{Path, State},
    response::{Html, IntoResponse},
};

use crate::AppState;
use crate::identity::auth::middleware::AuthUser;
use crate::identity::model::User;
use crate::okr::model::KeyResult;
use crate::okr::model::{CreateGoal, Goal, UpdateGoal};
use crate::okr::model::{CreateInitiative, Initiative, UpdateInitiative};
use crate::okr::model::{CreateKeyResult, UpdateKeyResult};
use crate::review::model::BragPhase;
use crate::shared::error::AppError;
use crate::shared::render::hx_redirect;

// ---------------------------------------------------------------------------
// Goals page
// ---------------------------------------------------------------------------

/// Goals & OKRs page — shows goals with nested KRs, initiatives, and phase management.
/// Consolidates the old /phases page into a single OKR management view.
pub async fn goals_page(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let phases = BragPhase::list_for_user(&state.db, auth.user_id).await?;

    let phase = match BragPhase::get_active(&state.db, auth.user_id).await? {
        Some(p) => p,
        None => {
            let mut ctx = tera::Context::new();
            ctx.insert("user", &user);
            ctx.insert("phases", &phases);
            ctx.insert("current_page", "goals");
            let html = state.templates.render("pages/no_phase.html", &ctx)?;
            return Ok(Html(html));
        }
    };

    let mut goals = Goal::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let key_results = KeyResult::list_for_user(&state.db, auth.user_id).await?;
    let initiatives = Initiative::list_for_phase(&state.db, phase.id, &auth.crypto).await?;

    // Build goal_id → Vec<KeyResult> map for nesting KRs under goals
    let mut goal_key_results: std::collections::HashMap<String, Vec<&KeyResult>> =
        std::collections::HashMap::new();
    let mut unassigned_krs: Vec<&KeyResult> = Vec::new();
    for kr in &key_results {
        if let Some(gid) = kr.goal_id {
            goal_key_results
                .entry(gid.to_string())
                .or_default()
                .push(kr);
        } else if !kr.is_archived {
            unassigned_krs.push(kr);
        }
    }

    // Goal-level progress: average of child KR progress
    let mut goal_progress: std::collections::HashMap<String, i64> =
        std::collections::HashMap::new();
    for goal in &goals {
        let gid_str = goal.id.to_string();
        if let Some(krs) = goal_key_results.get(&gid_str)
            && !krs.is_empty() {
                let total: i64 = krs.iter().map(|kr| kr.progress).sum();
                let avg = (total as f64 / krs.len() as f64).round() as i64;
                goal_progress.insert(gid_str, avg);
            }
    }

    // Sort goals by status priority: in_progress first, then not_started, on_hold, completed last
    goals.sort_by(|a, b| {
        let priority = |s: &str| match s {
            "in_progress" => 0,
            "not_started" => 1,
            "on_hold" => 2,
            "completed" => 3,
            _ => 1,
        };
        priority(&a.status)
            .cmp(&priority(&b.status))
            .then(a.sort_order.cmp(&b.sort_order))
            .then(a.id.cmp(&b.id))
    });

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("phases", &phases);
    ctx.insert("goals", &goals);
    ctx.insert("key_results", &key_results);
    ctx.insert("goal_key_results", &goal_key_results);
    ctx.insert("unassigned_krs", &unassigned_krs);
    ctx.insert("goal_progress", &goal_progress);
    ctx.insert("initiatives", &initiatives);
    ctx.insert("current_page", "goals");

    let html = state.templates.render("pages/goals.html", &ctx)?;
    Ok(Html(html))
}

// ---------------------------------------------------------------------------
// Goal CRUD
// ---------------------------------------------------------------------------

/// Form payload for creating a new goal under a phase.
#[derive(serde::Deserialize)]
pub struct CreateGoalForm {
    pub phase_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub status: Option<String>,
}

/// HTMX handler: creates a goal and returns the rendered goal item fragment with edit form open.
pub async fn create_goal(
    auth: AuthUser,
    State(state): State<AppState>,
    Form(input): Form<CreateGoalForm>,
) -> Result<Html<String>, AppError> {
    let goal = Goal::create(
        &state.db,
        input.phase_id,
        auth.user_id,
        &CreateGoal {
            title: input.title,
            description: input.description,
            category: input.category,
            status: input.status,
        },
        &auth.crypto,
    )
    .await?;

    let mut ctx = tera::Context::new();
    ctx.insert("goal", &goal);
    ctx.insert("show_edit_form", &true);
    let html = state.templates.render("components/goal_item.html", &ctx)?;
    Ok(Html(html))
}

/// HTMX handler: updates a goal and returns the re-rendered goal item fragment.
pub async fn update_goal(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(input): Form<UpdateGoal>,
) -> Result<Html<String>, AppError> {
    let goal = Goal::update(&state.db, id, auth.user_id, &input, &auth.crypto).await?;

    // Cascade: when a goal is completed, mark all child KRs as completed at 100%
    if goal.status == "completed" {
        KeyResult::complete_all_for_goal(&state.db, id, auth.user_id).await?;
    }

    let mut ctx = tera::Context::new();
    ctx.insert("goal", &goal);
    ctx.insert("show_edit_form", &true);
    let html = state.templates.render("components/goal_item.html", &ctx)?;
    Ok(Html(html))
}

/// HTMX handler: deletes a goal and returns empty HTML for outerHTML swap removal.
pub async fn delete_goal(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Html<String>, AppError> {
    Goal::delete(&state.db, id, auth.user_id).await?;

    // Return empty string — HTMX outerHTML swap removes the element
    Ok(Html(String::new()))
}

// ---------------------------------------------------------------------------
// Key Result CRUD
// ---------------------------------------------------------------------------

/// HTMX handler: creates a key result and redirects to the goals page.
pub async fn create_key_result(
    auth: AuthUser,
    State(state): State<AppState>,
    Form(input): Form<CreateKeyResult>,
) -> Result<impl IntoResponse, AppError> {
    let _key_result = KeyResult::create(&state.db, auth.user_id, &input).await?;

    Ok(hx_redirect("/goals"))
}

/// HTMX handler: updates a key result's fields and redirects to the goals page.
pub async fn update_key_result(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(input): Form<UpdateKeyResult>,
) -> Result<impl IntoResponse, AppError> {
    let _key_result = KeyResult::update(&state.db, id, auth.user_id, &input).await?;

    Ok(hx_redirect("/goals"))
}

/// HTMX handler: deletes a key result and redirects to the goals page.
pub async fn delete_key_result(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    KeyResult::delete(&state.db, id, auth.user_id).await?;
    Ok(hx_redirect("/goals"))
}

// ---------------------------------------------------------------------------
// Initiative CRUD + Panel
// ---------------------------------------------------------------------------

pub async fn create_initiative(
    auth: AuthUser,
    State(state): State<AppState>,
    Form(input): Form<CreateInitiative>,
) -> Result<Html<String>, AppError> {
    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No active phase".to_string()))?;

    let initiative =
        Initiative::create(&state.db, phase.id, auth.user_id, &input, &auth.crypto).await?;

    let mut ctx = tera::Context::new();
    ctx.insert("initiative", &initiative);
    let html = state
        .templates
        .render("components/initiative_card.html", &ctx)?;
    Ok(Html(html))
}

pub async fn update_initiative(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(input): Form<UpdateInitiative>,
) -> Result<Html<String>, AppError> {
    let initiative = Initiative::update(&state.db, id, auth.user_id, &input, &auth.crypto).await?;

    let mut ctx = tera::Context::new();
    ctx.insert("initiative", &initiative);
    let html = state
        .templates
        .render("components/initiative_card.html", &ctx)?;
    Ok(Html(html))
}

pub async fn delete_initiative(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Html<String>, AppError> {
    Initiative::delete(&state.db, id, auth.user_id).await?;
    Ok(Html(String::new()))
}

pub async fn initiative_panel(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Html<String>, AppError> {
    let initiative = Initiative::find_by_id(&state.db, id, auth.user_id, &auth.crypto)
        .await?
        .ok_or(AppError::NotFound("Initiative not found".to_string()))?;

    let linked_kr_ids = Initiative::linked_key_result_ids(&state.db, id).await?;
    let key_results = KeyResult::list_active_for_user(&state.db, auth.user_id).await?;

    let mut ctx = tera::Context::new();
    ctx.insert("initiative", &initiative);
    ctx.insert("linked_kr_ids", &linked_kr_ids);
    ctx.insert("key_results", &key_results);
    let html = state
        .templates
        .render("panels/initiative_detail.html", &ctx)?;
    Ok(Html(html))
}
