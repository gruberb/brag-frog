//! `GET /api/v1/config`: static, UI-only configuration. Prompts and other AI instructions
//! stay server-side: they are deliberately not part of any type in this file.

use axum::{Json, extract::State};
use serde::Serialize;
use ts_rs::TS;

use super::auth::ApiAuth;
use super::error::{ApiError, ApiResult};
use super::integrations::{ConfigField, config_fields};
use crate::AppState;
use crate::identity::clg;
use crate::identity::model::User;
use crate::integrations::catalog::{
    service_display_name, service_note, service_token_url,
};
use crate::integrations::model::SERVICES;
use crate::review::model::{ReviewSection, review_config};

#[derive(Serialize, TS)]
#[ts(export)]
pub struct ReviewSectionConfig {
    pub slug: String,
    pub title: String,
    pub question: String,
    pub form_question_number: Option<String>,
    pub form_required: bool,
    pub form_question: Option<String>,
    pub form_guidance: Option<String>,
    pub form_bullets: Vec<String>,
    pub form_tip: Option<String>,
    pub form_placeholder: Option<String>,
    pub focus_priorities: bool,
    pub focus_priority_help: Option<String>,
}

impl From<&ReviewSection> for ReviewSectionConfig {
    fn from(s: &ReviewSection) -> Self {
        Self {
            slug: s.slug.clone(),
            title: s.title.clone(),
            question: s.question.clone(),
            form_question_number: s.form_question_number.clone(),
            form_required: s.form_required,
            form_question: s.form_question.clone(),
            form_guidance: s.form_guidance.clone(),
            form_bullets: s.form_bullets.clone(),
            form_tip: s.form_tip.clone(),
            form_placeholder: s.form_placeholder.clone(),
            focus_priorities: s.focus_priorities,
            focus_priority_help: s.focus_priority_help.clone(),
        }
    }
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct ServiceMeta {
    pub service: String,
    pub display_name: String,
    pub token_url: String,
    pub note: String,
    pub token_only: bool,
    pub oauth_connect: bool,
    /// Field definitions with their defaults (placeholders, default orgs and URLs).
    pub fields: Vec<ConfigField>,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct ClgLevelConfig {
    pub id: String,
    pub title: String,
    pub one_liner: String,
    pub problems: ClgProblemsConfig,
    pub people: ClgPeopleConfig,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct ClgProblemsConfig {
    pub task_size: String,
    pub complexity: String,
    pub risk_management: String,
    pub domain_expertise: String,
    pub strategy: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct ClgPeopleConfig {
    pub influence: String,
    pub responsibility: String,
    pub communication: String,
    pub change_management: String,
    pub team_contributions: String,
    pub dei: String,
}

impl From<&clg::ClgLevel> for ClgLevelConfig {
    fn from(l: &clg::ClgLevel) -> Self {
        Self {
            id: l.id.clone(),
            title: l.title.clone(),
            one_liner: l.one_liner.clone(),
            problems: ClgProblemsConfig {
                task_size: l.problems.task_size.clone(),
                complexity: l.problems.complexity.clone(),
                risk_management: l.problems.risk_management.clone(),
                domain_expertise: l.problems.domain_expertise.clone(),
                strategy: l.problems.strategy.clone(),
            },
            people: ClgPeopleConfig {
                influence: l.people.influence.clone(),
                responsibility: l.people.responsibility.clone(),
                communication: l.people.communication.clone(),
                change_management: l.people.change_management.clone(),
                team_contributions: l.people.team_contributions.clone(),
                dei: l.people.dei.clone(),
            },
        }
    }
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct ConfigResponse {
    pub instance_name: Option<String>,
    pub review_platform: String,
    pub review_sections: Vec<ReviewSectionConfig>,
    pub services: Vec<ServiceMeta>,
    pub clg_levels: Vec<ClgLevelConfig>,
}

pub async fn get_config(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
) -> ApiResult<ConfigResponse> {
    // Some field defaults (Bugzilla and Atlassian email) are the user's own address.
    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("User not found"))?;
    let review = review_config();
    let no_saved = serde_json::Map::new();

    Ok(Json(ConfigResponse {
        instance_name: state.config.instance_name.clone(),
        review_platform: review.review_platform.clone(),
        review_sections: review.sections.iter().map(Into::into).collect(),
        services: SERVICES
            .iter()
            .map(|&s| ServiceMeta {
                service: s.to_string(),
                display_name: service_display_name(s).to_string(),
                token_url: service_token_url(s).to_string(),
                note: service_note(s).to_string(),
                token_only: s == "claude",
                oauth_connect: s == "google_drive" || s == "google_calendar",
                fields: config_fields(s, &user.email, &no_saved),
            })
            .collect(),
        clg_levels: clg::all_levels().iter().map(Into::into).collect(),
    }))
}
