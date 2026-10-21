pub mod helpers;
pub mod prompts;

pub use helpers::get_ai_client;

use serde::{Deserialize, Serialize};

use crate::kernel::config::AiSettings;
use crate::kernel::error::AppError;

/// Output ceiling per request. Thinking counts toward it (current models always think), so it
/// is sized for reasoning plus a long draft, while staying under the size where a
/// non-streaming request risks HTTP timeouts.
const MAX_TOKENS: u32 = 16_000;

/// Whole-request timeout. Generation with thinking can take minutes; this stays below Cloud
/// Run's default 300 s request timeout so the user gets our error, not a dropped connection.
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(240);

/// Beta header that enables `"fallbacks": "default"`.
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";

// Wire types for the Anthropic Messages API request.
#[derive(Debug, Serialize)]
struct ClaudeRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    messages: Vec<ClaudeMessage<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_config: Option<OutputConfig<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fallbacks: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct OutputConfig<'a> {
    effort: &'a str,
}

// Single message in the Messages API conversation.
#[derive(Debug, Serialize)]
struct ClaudeMessage<'a> {
    role: &'static str,
    content: &'a str,
}

// Top-level response envelope from the Messages API.
#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ContentBlock>,
    stop_reason: Option<String>,
    stop_details: Option<StopDetails>,
}

// One content block. Besides `text`, responses carry `thinking` blocks (empty by default)
// and, after a server-side fallback, `fallback` blocks, so blocks are read by type.
#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StopDetails {
    category: Option<String>,
}

/// Thin wrapper around the Anthropic Messages API.
/// One instance per user -- API key comes from the user's integration settings.
pub struct AiClient {
    api_key: String,
    settings: AiSettings,
    http_client: reqwest::Client,
}

impl AiClient {
    pub fn new(api_key: String, settings: AiSettings) -> Self {
        Self {
            api_key,
            settings,
            http_client: reqwest::Client::builder()
                .timeout(REQUEST_TIMEOUT)
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Sends a single-turn prompt to Claude and returns the text response.
    pub async fn generate(&self, prompt: &str) -> Result<String, AppError> {
        let request = ClaudeRequest {
            model: &self.settings.model,
            max_tokens: MAX_TOKENS,
            messages: vec![ClaudeMessage { role: "user", content: prompt }],
            output_config: self.settings.effort.as_deref().map(|effort| OutputConfig { effort }),
            fallbacks: self.settings.fallbacks.as_deref(),
        };

        let mut builder = self
            .http_client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json");
        if request.fallbacks.is_some() {
            builder = builder.header("anthropic-beta", FALLBACK_BETA);
        }
        let resp = builder.json(&request).send().await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!("Claude API error: {}", body)));
        }

        response_text(resp.json().await?)
    }
}

/// The reply text of a response: every `text` block in order. A refusal (HTTP 200 with
/// `stop_reason: "refusal"`, also after any fallback declined) becomes an error instead of an
/// empty draft.
fn response_text(response: ClaudeResponse) -> Result<String, AppError> {
    if response.stop_reason.as_deref() == Some("refusal") {
        let category = response
            .stop_details
            .and_then(|d| d.category)
            .unwrap_or_else(|| "unspecified".to_string());
        return Err(AppError::BadRequest(format!(
            "Claude declined this request (category: {category}). Try rephrasing the input."
        )));
    }
    let text: String = response
        .content
        .iter()
        .filter(|b| b.kind == "text")
        .filter_map(|b| b.text.as_deref())
        .collect();
    if response.stop_reason.as_deref() == Some("max_tokens") {
        // Kept rather than failed: a cut-off draft is still editable, a lost one is not.
        tracing::warn!(max_tokens = MAX_TOKENS, "AI response hit the output limit");
    }
    if text.trim().is_empty() {
        return Err(AppError::Internal("Claude returned no text".to_string()));
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: serde_json::Value) -> Result<String, AppError> {
        response_text(serde_json::from_value(json).unwrap())
    }

    #[test]
    fn reads_text_blocks_after_thinking_and_fallback_blocks() {
        let text = parse(serde_json::json!({
            "content": [
                {"type": "thinking", "thinking": "", "signature": "sig"},
                {"type": "fallback", "from": {"model": "a"}, "to": {"model": "b"}},
                {"type": "text", "text": "Draft "},
                {"type": "text", "text": "body"}
            ],
            "stop_reason": "end_turn",
            "stop_details": null
        }))
        .unwrap();
        assert_eq!(text, "Draft body");
    }

    #[test]
    fn refusal_is_an_error_with_its_category() {
        let err = parse(serde_json::json!({
            "content": [],
            "stop_reason": "refusal",
            "stop_details": {"type": "refusal", "category": "bio", "explanation": "x"}
        }))
        .unwrap_err();
        assert!(matches!(err, AppError::BadRequest(m) if m.contains("bio")));
    }

    #[test]
    fn thinking_only_response_is_an_error() {
        let result = parse(serde_json::json!({
            "content": [{"type": "thinking", "thinking": "", "signature": "sig"}],
            "stop_reason": "max_tokens"
        }));
        assert!(result.is_err());
    }

    #[test]
    fn request_omits_unset_settings() {
        let request = ClaudeRequest {
            model: "m",
            max_tokens: 1,
            messages: vec![],
            output_config: None,
            fallbacks: None,
        };
        let json = serde_json::to_value(&request).unwrap();
        assert!(json.get("output_config").is_none() && json.get("fallbacks").is_none());
        let request = ClaudeRequest {
            output_config: Some(OutputConfig { effort: "medium" }),
            fallbacks: Some("default"),
            ..request
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["output_config"]["effort"], "medium");
        assert_eq!(json["fallbacks"], "default");
    }
}
