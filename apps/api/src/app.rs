//! Composition root: builds the application state and router.

use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    Json, Router,
    extract::Request,
    http::{HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{any, get},
};
use serde_json::json;
use tower_http::compression::CompressionLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_sessions::{Expiry, SessionManagerLayer, cookie::SameSite};
use tower_sessions_sqlx_store::SqliteStore;

use crate::AppState;
use crate::identity::routes as identity_routes;
use crate::kernel::config::Config;
use crate::kernel::crypto::Crypto;
use crate::kernel::middleware::security_headers;

/// Resolves a config file path: checks the custom dir first, falls back to the config dir.
/// This allows organizations to override default config files by placing their
/// versions in the custom directory without modifying the defaults.
pub fn config_path(config: &Config, filename: &str) -> String {
    let custom = format!("{}/{}", config.custom_dir, filename);
    if std::path::Path::new(&custom).exists() {
        tracing::info!(path = %custom, "Using custom config override");
        custom
    } else {
        format!("{}/{}", config.config_dir, filename)
    }
}

/// Creates the application state: database pool, session store, crypto.
/// Returns `(AppState, SqliteStore)` so the caller can build the session layer.
pub async fn build_state(config: Config) -> (AppState, SqliteStore) {
    // Database
    let pool = crate::db::setup_pool(&config.database_path).await;
    crate::db::run_migrations(&pool).await;

    // Session store
    let session_store = SqliteStore::new(pool.clone());
    session_store
        .migrate()
        .await
        .expect("Failed to migrate session store");

    // Config files: check the custom overlay first, then fall back to the defaults
    crate::identity::clg::load_levels(&config_path(&config, "clg_levels.toml"));
    crate::review::model::load_review_config(&config_path(&config, "review_sections.toml"));
    crate::integrations::services_config::load(&config_path(&config, "services.toml"));

    // Crypto
    if config.encryption_key.is_empty() {
        panic!("BRAGFROG_ENCRYPTION_KEY must be set");
    }
    let crypto = Crypto::new(&config.encryption_key).expect("Failed to initialize crypto");

    // Post-SQL migrations that require encryption (e.g., KR name encryption)
    crate::db::run_post_migrations(&pool, &crypto).await;

    let state = AppState {
        db: pool,
        config: Arc::new(config),
        crypto: Arc::new(crypto),
        sync_status: crate::integrations::sync_status::new_sync_status_map(),
        entry_facets: crate::api::facets::new_facet_map(),
    };

    (state, session_store)
}

// ---------------------------------------------------------------------------
// Router assembly
// ---------------------------------------------------------------------------

/// Server-side routes: the OAuth redirects (they must stay server-side) and the JSON API.
/// Everything else is the web app, served by `build_app`.
pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/auth/login", get(identity_routes::login))
        .route("/auth/callback", get(identity_routes::callback))
        .route(
            "/auth/connect/google_drive",
            get(identity_routes::connect_google_drive),
        )
        .route(
            "/auth/connect/google_calendar",
            get(identity_routes::connect_google_calendar),
        )
        .merge(crate::api::public_router())
        .merge(crate::api::router())
        .route("/api/{*rest}", any(api_not_found))
}

async fn api_not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(json!({"error": {"code": "not_found", "message": "No such API route"}})),
    )
        .into_response()
}

fn custom_tokens_css_response(path: &std::path::Path) -> Response {
    let content_type = [(header::CONTENT_TYPE, "text/css; charset=utf-8")];

    match std::fs::read_to_string(path) {
        Ok(css) => (content_type, css).into_response(),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            // The custom overlay is optional. Return a valid empty stylesheet so
            // browsers with `nosniff` do not reject the response as HTML.
            (content_type, String::new()).into_response()
        }
        Err(err) => {
            tracing::warn!(error = %err, path = %path.display(), "Failed to read custom tokens CSS");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                "Failed to load custom tokens CSS",
            )
                .into_response()
        }
    }
}

/// Serves only `tokens.css` from the custom directory. The directory also holds operator
/// config (including AI prompts), so it must never be exposed as a static file tree.
fn custom_router<S: Clone + Send + Sync + 'static>(custom_dir: &str) -> Router<S> {
    let tokens = std::path::PathBuf::from(custom_dir).join("tokens.css");
    Router::new()
        .route(
            "/tokens.css",
            get(move || {
                let tokens = tokens.clone();
                async move { custom_tokens_css_response(&tokens) }
            }),
        )
        // Without this, other /custom paths would fall through to the web app's index.html.
        .fallback(|| async { StatusCode::NOT_FOUND })
}

/// Cache policy for the web app: hashed build assets never change, the HTML shell must
/// always be revalidated so a new release is picked up.
async fn static_cache_headers(request: Request, next: Next) -> Response {
    let path = request.uri().path().to_string();
    let mut response = next.run(request).await;
    let immutable = path.starts_with("/assets/") || path.starts_with("/fonts/");
    let value = if immutable {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    if response.status().is_success() {
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static(value));
    }
    response
}

/// Serves the built web app. Unknown paths return `index.html` so client-side routes work
/// after a reload.
fn web_app(dist: &str) -> Router {
    let dist = PathBuf::from(dist);
    let files = ServeDir::new(&dist)
        .precompressed_gzip()
        .fallback(ServeFile::new(dist.join("index.html")));
    Router::new()
        .fallback_service(files)
        .layer(middleware::from_fn(static_cache_headers))
}

/// Assembles the full Axum application: router, web app, sessions, security headers.
pub fn build_app(state: AppState, session_store: SqliteStore) -> Router {
    let is_production = state.config.base_url.starts_with("https://");
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(is_production)
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(time::Duration::hours(12)));

    let custom_files = custom_router(&state.config.custom_dir);
    let web = web_app(&state.config.web_dist);

    Router::new()
        .merge(create_router())
        .nest("/custom", custom_files)
        .with_state(state)
        .merge(web)
        .layer(session_layer)
        .layer(axum::middleware::from_fn(security_headers))
        .layer(CompressionLayer::new())
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;

    use super::*;

    #[tokio::test]
    async fn missing_custom_tokens_returns_empty_css() {
        let missing = std::env::temp_dir().join(format!(
            "bragfrog-missing-custom-tokens-{}-{}.css",
            std::process::id(),
            rand::random::<u64>()
        ));

        let response = custom_tokens_css_response(&missing);

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/css; charset=utf-8"
        );

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn custom_router_does_not_expose_config_files() {
        use tower::ServiceExt;

        let dir = std::env::temp_dir().join(format!("bragfrog-custom-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("review_sections.toml"), "prompt = \"secret\"").unwrap();

        let app: Router = custom_router(dir.to_str().unwrap());
        let request = |uri: &str| {
            axum::http::Request::builder()
                .uri(uri)
                .body(axum::body::Body::empty())
                .unwrap()
        };

        let toml = app.clone().oneshot(request("/review_sections.toml")).await.unwrap();
        assert_eq!(toml.status(), StatusCode::NOT_FOUND);

        let css = app.oneshot(request("/tokens.css")).await.unwrap();
        assert_eq!(css.status(), StatusCode::OK);
    }
}
