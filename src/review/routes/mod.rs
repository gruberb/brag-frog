pub mod checkins;
pub mod contribution_examples;
pub(crate) mod logbook_insights;
pub mod dashboard;
pub mod export;
pub mod logbook;
pub mod meeting_prep;
pub mod phases;
pub mod summaries;
pub mod trends;

use std::sync::Arc;

use crate::AppState;
use crate::ai;
use crate::shared::error::AppError;
use crate::sync::model::IntegrationConfig;

/// Get an AI client from the user's DB-stored Claude integration token.
pub async fn get_ai_client(state: &AppState, user_id: i64) -> Result<Arc<ai::AiClient>, AppError> {
    let user_crypto = state.crypto.for_user(user_id)?;
    let integration = IntegrationConfig::find_by_service(&state.db, user_id, "claude").await?;
    if let Some(config) = integration
        && config.is_enabled
        && let Some(encrypted_token) = config.encrypted_token
    {
        let key = user_crypto.decrypt(&encrypted_token)?;
        tracing::info!(
            user_id,
            service = "claude",
            action = "ai_generate",
            "Token decrypted for AI generation"
        );
        return Ok(Arc::new(ai::AiClient::new(
            key,
            state.config.ai_model.clone(),
        )));
    }

    Err(AppError::BadRequest(
        "AI not configured. Set up Claude AI in Integrations.".to_string(),
    ))
}

/// Check if AI is available for a user (per-user integration).
pub async fn has_ai_for_user(state: &AppState, user_id: i64) -> bool {
    if let Ok(Some(config)) = IntegrationConfig::find_by_service(&state.db, user_id, "claude").await
    {
        return config.is_enabled && config.encrypted_token.is_some();
    }
    false
}
