use super::common;

use axum::http::StatusCode;
use brag_frog::integrations::model::IntegrationConfig;
use serde_json::{Value, json};

fn keys_recursive(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(m) => {
            for (k, v) in m {
                out.push(k.clone());
                keys_recursive(v, out);
            }
        }
        Value::Array(a) => a.iter().for_each(|v| keys_recursive(v, out)),
        _ => {}
    }
}

// ── config ──

#[tokio::test]
async fn config_exposes_ui_metadata_but_never_prompts() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let cookie = app.login(user_id).await;

    let resp = app.get_json("/api/v1/config", Some(&cookie)).await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();

    assert!(!body["review_sections"].as_array().unwrap().is_empty());
    let first = &body["review_sections"][0];
    assert!(first["title"].is_string() && first["form_bullets"].is_array());
    assert_eq!(first["focus_priorities"], true);
    assert!(!body["clg_levels"].as_array().unwrap().is_empty());
    assert!(body["clg_levels"][0]["problems"]["task_size"].is_string());
    let github = body["services"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["service"] == "github")
        .unwrap();
    assert!(github["token_url"].is_string());
    assert!(
        github["fields"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["name"] == "org")
    );

    let mut keys = Vec::new();
    keys_recursive(&body, &mut keys);
    for forbidden in [
        "prompt",
        "prompt_with_clg",
        "ai_prompt",
        "promotion_addendum",
    ] {
        assert!(!keys.iter().any(|k| k == forbidden), "{forbidden} leaked");
    }
    // The prompt text itself must not appear under another key either.
    assert!(!resp.body.contains("paste-ready answer"));
}

#[tokio::test]
async fn config_requires_a_session() {
    let app = common::TestApp::new().await;
    let resp = app.get_json("/api/v1/config", None).await;
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
}

// ── trends ──

#[tokio::test]
async fn trends_returns_typed_aggregates() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let phase_id = common::create_test_phase(&app.pool, user_id).await;
    let crypto = app.crypto.for_user(user_id).unwrap();
    let priority =
        common::create_test_priority(&app.pool, phase_id, user_id, "Ship it", None, &crypto).await;
    // 2025-01-06 is a Monday, 2025-01-13 the next one.
    let w1 =
        common::create_test_week(&app.pool, phase_id, 2, 2025, "2025-01-06", "2025-01-12").await;
    let w2 =
        common::create_test_week(&app.pool, phase_id, 3, 2025, "2025-01-13", "2025-01-19").await;
    for (week, ty, date, pri) in [
        (&w1, "pr_merged", "2025-01-06", Some(priority.id)),
        (&w1, "pr_reviewed", "2025-01-07", None),
        (&w2, "meeting", "2025-01-13", Some(priority.id)),
        (&w2, "document", "2025-01-14", None),
    ] {
        common::create_test_entry(&app.pool, user_id, week.id, "t", ty, date, pri, &crypto).await;
    }
    let cookie = app.login(user_id).await;

    let resp = app
        .get_json(
            &format!("/api/v1/trends?cycle_id={phase_id}&range=cycle"),
            Some(&cookie),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();

    assert_eq!(body["cycle_id"], phase_id);
    assert_eq!(body["range"], "cycle");
    assert_eq!(body["kpis"]["total"], 4);
    assert_eq!(body["kpis"]["prs_merged"], 1);
    assert_eq!(body["kpis"]["prs_reviewed"], 1);
    assert_eq!(body["kpis"]["meetings"], 1);
    assert_eq!(body["kpis"]["docs_worked_on"], 1);

    let weekly = body["weekly"].as_array().unwrap();
    let week = |d: &str| weekly.iter().find(|w| w["week_start"] == d).unwrap();
    assert_eq!(
        week("2025-01-06")["by_type"],
        json!({"code":1,"review":1,"meeting":0,"other":0})
    );
    assert_eq!(
        week("2025-01-13")["by_type"],
        json!({"code":0,"review":0,"meeting":1,"other":1})
    );
    // Zero-filled from the cycle start (Monday 2024-12-30).
    assert_eq!(weekly[0]["week_start"], "2024-12-30");

    let share = body["priority_share"].as_array().unwrap();
    let ship = share.iter().find(|s| s["title"] == "Ship it").unwrap();
    assert_eq!(ship["count"], 2);
    assert_eq!(ship["priority_id"], priority.id);
    assert!(
        share
            .iter()
            .any(|s| s["priority_id"].is_null() && s["count"] == 2)
    );
    assert!(ship.get("department_goal_title").is_some());

    assert_eq!(body["weekday"].as_array().unwrap().len(), 7);
    for key in ["people", "repos", "teams"] {
        assert!(body[key].is_array(), "{key}");
    }
    // Colours are a client concern.
    assert!(!resp.body.contains("#4A90D9"));
}

#[tokio::test]
async fn trends_four_week_range_excludes_old_entries() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let phase_id = common::create_test_phase(&app.pool, user_id).await;
    let crypto = app.crypto.for_user(user_id).unwrap();
    let w =
        common::create_test_week(&app.pool, phase_id, 2, 2025, "2025-01-06", "2025-01-12").await;
    common::create_test_entry(
        &app.pool,
        user_id,
        w.id,
        "old",
        "pr_merged",
        "2025-01-06",
        None,
        &crypto,
    )
    .await;
    let cookie = app.login(user_id).await;

    // The test cycle ended long before "today", so the window clamps to an empty range.
    let body = app
        .get_json("/api/v1/trends?range=4w", Some(&cookie))
        .await
        .json();
    assert_eq!(body["range"], "4w");
    assert_eq!(body["kpis"]["total"], 0);
}

#[tokio::test]
async fn trends_ownership_and_validation() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let other = common::create_other_user(&app.pool).await;
    let other_phase = common::create_test_phase(&app.pool, other).await;
    let cookie = app.login(user_id).await;

    let resp = app
        .get_json(
            &format!("/api/v1/trends?cycle_id={other_phase}"),
            Some(&cookie),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND);
    assert_eq!(resp.json()["error"]["code"], "not_found");

    // No active cycle for this user.
    assert_eq!(
        app.get_json("/api/v1/trends", Some(&cookie)).await.status,
        StatusCode::NOT_FOUND
    );

    let phase_id = common::create_test_phase(&app.pool, user_id).await;
    let bad = app
        .get_json(
            &format!("/api/v1/trends?cycle_id={phase_id}&range=decade"),
            Some(&cookie),
        )
        .await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(bad.json()["error"]["code"], "invalid_input");

    assert_eq!(
        app.get_json("/api/v1/trends", None).await.status,
        StatusCode::UNAUTHORIZED
    );
}

// ── integrations ──

#[tokio::test]
async fn integrations_list_shape() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let cookie = app.login(user_id).await;

    let body = app
        .get_json("/api/v1/integrations", Some(&cookie))
        .await
        .json();
    let services = body["services"].as_array().unwrap();
    assert_eq!(services.len(), 7);
    let atl = services
        .iter()
        .find(|s| s["service"] == "atlassian")
        .unwrap();
    assert_eq!(atl["is_enabled"], false);
    assert_eq!(atl["has_token"], false);
    assert_eq!(atl["entry_count"], 0);
    let completed = atl["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "sync_jira_completed")
        .unwrap();
    assert_eq!(
        completed["value"], true,
        "checkbox defaults are real booleans"
    );
    let claude = services.iter().find(|s| s["service"] == "claude").unwrap();
    assert_eq!(claude["token_only"], true);
}

#[tokio::test]
async fn saving_a_token_only_service_never_echoes_the_token() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let cookie = app.login(user_id).await;

    let resp = app
        .json(
            "PUT",
            "/api/v1/integrations/claude",
            Some(json!({"token": "sk-secret-value"})),
            Some(&cookie),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    assert_eq!(body["has_token"], true);
    assert_eq!(body["is_enabled"], true);
    assert!(!resp.body.contains("sk-secret-value"));

    let saved = IntegrationConfig::find_by_service(&app.pool, user_id, "claude")
        .await
        .unwrap()
        .unwrap();
    assert_ne!(saved.encrypted_token.unwrap(), b"sk-secret-value");
}

#[tokio::test]
async fn integration_write_errors() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let cookie = app.login(user_id).await;

    let unknown = app
        .json(
            "PUT",
            "/api/v1/integrations/nope",
            Some(json!({})),
            Some(&cookie),
        )
        .await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);

    // Atlassian needs a token and none is saved.
    let no_token = app
        .json(
            "POST",
            "/api/v1/integrations/atlassian/test",
            Some(json!({})),
            Some(&cookie),
        )
        .await;
    assert_eq!(no_token.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(no_token.json()["error"]["code"], "invalid_input");

    let save_no_token = app
        .json(
            "PUT",
            "/api/v1/integrations/atlassian",
            Some(json!({})),
            Some(&cookie),
        )
        .await;
    assert_eq!(save_no_token.status, StatusCode::UNPROCESSABLE_ENTITY);

    let anon = app
        .json("PUT", "/api/v1/integrations/claude", Some(json!({})), None)
        .await;
    assert_eq!(anon.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn reset_removes_the_config_and_optionally_entries() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let cookie = app.login(user_id).await;
    app.json(
        "PUT",
        "/api/v1/integrations/claude",
        Some(json!({"token": "x"})),
        Some(&cookie),
    )
    .await;

    let resp = app
        .json(
            "DELETE",
            "/api/v1/integrations/claude?delete_entries=true",
            None,
            Some(&cookie),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT);
    let body = app
        .get_json("/api/v1/integrations", Some(&cookie))
        .await
        .json();
    let claude = body["services"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["service"] == "claude")
        .unwrap()
        .clone();
    assert_eq!(claude["has_token"], false);
}

#[tokio::test]
async fn excluded_items_are_listed_and_restorable() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    IntegrationConfig::upsert(
        &app.pool,
        user_id,
        "google_drive",
        true,
        None,
        Some(r#"{"excluded_files":[{"file_id":"f1","title":"Plan"},{"file_id":"f2","title":"Notes"}]}"#),
    )
    .await
    .unwrap();
    IntegrationConfig::upsert(
        &app.pool,
        user_id,
        "google_calendar",
        true,
        None,
        Some(r#"{"excluded_events":[{"event_id":"e1","title":"Standup"}]}"#),
    )
    .await
    .unwrap();
    let cookie = app.login(user_id).await;

    let find = |body: &Value, s: &str| {
        body["services"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["service"] == s)
            .unwrap()
            .clone()
    };
    let body = app
        .get_json("/api/v1/integrations", Some(&cookie))
        .await
        .json();
    assert_eq!(
        find(&body, "google_drive")["excluded_files"],
        json!([{"id":"f1","title":"Plan"},{"id":"f2","title":"Notes"}])
    );
    assert_eq!(
        find(&body, "google_calendar")["excluded_events"][0]["id"],
        "e1"
    );

    let resp = app
        .json(
            "DELETE",
            "/api/v1/integrations/google_drive/excluded/f1",
            None,
            Some(&cookie),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT);
    let resp = app
        .json(
            "DELETE",
            "/api/v1/integrations/google_calendar/excluded/e1",
            None,
            Some(&cookie),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT);

    let body = app
        .get_json("/api/v1/integrations", Some(&cookie))
        .await
        .json();
    assert_eq!(
        find(&body, "google_drive")["excluded_files"],
        json!([{"id":"f2","title":"Notes"}])
    );
    assert_eq!(find(&body, "google_calendar")["excluded_events"], json!([]));

    let bad = app
        .json(
            "DELETE",
            "/api/v1/integrations/github/excluded/x",
            None,
            Some(&cookie),
        )
        .await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ── sync ──

#[tokio::test]
async fn sync_all_returns_202_and_status_settles() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let cookie = app.login(user_id).await;

    let resp = app.json("POST", "/api/v1/sync", None, Some(&cookie)).await;
    assert_eq!(resp.status, StatusCode::ACCEPTED);
    assert_eq!(resp.json()["started"], true);

    // No enabled services: the background task finishes immediately and clears the flag.
    let mut running = true;
    for _ in 0..50 {
        let status = app.get_json("/api/v1/sync/status", Some(&cookie)).await;
        assert_eq!(status.status, StatusCode::OK);
        let body = status.json();
        assert!(body["services"].is_array() && body["recent_logs"].is_array());
        running = body["running"].as_bool().unwrap();
        if !running {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(!running);
}

#[tokio::test]
async fn sync_service_validates_before_accepting() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let cookie = app.login(user_id).await;

    let unknown = app
        .json("POST", "/api/v1/sync/nope", None, Some(&cookie))
        .await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
    // The claude service has no sync implementation.
    let token_only = app
        .json("POST", "/api/v1/sync/claude", None, Some(&cookie))
        .await;
    assert_eq!(token_only.status, StatusCode::NOT_FOUND);
    let not_connected = app
        .json("POST", "/api/v1/sync/github?hard=true", None, Some(&cookie))
        .await;
    assert_eq!(not_connected.status, StatusCode::UNPROCESSABLE_ENTITY);

    let anon = app.json("POST", "/api/v1/sync", None, None).await;
    assert_eq!(anon.status, StatusCode::UNAUTHORIZED);
}
