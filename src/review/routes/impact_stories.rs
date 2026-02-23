use axum::{
    Form,
    extract::{Path, State},
    response::Html,
};

use crate::AppState;
use crate::identity::auth::middleware::AuthUser;
use crate::identity::model::User;
use crate::review::model::{BragPhase, CreateImpactStory, ImpactStory, UpdateImpactStory};
use crate::shared::error::AppError;

/// Renders the impact stories page for the active phase.
pub async fn impact_stories_page(
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

    let stories = ImpactStory::list_for_phase(&state.db, phase.id, &auth.crypto).await?;

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("stories", &stories);
    ctx.insert("current_page", "review");

    let html = state.templates.render("pages/impact_stories.html", &ctx)?;
    Ok(Html(html))
}

pub async fn create_impact_story(
    auth: AuthUser,
    State(state): State<AppState>,
    Form(input): Form<CreateImpactStory>,
) -> Result<Html<String>, AppError> {
    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No active phase".to_string()))?;

    let story =
        ImpactStory::create(&state.db, phase.id, auth.user_id, &input, &auth.crypto).await?;

    let mut ctx = tera::Context::new();
    ctx.insert("story", &story);
    let html = state.templates.render("components/story_card.html", &ctx)?;
    Ok(Html(html))
}

pub async fn update_impact_story(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(input): Form<UpdateImpactStory>,
) -> Result<Html<String>, AppError> {
    let story = ImpactStory::update(&state.db, id, auth.user_id, &input, &auth.crypto).await?;

    let mut ctx = tera::Context::new();
    ctx.insert("story", &story);
    let html = state.templates.render("components/story_card.html", &ctx)?;
    Ok(Html(html))
}

pub async fn delete_impact_story(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Html<String>, AppError> {
    ImpactStory::delete(&state.db, id, auth.user_id).await?;
    Ok(Html(String::new()))
}
