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
