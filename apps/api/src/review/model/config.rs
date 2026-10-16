use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

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
