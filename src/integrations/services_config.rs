use serde::Deserialize;
use std::sync::OnceLock;

/// Per-service configuration defaults loaded from `config/services.toml`.
#[derive(Debug, Deserialize)]
pub struct ServicesConfig {
    pub github: GitHubDefaults,
    pub phabricator: PhabricatorDefaults,
    pub bugzilla: BugzillaDefaults,
    pub atlassian: AtlassianDefaults,
    pub claude: ClaudeDefaults,
}

#[derive(Debug, Deserialize)]
pub struct GitHubDefaults {
    pub default_orgs: String,
    pub org_placeholder: String,
    pub token_url: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub allowed_orgs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct PhabricatorDefaults {
    pub default_base_url: String,
    pub base_url_placeholder: String,
    #[serde(default)]
    pub token_url: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub allowed_projects: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct BugzillaDefaults {
    pub default_base_url: String,
    pub base_url_placeholder: String,
    pub email_placeholder: String,
    #[serde(default)]
    pub token_url: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub allowed_products: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct AtlassianDefaults {
    pub default_base_url: String,
    pub base_url_placeholder: String,
    pub email_placeholder: String,
    #[serde(default)]
    pub token_url: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub allowed_jira_projects: Vec<String>,
    #[serde(default)]
    pub allowed_confluence_spaces: Vec<String>,
    #[serde(default)]
    pub excluded_jira_projects: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ClaudeDefaults {
    #[serde(default)]
    pub token_url: String,
}

static CONFIG: OnceLock<ServicesConfig> = OnceLock::new();

/// Loads service defaults from the TOML config file. Must be called once at startup.
pub fn load(path: &str) {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read services config at {}: {}", path, e));
    let config: ServicesConfig = toml::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse services config at {}: {}", path, e));
    CONFIG
        .set(config)
        .unwrap_or_else(|_| panic!("Services config already loaded"));
}

/// Returns the loaded services configuration.
pub fn get() -> &'static ServicesConfig {
    CONFIG
        .get()
        .expect("Services config not loaded. Call services_config::load() at startup.")
}
