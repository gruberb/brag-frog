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
    /// AI model identifier for summary generation. Default: `claude-sonnet-4-5-20250929`.
    pub ai_model: String,
    /// When true, sync services only fetch public/non-confidential data.
    /// Operator-level security policy via `BRAGFROG_PUBLIC_ONLY` env var.
    pub public_only: bool,
    /// Optional org/instance name (e.g., `Acme Corp`). When set, the login page shows
    /// "Brag Frog | {name}" in the nav bar.
    pub instance_name: Option<String>,
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
            ai_model: env::var("BRAGFROG_AI_MODEL")
                .unwrap_or_else(|_| "claude-sonnet-4-5-20250929".to_string()),
            public_only: env::var("BRAGFROG_PUBLIC_ONLY")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            instance_name: env::var("BRAGFROG_INSTANCE_NAME").ok(),
        }
    }
}
