use std::env;

/// Application configuration populated from `BRAGFROG_*` environment variables (with `.env` fallback).
#[derive(Clone)]
pub struct Config {
    /// Path to the SQLite database file. Default: `bragfrog.db`.
    pub database_path: String,
    /// Bind address. Default: `0.0.0.0`.
    pub host: String,
    /// Listen port. Reads `BRAGFROG_PORT`, then `PORT`, default `8080`.
    pub port: u16,
    /// Google OAuth 2.0 client ID (required).
    pub google_client_id: String,
    /// Google OAuth 2.0 client secret (required).
    pub google_client_secret: String,
    /// OAuth redirect URI. Default: `{base_url}/auth/callback`.
    pub google_redirect_uri: String,
    /// Base64-encoded 32-byte AES-256-GCM key for token encryption (required).
    pub encryption_key: String,
    /// Optional email domain restriction for sign-ups (e.g., `your-company.com`).
    pub allowed_domain: Option<String>,
    /// Public base URL for constructing absolute links. Default: `http://localhost:{port}`.
    pub base_url: String,
    /// Model and request settings for every AI call.
    pub ai: AiSettings,
    /// When true, sync services only fetch public/non-confidential data.
    /// Operator-level security policy via `BRAGFROG_PUBLIC_ONLY` env var.
    pub public_only: bool,
    /// Optional org/instance name (e.g., `Acme Corp`). When set, the login page shows
    /// "Brag Frog | {name}" in the nav bar.
    pub instance_name: Option<String>,
    /// Directory with the default TOML configs. Default: `config`.
    pub config_dir: String,
    /// Directory with optional operator overrides (TOML files, `tokens.css`). Default: `custom`.
    pub custom_dir: String,
    /// Directory with the built web app (`index.html`, `assets/`). Default: `../web/dist`.
    pub web_dist: String,
}

impl Config {
    /// Loads config from environment variables (`.env` file supported via dotenvy).
    /// Panics on missing required vars.
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let host = env::var("BRAGFROG_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port: u16 = env::var("BRAGFROG_PORT")
            .unwrap_or_else(|_| env::var("PORT").unwrap_or_else(|_| "8080".to_string()))
            .parse()
            .expect("PORT must be a number");

        let base_url =
            env::var("BRAGFROG_BASE_URL").unwrap_or_else(|_| format!("http://localhost:{}", port));

        let google_redirect_uri = env::var("BRAGFROG_GOOGLE_REDIRECT_URI")
            .unwrap_or_else(|_| format!("{}/auth/callback", base_url));

        Self {
            database_path: env::var("BRAGFROG_DATABASE_PATH")
                .unwrap_or_else(|_| "bragfrog.db".to_string()),
            host,
            port,
            google_client_id: env::var("BRAGFROG_GOOGLE_CLIENT_ID")
                .expect("BRAGFROG_GOOGLE_CLIENT_ID must be set"),
            google_client_secret: env::var("BRAGFROG_GOOGLE_CLIENT_SECRET")
                .expect("BRAGFROG_GOOGLE_CLIENT_SECRET must be set"),
            google_redirect_uri,
            encryption_key: env::var("BRAGFROG_ENCRYPTION_KEY").unwrap_or_default(),
            allowed_domain: env::var("BRAGFROG_ALLOWED_DOMAIN").ok(),
            base_url,
            ai: AiSettings::from_env(),
            public_only: env::var("BRAGFROG_PUBLIC_ONLY")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            instance_name: env::var("BRAGFROG_INSTANCE_NAME").ok(),
            config_dir: env::var("BRAGFROG_CONFIG_DIR").unwrap_or_else(|_| "config".to_string()),
            custom_dir: env::var("BRAGFROG_CUSTOM_DIR").unwrap_or_else(|_| "custom".to_string()),
            web_dist: env::var("BRAGFROG_WEB_DIST").unwrap_or_else(|_| "../web/dist".to_string()),
        }
    }
}

/// Operator settings for the Anthropic Messages API. Users bring their own API key; the
/// operator picks the model and how hard it works.
#[derive(Debug, Clone)]
pub struct AiSettings {
    /// `BRAGFROG_AI_MODEL`. Default: `claude-opus-5-5`.
    pub model: String,
    /// `BRAGFROG_AI_EFFORT` (`low`, `medium`, `high`, `xhigh`, `max`), sent as
    /// `output_config.effort`. Default: `medium`. Empty leaves it to the model's default.
    pub effort: Option<String>,
    /// `BRAGFROG_AI_FALLBACKS`, sent as `fallbacks`. Default: `default`, which lets the API
    /// retry a safety-classifier refusal on the model Anthropic recommends for that category.
    /// Empty turns it off, for models without server-side fallback.
    pub fallbacks: Option<String>,
}

impl AiSettings {
    const EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max"];

    /// Reads the settings from the environment. Panics on an unknown effort level, like the
    /// other required settings: a typo would otherwise fail every AI request at runtime.
    fn from_env() -> Self {
        let optional = |name: &str, default: &str| {
            let value = env::var(name).unwrap_or_else(|_| default.to_string());
            Some(value.trim().to_string()).filter(|v| !v.is_empty())
        };
        let effort = optional("BRAGFROG_AI_EFFORT", "medium");
        if let Some(e) = effort.as_deref() {
            assert!(
                Self::EFFORTS.contains(&e),
                "BRAGFROG_AI_EFFORT must be one of {:?} or empty, got {e:?}",
                Self::EFFORTS
            );
        }
        Self {
            model: optional("BRAGFROG_AI_MODEL", "claude-opus-5-5")
                .unwrap_or_else(|| "claude-opus-5-5".to_string()),
            effort,
            fallbacks: optional("BRAGFROG_AI_FALLBACKS", "default"),
        }
    }
}
