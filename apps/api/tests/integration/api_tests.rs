use super::common;

use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn healthz_needs_no_session() {
    let app = common::TestApp::new().await;
    let resp = app.get("/healthz", None).await;
    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.body, "ok");
}

#[tokio::test]
async fn public_config_needs_no_session() {
    let app = common::TestApp::new().await;
    let resp = app.get_json("/api/v1/public-config", None).await;
    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.json()["public_only"], false);
}

#[tokio::test]
async fn me_without_session_is_json_401() {
    let app = common::TestApp::new().await;
    let resp = app.get_json("/api/v1/me", None).await;
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
    assert_eq!(resp.json()["error"]["code"], "unauthorized");
}

#[tokio::test]
async fn me_returns_user_and_active_cycle() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    let resp = app.get_json("/api/v1/me", Some(&cookie)).await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    assert_eq!(body["user"]["email"], "test@example.com");
    assert!(body["user"].get("google_id").is_none(), "google_id must not leak");
    assert_eq!(body["active_cycle"]["name"], "Test Phase");
    assert_eq!(body["has_ai"], false);
}

#[tokio::test]
async fn me_without_cycle_has_null_active_cycle() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let cookie = app.login(user_id).await;
    let body = app.get_json("/api/v1/me", Some(&cookie)).await.json();
    assert!(body["active_cycle"].is_null());
}

#[tokio::test]
async fn state_changing_api_request_requires_csrf_header() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let cookie = app.login(user_id).await;

    // Same request, no X-Requested-With header: built by hand.
    let req = axum::http::Request::builder()
        .uri("/api/v1/auth/logout")
        .method("POST")
        .header("cookie", &cookie)
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = tower::ServiceExt::oneshot(app.app.clone(), req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let ok = app.json("POST", "/api/v1/auth/logout", Some(json!({})), Some(&cookie)).await;
    assert_eq!(ok.status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn cross_site_post_is_refused_even_with_header() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let cookie = app.login(user_id).await;
    let req = axum::http::Request::builder()
        .uri("/api/v1/auth/logout")
        .method("POST")
        .header("cookie", &cookie)
        .header("X-Requested-With", "brag-frog")
        .header("Sec-Fetch-Site", "cross-site")
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = tower::ServiceExt::oneshot(app.app.clone(), req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
