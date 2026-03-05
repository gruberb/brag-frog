use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    extract::Path,
    http::{Request, StatusCode, header::HeaderMap},
    routing::get,
};
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

use brag_frog::AppState;
use brag_frog::db;
use brag_frog::worklog::model::BragEntry;
use brag_frog::worklog::model::CreateEntry;
use brag_frog::objectives::model::{
    CreateDepartmentGoal, CreatePriority, DepartmentGoal, Priority,
};
use brag_frog::cycle::model::Week;
use brag_frog::kernel::config::Config;
use brag_frog::kernel::crypto::{Crypto, UserCrypto};
use sqlx::SqlitePool;

/// Initialize OnceLock configs exactly once across all tests.
fn init_test_configs() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        brag_frog::identity::clg::load_levels("config/clg_levels.toml");
        brag_frog::cycle::model::initialize_config(|f| format!("config/{}", f));
        brag_frog::integrations::services_config::load("config/services.toml");
    });
}

/// Create a Config with dummy values for testing (no real OAuth needed).
fn test_config() -> Config {
    Config {
        database_path: ":memory:".to_string(),
        host: "127.0.0.1".to_string(),
        port: 0,
        google_client_id: "test-client-id".to_string(),
        google_client_secret: "test-client-secret".to_string(),
        google_redirect_uri: "http://localhost/auth/callback".to_string(),
        encryption_key: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_string(),
        allowed_domain: None,
        base_url: "http://localhost".to_string(),
        ai_model: "test-model".to_string(),
        public_only: false,
        instance_name: None,
    }
}

/// Full application wrapper for HTTP integration tests.
pub struct TestApp {
    pub app: Router,
    pub pool: SqlitePool,
    pub crypto: Arc<Crypto>,
}

/// Parsed HTTP response for test assertions.
pub struct TestResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: String,
}

impl TestApp {
    /// Builds a full app with in-memory SQLite, session support, and test login route.
    pub async fn new() -> Self {
        init_test_configs();

        let pool = db::setup_pool(":memory:").await;
        db::run_migrations(&pool).await;

        let config = Arc::new(test_config());
        let crypto = Arc::new(test_crypto());

        let mut templates =
            tera::Tera::new("templates/**/*.html").expect("Failed to load templates");
        brag_frog::register_tera_filters(&mut templates);
        // Register a no-op markdown filter for tests (avoids needing pulldown-cmark in lib)
        templates.register_filter(
            "markdown",
            |value: &tera::Value,
             _args: &std::collections::HashMap<String, tera::Value>|
             -> tera::Result<tera::Value> { Ok(value.clone()) },
        );

        let state = AppState {
            db: pool.clone(),
            config: config.clone(),
            templates: Arc::new(templates),
            crypto: crypto.clone(),
            sync_status: brag_frog::integrations::sync_status::new_sync_status_map(),
        };

        // Build app with a test-only login route
        let test_routes = Router::new().route("/test/login/{user_id}", get(test_login_handler));

        let session_store = MemoryStore::default();
        let session_layer = SessionManagerLayer::new(session_store);

        let app = Router::new()
            .merge(test_routes)
            .merge(brag_frog::app::create_router())
            .layer(session_layer)
            .with_state(state);

        TestApp { app, pool, crypto }
    }

    /// Performs a test login and returns the session cookie string.
    pub async fn login(&self, user_id: i64) -> String {
        let req = Request::builder()
            .uri(format!("/test/login/{}", user_id))
            .body(Body::empty())
            .unwrap();

        let resp = self.app.clone().oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK, "Test login should succeed");

        // Extract session cookie
        resp.headers()
            .get_all("set-cookie")
            .iter()
            .find_map(|v| {
                let s = v.to_str().ok()?;
                if s.contains("id=") {
                    // Return the cookie name=value part
                    Some(s.split(';').next().unwrap_or(s).to_string())
                } else {
                    None
                }
            })
            .expect("Login response should set a session cookie")
    }

    /// Sends a GET request with an optional session cookie.
    pub async fn get(&self, path: &str, cookie: Option<&str>) -> TestResponse {
        let mut builder = Request::builder().uri(path).method("GET");

        if let Some(c) = cookie {
            builder = builder.header("cookie", c);
        }

        let req = builder.body(Body::empty()).unwrap();
        self.send(req).await
    }

    /// Sends a POST with URL-encoded form body + HX-Request header.
    pub async fn post_form(&self, path: &str, body: &str, cookie: Option<&str>) -> TestResponse {
        let mut builder = Request::builder()
            .uri(path)
            .method("POST")
            .header("content-type", "application/x-www-form-urlencoded")
            .header("HX-Request", "true");

        if let Some(c) = cookie {
            builder = builder.header("cookie", c);
        }

        let req = builder.body(Body::from(body.to_string())).unwrap();
        self.send(req).await
    }

    /// Sends a PUT with URL-encoded form body + HX-Request header.
    pub async fn put_form(&self, path: &str, body: &str, cookie: Option<&str>) -> TestResponse {
        let mut builder = Request::builder()
            .uri(path)
            .method("PUT")
            .header("content-type", "application/x-www-form-urlencoded")
            .header("HX-Request", "true");

        if let Some(c) = cookie {
            builder = builder.header("cookie", c);
        }

        let req = builder.body(Body::from(body.to_string())).unwrap();
        self.send(req).await
    }

    /// Sends a DELETE with HX-Request header.
    pub async fn delete(&self, path: &str, cookie: Option<&str>) -> TestResponse {
        let mut builder = Request::builder()
            .uri(path)
            .method("DELETE")
            .header("HX-Request", "true");

        if let Some(c) = cookie {
            builder = builder.header("cookie", c);
        }

        let req = builder.body(Body::empty()).unwrap();
        self.send(req).await
    }

    async fn send(&self, req: Request<Body>) -> TestResponse {
        let resp = self.app.clone().oneshot(req).await.unwrap();

        let status = resp.status();
        let headers = resp.headers().clone();
        let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8_lossy(&body_bytes).to_string();

        TestResponse {
            status,
            headers,
            body,
        }
    }
}

/// Test-only handler: sets user_id in session and returns 200.
async fn test_login_handler(
    session: tower_sessions::Session,
    Path(user_id): Path<i64>,
) -> StatusCode {
    brag_frog::identity::auth::middleware::set_user_session(&session, user_id)
        .await
        .expect("Failed to set test session");
    StatusCode::OK
}

// ── Existing test helpers (preserved from original) ──

/// Create an in-memory SQLite pool with the full schema applied.
pub async fn test_pool() -> SqlitePool {
    let pool = db::setup_pool(":memory:").await;
    db::run_migrations(&pool).await;
    pool
}

/// Create a master Crypto instance with a fixed, deterministic key (for testing only).
pub fn test_crypto() -> Crypto {
    // 32 zero bytes, base64-encoded
    Crypto::new("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=").unwrap()
}

/// Derive a per-user crypto for tests. Uses user_id = 1 as default test user.
pub fn test_user_crypto(crypto: &Crypto, user_id: i64) -> UserCrypto {
    crypto.for_user(user_id).unwrap()
}

/// Insert a test user and return its id.
pub async fn create_test_user(pool: &SqlitePool) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO users (google_id, email, name) VALUES ('g-test-1', 'test@example.com', 'Test User') RETURNING id",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

/// Insert an active test phase for the given user and return its id.
pub async fn create_test_phase(pool: &SqlitePool, user_id: i64) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO brag_phases (user_id, name, start_date, end_date, is_active) VALUES (?, 'Test Phase', '2025-01-01', '2025-06-30', 1) RETURNING id",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// Create a department goal under the given phase and return it.
pub async fn create_test_department_goal(
    pool: &SqlitePool,
    phase_id: i64,
    user_id: i64,
    title: &str,
    crypto: &UserCrypto,
) -> DepartmentGoal {
    DepartmentGoal::create(
        pool,
        phase_id,
        user_id,
        &CreateDepartmentGoal {
            title: title.to_string(),
            description: None,
            status: None,
        },
        None,
        crypto,
    )
    .await
    .unwrap()
}

/// Create a priority, optionally under a department goal, and return it.
pub async fn create_test_priority(
    pool: &SqlitePool,
    phase_id: i64,
    user_id: i64,
    title: &str,
    department_goal_id: Option<i64>,
    crypto: &UserCrypto,
) -> Priority {
    Priority::create(
        pool,
        phase_id,
        user_id,
        &CreatePriority {
            title: title.to_string(),
            status: Some("active".to_string()),
            scope: None,
            impact_narrative: None,
            department_goal_id,
            priority_level: None,
            measure_type: None,
            measure_start: None,
            measure_target: None,
            description: None,
        },
        crypto,
    )
    .await
    .unwrap()
}

/// Create an entry in the given week and return it.
#[allow(clippy::too_many_arguments)]
pub async fn create_test_entry(
    pool: &SqlitePool,
    user_id: i64,
    week_id: i64,
    title: &str,
    entry_type: &str,
    occurred_at: &str,
    priority_id: Option<i64>,
    crypto: &UserCrypto,
) -> BragEntry {
    BragEntry::create(
        pool,
        &CreateEntry {
            week_id,
            priority_id,
            title: title.to_string(),
            description: None,
            entry_type: entry_type.to_string(),
            occurred_at: occurred_at.to_string(),
            teams: None,
            collaborators: None,
            source_url: None,
            reach: None,
            complexity: None,
            role: None,
        },
        user_id,
        crypto,
    )
    .await
    .unwrap()
}

/// Create a week for the given phase and return it.
pub async fn create_test_week(
    pool: &SqlitePool,
    phase_id: i64,
    iso_week: i64,
    year: i64,
    start_date: &str,
    end_date: &str,
) -> Week {
    Week::find_or_create(pool, phase_id, iso_week, year, start_date, end_date)
        .await
        .unwrap()
}
