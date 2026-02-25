use axum::{
    Form,
    extract::{Path, State},
    response::Html,
};

use crate::AppState;
use crate::identity::auth::middleware::AuthUser;
use crate::identity::model::User;
use crate::goals::model::{
    CreateDepartmentGoal, CreatePriority, DepartmentGoal, Priority, UpdateDepartmentGoal,
    UpdatePriority,
};
use crate::review::model::BragPhase;
use crate::shared::error::AppError;

// ---------------------------------------------------------------------------
// Priorities
// ---------------------------------------------------------------------------

/// Priorities page — shows department goals with nested priorities.
pub async fn priorities_page(
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
            ctx.insert("current_page", "priorities");
            let html = state.templates.render("pages/no_phase.html", &ctx)?;
            return Ok(Html(html));
        }
    };

    let dept_goals = DepartmentGoal::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let mut priorities = Priority::list_for_phase(&state.db, phase.id, &auth.crypto).await?;

    // Sort priorities by status: active first, then not_started, on_hold, completed/cancelled last
    priorities.sort_by(|a, b| {
        let priority_order = |s: &str| match s {
            "active" => 0,
            "not_started" => 1,
            "on_hold" => 2,
            "completed" => 3,
            "cancelled" => 4,
            _ => 1,
        };
        priority_order(&a.status)
            .cmp(&priority_order(&b.status))
            .then(a.sort_order.cmp(&b.sort_order))
            .then(a.id.cmp(&b.id))
    });

    // Build dept_goal_id -> Vec<Priority> map (after sort so ordering is preserved)
    let mut goal_priorities: std::collections::HashMap<String, Vec<&Priority>> =
        std::collections::HashMap::new();
    let mut unassigned_priorities: Vec<&Priority> = Vec::new();
    for p in &priorities {
        if let Some(gid) = p.department_goal_id {
            goal_priorities
                .entry(gid.to_string())
                .or_default()
                .push(p);
        } else {
            unassigned_priorities.push(p);
        }
    }

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("phases", &phases);
    ctx.insert("dept_goals", &dept_goals);
    ctx.insert("priorities", &priorities);
    ctx.insert("goal_priorities", &goal_priorities);
    ctx.insert("unassigned_priorities", &unassigned_priorities);
    ctx.insert("current_page", "priorities");

    let html = state.templates.render("pages/priorities.html", &ctx)?;
    Ok(Html(html))
}

// ---------------------------------------------------------------------------
// Department Goal CRUD
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
pub struct CreateDepartmentGoalForm {
    pub phase_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub status: Option<String>,
}

pub async fn create_department_goal(
    auth: AuthUser,
    State(state): State<AppState>,
    Form(input): Form<CreateDepartmentGoalForm>,
) -> Result<Html<String>, AppError> {
    let goal = DepartmentGoal::create(
        &state.db,
        input.phase_id,
        auth.user_id,
        &CreateDepartmentGoal {
            title: input.title,
            description: input.description,
            status: input.status,
        },
        &auth.crypto,
    )
    .await?;

    let mut ctx = tera::Context::new();
    ctx.insert("goal", &goal);
    ctx.insert("show_edit_form", &true);
    let html = state
        .templates
        .render("components/department_goal_item.html", &ctx)?;
    Ok(Html(html))
}

pub async fn update_department_goal(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(input): Form<UpdateDepartmentGoal>,
) -> Result<Html<String>, AppError> {
    let goal = DepartmentGoal::update(&state.db, id, auth.user_id, &input, &auth.crypto).await?;

    let mut ctx = tera::Context::new();
    ctx.insert("goal", &goal);
    ctx.insert("show_edit_form", &true);
    let html = state
        .templates
        .render("components/department_goal_item.html", &ctx)?;
    Ok(Html(html))
}

pub async fn delete_department_goal(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Html<String>, AppError> {
    DepartmentGoal::delete(&state.db, id, auth.user_id).await?;
    Ok(Html(String::new()))
}

// ---------------------------------------------------------------------------
// Priority CRUD
// ---------------------------------------------------------------------------

pub async fn create_priority(
    auth: AuthUser,
    State(state): State<AppState>,
    Form(input): Form<CreatePriority>,
) -> Result<Html<String>, AppError> {
    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No active phase".to_string()))?;

    let priority =
        Priority::create(&state.db, phase.id, auth.user_id, &input, &auth.crypto).await?;

    let mut ctx = tera::Context::new();
    ctx.insert("priority", &priority);
    let html = state
        .templates
        .render("components/priority_card.html", &ctx)?;
    Ok(Html(html))
}

pub async fn update_priority(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(input): Form<UpdatePriority>,
) -> Result<Html<String>, AppError> {
    let priority = Priority::update(&state.db, id, auth.user_id, &input, &auth.crypto).await?;

    let mut ctx = tera::Context::new();
    ctx.insert("priority", &priority);
    let html = state
        .templates
        .render("components/priority_card.html", &ctx)?;
    Ok(Html(html))
}

pub async fn delete_priority(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Html<String>, AppError> {
    Priority::delete(&state.db, id, auth.user_id).await?;
    Ok(Html(String::new()))
}
