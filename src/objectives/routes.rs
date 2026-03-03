use axum::{
    Form,
    extract::{Multipart, Path, State},
    response::{Html, IntoResponse},
};

use crate::AppState;
use crate::identity::auth::middleware::AuthUser;
use crate::identity::model::User;
use crate::objectives::import;
use crate::objectives::model::{
    CreateDepartmentGoal, CreateDepartmentGoalForm, CreatePriority, DepartmentGoal, Priority,
    UpdateDepartmentGoal, UpdatePriority,
};
use crate::cycle::model::BragPhase;
use crate::kernel::error::AppError;

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

    crate::objectives::service::sort_priorities(&mut priorities);
    let (goal_priorities, unassigned_priorities) =
        crate::objectives::service::group_by_department_goal(&priorities);

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
// Panel endpoints for creation forms
// ---------------------------------------------------------------------------

/// Renders the priority creation form inside a right-hand panel.
pub async fn priority_form_panel(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No active phase".to_string()))?;

    let dept_goals = DepartmentGoal::list_for_phase(&state.db, phase.id, &auth.crypto).await?;

    let mut ctx = tera::Context::new();
    ctx.insert("phase", &phase);
    ctx.insert("dept_goals", &dept_goals);

    let html = state.templates.render("panels/priority_form.html", &ctx)?;
    Ok(Html(html))
}

/// Renders the department goal creation form inside a right-hand panel.
pub async fn department_goal_form_panel(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No active phase".to_string()))?;

    let mut ctx = tera::Context::new();
    ctx.insert("phase", &phase);

    let html = state
        .templates
        .render("panels/department_goal_form.html", &ctx)?;
    Ok(Html(html))
}

// ---------------------------------------------------------------------------
// Panel endpoints for edit forms
// ---------------------------------------------------------------------------

/// Renders the priority edit form inside a right-hand panel.
pub async fn priority_edit_panel(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Html<String>, AppError> {
    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No active phase".to_string()))?;

    let priority = Priority::find_by_id(&state.db, id, auth.user_id, &auth.crypto)
        .await?
        .ok_or_else(|| AppError::NotFound("Priority not found".to_string()))?;

    let dept_goals = DepartmentGoal::list_for_phase(&state.db, phase.id, &auth.crypto).await?;

    let mut ctx = tera::Context::new();
    ctx.insert("priority", &priority);
    ctx.insert("dept_goals", &dept_goals);

    let html = state.templates.render("panels/priority_edit.html", &ctx)?;
    Ok(Html(html))
}

/// Renders the department goal edit form inside a right-hand panel.
pub async fn department_goal_edit_panel(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Html<String>, AppError> {
    let goal = DepartmentGoal::find_by_id(&state.db, id, auth.user_id, &auth.crypto)
        .await?
        .ok_or_else(|| AppError::NotFound("Department goal not found".to_string()))?;

    let mut ctx = tera::Context::new();
    ctx.insert("goal", &goal);

    let html = state
        .templates
        .render("panels/department_goal_edit.html", &ctx)?;
    Ok(Html(html))
}

// ---------------------------------------------------------------------------
// Department Goal CRUD
// ---------------------------------------------------------------------------

pub async fn create_department_goal(
    auth: AuthUser,
    State(state): State<AppState>,
    Form(input): Form<CreateDepartmentGoalForm>,
) -> Result<axum::response::Response, AppError> {
    DepartmentGoal::create(
        &state.db,
        input.phase_id,
        auth.user_id,
        &CreateDepartmentGoal {
            title: input.title,
            description: input.description,
            status: input.status,
        },
        None,
        &auth.crypto,
    )
    .await?;

    Ok(([("HX-Redirect", "/priorities")], "").into_response())
}

pub async fn update_department_goal(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(input): Form<UpdateDepartmentGoal>,
) -> Result<axum::response::Response, AppError> {
    DepartmentGoal::update(&state.db, id, auth.user_id, &input, &auth.crypto).await?;

    Ok(([("HX-Redirect", "/priorities")], "").into_response())
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
) -> Result<axum::response::Response, AppError> {
    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No active phase".to_string()))?;

    Priority::create(&state.db, phase.id, auth.user_id, &input, &auth.crypto).await?;

    Ok(([("HX-Redirect", "/priorities")], "").into_response())
}

pub async fn update_priority(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(input): Form<UpdatePriority>,
) -> Result<axum::response::Response, AppError> {
    Priority::update(&state.db, id, auth.user_id, &input, &auth.crypto).await?;

    Ok(([("HX-Redirect", "/priorities")], "").into_response())
}

pub async fn delete_priority(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Html<String>, AppError> {
    Priority::delete(&state.db, id, auth.user_id).await?;
    Ok(Html(String::new()))
}

// ---------------------------------------------------------------------------
// Lattice CSV Import
// ---------------------------------------------------------------------------

/// Imports department goals and priorities from a Lattice OKR CSV export.
pub async fn import_lattice_csv(
    auth: AuthUser,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<axum::response::Response, AppError> {
    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No active phase".to_string()))?;

    let mut csv_bytes: Option<Vec<u8>> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Multipart error: {e}")))?
    {
        if field.name() == Some("csv_file") {
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::BadRequest(format!("Failed to read file: {e}")))?;
            csv_bytes = Some(data.to_vec());
            break;
        }
    }

    let bytes = csv_bytes.ok_or_else(|| AppError::BadRequest("No CSV file provided".to_string()))?;
    let rows = import::parse_lattice_csv(&bytes)
        .map_err(|e| AppError::BadRequest(format!("CSV parse error: {e}")))?;

    crate::objectives::service::import_lattice_rows(
        &state.db,
        phase.id,
        auth.user_id,
        &rows,
        &auth.crypto,
    )
    .await?;

    Ok(([("HX-Redirect", "/priorities")], "").into_response())
}

