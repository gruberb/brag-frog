use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;

/// Raw database row with AES-256-GCM encrypted `content` and `prompt_used` BLOBs.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SummaryRow {
    pub id: i64,
    pub phase_id: i64,
    pub section: String,
    pub content: Vec<u8>,
    pub prompt_used: Option<Vec<u8>>,
    pub model_used: Option<String>,
    pub generated_at: String,
}

impl SummaryRow {
    /// Decrypts content and prompt into a [`Summary`].
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<Summary, AppError> {
        Ok(Summary {
            id: self.id,
            phase_id: self.phase_id,
            section: self.section,
            content: crypto.decrypt(&self.content)?,
            prompt_used: crypto.decrypt_opt(&self.prompt_used)?,
            model_used: self.model_used,
            generated_at: self.generated_at,
        })
    }
}

/// AI-generated self-reflection summary for one review section within a phase.
/// Each phase has at most one summary per section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub id: i64,
    pub phase_id: i64,
    pub section: String,
    pub content: String,
    pub prompt_used: Option<String>,
    pub model_used: Option<String>,
    pub generated_at: String,
}

/// A single review section definition loaded from config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewSection {
    pub slug: String,
    pub title: String,
    pub question: String,
    pub prompt: String,
    /// Optional alternate prompt used when CLG level information is available.
    #[serde(default)]
    pub prompt_with_clg: Option<String>,
    /// Extra text appended to the prompt when the user is targeting promotion.
    #[serde(default)]
    pub promotion_addendum: String,
}

/// Top-level review config loaded from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewConfig {
    pub review_platform: String,
    pub sections: Vec<ReviewSection>,
}

static REVIEW_CONFIG: OnceLock<ReviewConfig> = OnceLock::new();

/// Loads review sections from the TOML config file. Must be called once at startup.
pub fn load_review_config(path: &str) {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read review config at {}: {}", path, e));
    let config: ReviewConfig = toml::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse review config at {}: {}", path, e));
    REVIEW_CONFIG
        .set(config)
        .unwrap_or_else(|_| panic!("Review config already loaded"));
}

/// Returns the loaded review configuration.
pub fn review_config() -> &'static ReviewConfig {
    REVIEW_CONFIG
        .get()
        .expect("Review config not loaded. Call load_review_config() at startup.")
}

/// Returns section slugs from the config.
pub fn section_slugs() -> Vec<&'static str> {
    review_config()
        .sections
        .iter()
        .map(|s| s.slug.as_str())
        .collect()
}

/// Human-readable heading for a section slug.
pub fn section_title(section: &str) -> &'static str {
    review_config()
        .sections
        .iter()
        .find(|s| s.slug == section)
        .map(|s| s.title.as_str())
        .unwrap_or("Unknown")
}

/// Review prompt/question for a section slug.
pub fn section_question(section: &str) -> &'static str {
    review_config()
        .sections
        .iter()
        .find(|s| s.slug == section)
        .map(|s| s.question.as_str())
        .unwrap_or("")
}

/// Returns the ReviewSection for a given slug.
pub fn get_section(slug: &str) -> Option<&'static ReviewSection> {
    review_config().sections.iter().find(|s| s.slug == slug)
}
