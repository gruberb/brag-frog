use axum::extract::{Path, State};
use axum::response::{Html, IntoResponse};
use axum::Form;

use crate::AppState;
use crate::cycle::model::BragPhase;
use crate::identity::auth::middleware::AuthUser;
use crate::identity::model::User;
use crate::kernel::error::AppError;
use crate::kernel::render::hx_redirect;
use crate::todos::model::{CreateTodo, Todo};

/// GET /todos -- list page.
pub async fn todos_page(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let phase = BragPhase::get_active(&state.db, auth.user_id).await?;
    let active = Todo::list_active(&state.db, auth.user_id, &auth.crypto).await?;
    let completed = Todo::list_completed(&state.db, auth.user_id, &auth.crypto).await?;

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("active_todos", &active);
    ctx.insert("completed_todos", &completed);
    ctx.insert("current_page", "todos");

    let html = state.templates.render("pages/todos.html", &ctx)?;
    Ok(Html(html))
}

/// POST /todos -- create a new todo.
pub async fn create_todo(
    auth: AuthUser,
    State(state): State<AppState>,
    Form(input): Form<CreateTodo>,
) -> Result<impl IntoResponse, AppError> {
    if !input.title.trim().is_empty() {
        Todo::create(&state.db, auth.user_id, &input, &auth.crypto).await?;
    }
    Ok(hx_redirect("/todos"))
}

/// POST /todos/{id}/toggle -- toggle completed.
pub async fn toggle_todo(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    Todo::toggle(&state.db, id, auth.user_id).await?;
    Ok(hx_redirect("/todos"))
}

/// DELETE /todos/{id} -- delete a todo.
pub async fn delete_todo(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    Todo::delete(&state.db, id, auth.user_id).await?;
    Ok(hx_redirect("/todos"))
}
