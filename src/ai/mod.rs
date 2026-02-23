pub mod prompts;

use serde::{Deserialize, Serialize};

use crate::shared::error::AppError;

// Wire types for the Anthropic Messages API request.
#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ClaudeMessage>,
}

// Single message in the Messages API conversation.
#[derive(Debug, Serialize, Deserialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

// Top-level response envelope from the Messages API.
#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
}

// One content block in the API response (always text for our usage).
#[derive(Debug, Deserialize)]
struct ClaudeContent {
    text: String,
}

/// Thin wrapper around the Anthropic Messages API.
/// One instance per user -- API key comes from the user's integration settings.
pub struct AiClient {
    api_key: String,
    model: String,
    http_client: reqwest::Client,
}

impl AiClient {
    /// Creates a client with a 120s timeout (generation can be slow).
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120)) // AI generation can take a while
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Sends a single-turn prompt to Claude and returns the text response.
    pub async fn generate(&self, prompt: &str) -> Result<String, AppError> {
        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 4096,
            messages: vec![ClaudeMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        let resp = self
            .http_client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!("Claude API error: {}", body)));
        }

        let response: ClaudeResponse = resp.json().await?;
        let text = response
            .content
            .first()
            .map(|c| c.text.clone())
            .unwrap_or_default();

        Ok(text)
    }
}
