//! Profile and people aliases.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::auth::ApiAuth;
use super::dto::UserDto;
use super::error::{ApiError, ApiResult};
use crate::AppState;
use crate::identity::model::{PeopleAlias, ProfileUpdate, User};

/// Every field is optional text; an empty string clears the value (same rule as the old form).
#[derive(Deserialize, TS)]
#[ts(export)]
pub struct UpdateProfileRequest {
    #[serde(default)]
    #[ts(optional)]
    pub role: Option<String>,
    #[serde(default)]
    pub wants_promotion: bool,
    #[serde(default)]
    #[ts(optional)]
    pub display_name: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub team: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub manager_name: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub skip_level_name: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub direct_reports: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub timezone: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub week_start: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub work_start_time: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub work_end_time: Option<String>,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct PersonDto {
    pub id: i64,
    pub email: String,
    pub display_name: String,
    pub team: Option<String>,
}

impl From<PeopleAlias> for PersonDto {
    fn from(a: PeopleAlias) -> Self {
        Self {
            id: a.id,
            email: a.email,
            display_name: a.display_name,
            team: a.team,
        }
    }
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct UpsertPersonRequest {
    pub email: String,
    pub display_name: String,
    #[serde(default)]
    #[ts(optional)]
    pub team: Option<String>,
}

fn non_empty(v: &Option<String>) -> Option<&str> {
    v.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

async fn load_user(state: &AppState, user_id: i64) -> Result<User, ApiError> {
    User::find_by_id(&state.db, user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("User not found"))
}

/// `GET /api/v1/settings/profile`
pub async fn get_profile(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
) -> ApiResult<UserDto> {
    Ok(Json(load_user(&state, auth.user_id).await?.into()))
}

/// `PUT /api/v1/settings/profile`
pub async fn put_profile(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Json(input): Json<UpdateProfileRequest>,
) -> ApiResult<UserDto> {
    let role = non_empty(&input.role);
    if let Some(r) = role
        && crate::identity::clg::get_level(r).is_none()
    {
        return Err(ApiError::invalid("Unknown career level"));
    }
    User::update_settings(&state.db, auth.user_id, role, input.wants_promotion).await?;
    User::update_profile(
        &state.db,
        auth.user_id,
        &ProfileUpdate {
            display_name: non_empty(&input.display_name),
            team: non_empty(&input.team),
            manager_name: non_empty(&input.manager_name),
            skip_level_name: non_empty(&input.skip_level_name),
            direct_reports: non_empty(&input.direct_reports),
            timezone: non_empty(&input.timezone),
            week_start: non_empty(&input.week_start),
            work_start_time: non_empty(&input.work_start_time),
            work_end_time: non_empty(&input.work_end_time),
        },
    )
    .await?;
    Ok(Json(load_user(&state, auth.user_id).await?.into()))
}

/// `GET /api/v1/settings/people`
pub async fn list_people(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
) -> ApiResult<Vec<PersonDto>> {
    let people = PeopleAlias::list_for_user(&state.db, auth.user_id).await?;
    Ok(Json(people.into_iter().map(Into::into).collect()))
}

/// `POST /api/v1/settings/people`: upsert by email.
pub async fn upsert_person(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Json(input): Json<UpsertPersonRequest>,
) -> ApiResult<Vec<PersonDto>> {
    let email = input.email.trim();
    let display_name = input.display_name.trim();
    if email.is_empty() || display_name.is_empty() {
        return Err(ApiError::invalid("Email and display name are required"));
    }
    PeopleAlias::upsert(
        &state.db,
        auth.user_id,
        email,
        display_name,
        non_empty(&input.team),
    )
    .await?;
    list_people(ApiAuth(auth), State(state)).await
}

/// `DELETE /api/v1/settings/people/{id}`
pub async fn delete_person(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    PeopleAlias::delete(&state.db, id, auth.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
