use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// A single check-in section definition loaded from config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckinSection {
    pub slug: String,
    pub title: String,
    pub weekly_question: String,
    pub quarterly_question: String,
    #[serde(default)]
    pub quarterly_optional: Option<String>,
    #[serde(default)]
    pub ai_prompt: String,
}

/// Top-level check-in config loaded from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckinConfig {
    pub sections: Vec<CheckinSection>,
}

static CHECKIN_CONFIG: OnceLock<CheckinConfig> = OnceLock::new();

/// Loads check-in sections from the TOML config file. Must be called once at startup.
pub fn load_checkin_config(path: &str) {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read checkin config at {}: {}", path, e));
    let config: CheckinConfig = toml::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse checkin config at {}: {}", path, e));
    CHECKIN_CONFIG
        .set(config)
        .unwrap_or_else(|_| panic!("Checkin config already loaded"));
}

/// Returns the loaded check-in configuration.
pub fn checkin_config() -> &'static CheckinConfig {
    CHECKIN_CONFIG
        .get()
        .expect("Checkin config not loaded. Call load_checkin_config() at startup.")
}

/// Returns a check-in section by slug.
pub fn get_checkin_section(slug: &str) -> Option<&'static CheckinSection> {
    checkin_config().sections.iter().find(|s| s.slug == slug)
}

/// A single assessment template (mid-year or year-end).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentTemplate {
    pub title: String,
    pub max_examples: i64,
    pub question: String,
    pub guidance: String,
    pub bullets: Vec<String>,
    #[serde(default)]
    pub tip: String,
    #[serde(default)]
    pub ai_prompt: String,
}

/// Top-level assessment config loaded from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentConfig {
    pub mid_year: AssessmentTemplate,
    pub year_end: AssessmentTemplate,
}

static ASSESSMENT_CONFIG: OnceLock<AssessmentConfig> = OnceLock::new();

/// Loads assessment templates from the TOML config file. Must be called once at startup.
pub fn load_assessment_config(path: &str) {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read assessment config at {}: {}", path, e));
    let config: AssessmentConfig = toml::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse assessment config at {}: {}", path, e));
    ASSESSMENT_CONFIG
        .set(config)
        .unwrap_or_else(|_| panic!("Assessment config already loaded"));
}

/// Returns the loaded assessment configuration.
pub fn assessment_config() -> &'static AssessmentConfig {
    ASSESSMENT_CONFIG
        .get()
        .expect("Assessment config not loaded. Call load_assessment_config() at startup.")
}

/// A single rating tier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingTier {
    pub id: String,
    pub label: String,
    pub definition: String,
    pub anchor: String,
    pub percentage: i64,
}

/// Top-level rating scale config loaded from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingScaleConfig {
    pub ratings: Vec<RatingTier>,
}

static RATING_SCALE_CONFIG: OnceLock<RatingScaleConfig> = OnceLock::new();

/// Loads the rating scale from the TOML config file. Must be called once at startup.
pub fn load_rating_scale(path: &str) {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read rating scale at {}: {}", path, e));
    let config: RatingScaleConfig = toml::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse rating scale at {}: {}", path, e));
    RATING_SCALE_CONFIG
        .set(config)
        .unwrap_or_else(|_| panic!("Rating scale config already loaded"));
}

/// Returns the loaded rating scale configuration.
pub fn rating_scale_config() -> &'static RatingScaleConfig {
    RATING_SCALE_CONFIG
        .get()
        .expect("Rating scale config not loaded. Call load_rating_scale() at startup.")
}

/// Loads all review-related configs (review sections, check-in sections, assessment
/// templates, rating scale). `resolve_path` maps a config filename to its full path,
/// enabling the custom/ overlay logic in prod and direct config/ paths in tests.
pub fn initialize_config(resolve_path: impl Fn(&str) -> String) {
    crate::cycle::model::load_review_config(&resolve_path("review_sections.toml"));
    load_checkin_config(&resolve_path("checkin_sections.toml"));
    load_assessment_config(&resolve_path("assessment_templates.toml"));
    load_rating_scale(&resolve_path("rating_scale.toml"));
}
