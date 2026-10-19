use super::common::{self, TestApp};

use axum::http::StatusCode;
use serde_json::json;

/// A logged-in user with an active cycle that covers today.
async fn setup() -> (TestApp, i64, String) {
    let app = TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    sqlx::query(
        "INSERT INTO brag_phases (user_id, name, start_date, end_date, is_active) VALUES (?, 'Cycle', '2000-01-01', '2999-12-31', 1)",
    )
    .bind(user_id)
    .execute(&app.pool)
    .await
    .unwrap();
    let cookie = app.login(user_id).await;
    (app, user_id, cookie)
}

fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

// ── cycles ──

#[tokio::test]
async fn cycles_create_list_activate_delete() {
    let (app, _uid, cookie) = setup().await;

    let created = app
        .json(
            "POST",
            "/api/v1/cycles",
            Some(json!({"name": "H2 2026", "start_date": "2026-07-01", "end_date": "2026-12-31"})),
            Some(&cookie),
        )
        .await;
    assert_eq!(created.status, StatusCode::CREATED);
    let new_id = created.json()["id"].as_i64().unwrap();

    let list = app.get_json("/api/v1/cycles", Some(&cookie)).await.json();
    assert_eq!(list.as_array().unwrap().len(), 2);
    let active: Vec<_> = list
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["is_active"] == true)
        .collect();
    assert_eq!(active.len(), 1, "creating a cycle deactivates the previous one");
    assert_eq!(active[0]["id"], new_id);

    let wrong = app
        .json(
            "DELETE",
            &format!("/api/v1/cycles/{new_id}"),
            Some(json!({"confirm_name": "nope"})),
            Some(&cookie),
        )
        .await;
    assert_eq!(wrong.status, StatusCode::UNPROCESSABLE_ENTITY);

    let ok = app
        .json(
            "DELETE",
            &format!("/api/v1/cycles/{new_id}"),
            Some(json!({"confirm_name": "H2 2026"})),
            Some(&cookie),
        )
        .await;
    assert_eq!(ok.status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn cycle_validation_rejects_bad_dates() {
    let (app, _uid, cookie) = setup().await;
    let bad = app
        .json(
            "POST",
            "/api/v1/cycles",
            Some(json!({"name": "x", "start_date": "2026-12-31", "end_date": "2026-01-01"})),
            Some(&cookie),
        )
        .await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(bad.json()["error"]["code"], "invalid_input");
}

#[tokio::test]
async fn activating_or_deleting_another_users_cycle_is_404_and_keeps_active_cycle() {
    let (app, uid, cookie) = setup().await;
    let other = common::create_other_user(&app.pool).await;
    let foreign: i64 = sqlx::query_scalar(
        "INSERT INTO brag_phases (user_id, name, start_date, end_date, is_active) VALUES (?, 'Theirs', '2026-01-01', '2026-06-30', 1) RETURNING id",
    )
    .bind(other)
    .fetch_one(&app.pool)
    .await
    .unwrap();

    let r = app
        .json("POST", &format!("/api/v1/cycles/{foreign}/activate"), Some(json!({})), Some(&cookie))
        .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    let d = app
        .json(
            "DELETE",
            &format!("/api/v1/cycles/{foreign}"),
            Some(json!({"confirm_name": "Theirs"})),
            Some(&cookie),
        )
        .await;
    assert_eq!(d.status, StatusCode::NOT_FOUND);

    // The failed activation must not have deactivated the caller's own cycle.
    let me = app.get_json("/api/v1/me", Some(&cookie)).await.json();
    assert_eq!(me["active_cycle"]["name"], "Cycle");
    let _ = uid;
}

// ── settings ──

#[tokio::test]
async fn profile_update_clears_blank_fields_and_rejects_unknown_level() {
    let (app, _uid, cookie) = setup().await;
    let ok = app
        .json(
            "PUT",
            "/api/v1/settings/profile",
            Some(json!({"team": "Platform", "manager_name": "  ", "wants_promotion": true})),
            Some(&cookie),
        )
        .await;
    assert_eq!(ok.status, StatusCode::OK);
    let body = ok.json();
    assert_eq!(body["team"], "Platform");
    assert!(body["manager_name"].is_null());
    assert_eq!(body["wants_promotion"], true);

    let bad = app
        .json("PUT", "/api/v1/settings/profile", Some(json!({"role": "emperor"})), Some(&cookie))
        .await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn people_upsert_list_delete() {
    let (app, _uid, cookie) = setup().await;
    let r = app
        .json(
            "POST",
            "/api/v1/settings/people",
            Some(json!({"email": "sam@example.com", "display_name": "Sam", "team": "Platform"})),
            Some(&cookie),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);
    let id = r.json()[0]["id"].as_i64().unwrap();
    let del = app
        .json("DELETE", &format!("/api/v1/settings/people/{id}"), None, Some(&cookie))
        .await;
    assert_eq!(del.status, StatusCode::NO_CONTENT);
    let list = app.get_json("/api/v1/settings/people", Some(&cookie)).await.json();
    assert!(list.as_array().unwrap().is_empty());
}

// ── entries ──

async fn create_entry(app: &TestApp, cookie: &str, title: &str, date: &str) -> serde_json::Value {
    let r = app
        .json(
            "POST",
            "/api/v1/entries",
            Some(json!({"title": title, "entry_type": "development", "occurred_at": date,
                        "teams": ["Platform"], "collaborators": ["Sam"]})),
            Some(cookie),
        )
        .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    r.json()
}

#[tokio::test]
async fn entries_crud_roundtrip_splits_csv_into_arrays() {
    let (app, _uid, cookie) = setup().await;
    let e = create_entry(&app, &cookie, "Built the thing", &today()).await;
    assert_eq!(e["teams"], json!(["Platform"]));
    assert_eq!(e["collaborators"], json!(["Sam"]));
    let id = e["id"].as_i64().unwrap();

    let upd = app
        .json(
            "PUT",
            &format!("/api/v1/entries/{id}"),
            Some(json!({"title": "Built it better", "entry_type": "development", "occurred_at": today()})),
            Some(&cookie),
        )
        .await;
    assert_eq!(upd.status, StatusCode::OK, "{}", upd.body);
    assert_eq!(upd.json()["title"], "Built it better");
    assert_eq!(upd.json()["teams"], json!([]));

    let del = app.json("DELETE", &format!("/api/v1/entries/{id}"), None, Some(&cookie)).await;
    assert_eq!(del.status, StatusCode::NO_CONTENT);
    let gone = app.get_json(&format!("/api/v1/entries/{id}"), Some(&cookie)).await;
    assert_eq!(gone.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn entry_validation() {
    let (app, _uid, cookie) = setup().await;
    for body in [
        json!({"title": " ", "entry_type": "development"}),
        json!({"title": "x", "entry_type": "not-a-type"}),
        json!({"title": "x", "entry_type": "development", "occurred_at": "13/13/2026"}),
        json!({"title": "x", "entry_type": "development", "priority_id": 99999}),
    ] {
        let r = app.json("POST", "/api/v1/entries", Some(body), Some(&cookie)).await;
        assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", r.body);
    }
}

#[tokio::test]
async fn entries_list_pages_with_cursor_and_reports_total() {
    let (app, _uid, cookie) = setup().await;
    for i in 0..5 {
        create_entry(&app, &cookie, &format!("Entry {i}"), &today()).await;
    }
    let first = app.get_json("/api/v1/entries?limit=2", Some(&cookie)).await.json();
    assert_eq!(first["items"].as_array().unwrap().len(), 2);
    assert_eq!(first["total_matching"], 5);
    let cursor = first["next_cursor"].as_str().unwrap().to_string();

    let second = app
        .get_json(&format!("/api/v1/entries?limit=2&cursor={cursor}"), Some(&cookie))
        .await
        .json();
    let third_cursor = second["next_cursor"].as_str().unwrap().to_string();
    let third = app
        .get_json(&format!("/api/v1/entries?limit=2&cursor={third_cursor}"), Some(&cookie))
        .await
        .json();
    assert_eq!(third["items"].as_array().unwrap().len(), 1);
    assert!(third["next_cursor"].is_null());

    let mut ids: Vec<i64> = [&first, &second, &third]
        .iter()
        .flat_map(|p| p["items"].as_array().unwrap().iter().map(|e| e["id"].as_i64().unwrap()))
        .collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 5, "pages must not overlap or skip entries");
}

#[tokio::test]
async fn entries_filter_by_text_team_and_person() {
    let (app, _uid, cookie) = setup().await;
    create_entry(&app, &cookie, "Alpha work", &today()).await;
    app.json(
        "POST",
        "/api/v1/entries",
        Some(json!({"title": "Beta work", "entry_type": "development", "occurred_at": today(), "teams": ["Infra"]})),
        Some(&cookie),
    )
    .await;

    let q = app.get_json("/api/v1/entries?q=alpha", Some(&cookie)).await.json();
    assert_eq!(q["total_matching"], 1);
    let team = app.get_json("/api/v1/entries?team=Infra", Some(&cookie)).await.json();
    assert_eq!(team["items"][0]["title"], "Beta work");
    let person = app.get_json("/api/v1/entries?person=Sam", Some(&cookie)).await.json();
    assert_eq!(person["items"][0]["title"], "Alpha work");
    let none = app.get_json("/api/v1/entries?no_team=true", Some(&cookie)).await.json();
    assert_eq!(none["total_matching"], 0);
}

#[tokio::test]
async fn entries_of_other_users_are_invisible() {
    let (app, _uid, cookie) = setup().await;
    let e = create_entry(&app, &cookie, "Private", &today()).await;
    let id = e["id"].as_i64().unwrap();

    let other = common::create_other_user(&app.pool).await;
    let other_cookie = app.login(other).await;
    for (method, body) in [("GET", None), ("DELETE", None), ("POST", Some(json!({"meeting_role": "peer"})))] {
        let path = if method == "POST" {
            format!("/api/v1/entries/{id}/classify")
        } else {
            format!("/api/v1/entries/{id}")
        };
        let r = app.json(method, &path, body, Some(&other_cookie)).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{method} {path}");
    }
    let prep = app
        .get_json(&format!("/api/v1/meetings/{id}/prep"), Some(&other_cookie))
        .await;
    assert_eq!(prep.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn bulk_update_changes_teams_and_requires_ids() {
    let (app, _uid, cookie) = setup().await;
    let a = create_entry(&app, &cookie, "A", &today()).await["id"].as_i64().unwrap();
    let empty = app
        .json("POST", "/api/v1/entries/bulk", Some(json!({"ids": []})), Some(&cookie))
        .await;
    assert_eq!(empty.status, StatusCode::UNPROCESSABLE_ENTITY);
    let r = app
        .json(
            "POST",
            "/api/v1/entries/bulk",
            Some(json!({"ids": [a], "teams": ["Infra"], "merge_mode": "append"})),
            Some(&cookie),
        )
        .await;
    assert_eq!(r.json()["updated"], 1);
    let e = app.get_json(&format!("/api/v1/entries/{a}"), Some(&cookie)).await.json();
    assert_eq!(e["teams"], json!(["Platform", "Infra"]));
}

#[tokio::test]
async fn facets_list_teams_and_people_and_refresh_after_writes() {
    let (app, _uid, cookie) = setup().await;
    create_entry(&app, &cookie, "A", &today()).await;
    let f = app.get_json("/api/v1/entries/facets", Some(&cookie)).await.json();
    assert_eq!(f["teams"], json!(["Platform"]));
    assert_eq!(f["people"], json!(["Sam"]));

    app.json(
        "POST",
        "/api/v1/entries",
        Some(json!({"title": "B", "entry_type": "development", "teams": ["Infra"]})),
        Some(&cookie),
    )
    .await;
    let f2 = app.get_json("/api/v1/entries/facets", Some(&cookie)).await.json();
    assert_eq!(f2["teams"], json!(["Infra", "Platform"]), "writes invalidate the cache");
}

#[tokio::test]
async fn facets_list_only_categories_with_entries_in_display_order() {
    let (app, _uid, cookie) = setup().await;
    for (title, ty) in [("Fix", "jira_progress"), ("Sync", "meeting"), ("Call", "decision"), ("Plan", "development")] {
        let r = app
            .json("POST", "/api/v1/entries", Some(json!({"title": title, "entry_type": ty})), Some(&cookie))
            .await;
        assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    }
    let f = app.get_json("/api/v1/entries/facets", Some(&cookie)).await.json();
    let cats: Vec<(&str, i64)> = f["categories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (c["value"].as_str().unwrap(), c["count"].as_i64().unwrap()))
        .collect();
    assert_eq!(cats, vec![("code", 1), ("tickets", 1), ("meetings", 1), ("other", 1)]);

    // The filter accepts every offered value, including `other` (manual impact types).
    let r = app.get_json("/api/v1/entries?category=other", Some(&cookie)).await.json();
    let titles: Vec<&str> = r["items"].as_array().unwrap().iter().map(|e| e["title"].as_str().unwrap()).collect();
    assert_eq!(titles, vec!["Call"]);
}

#[tokio::test]
async fn entry_types_metadata_marks_manual_types() {
    let (app, _uid, cookie) = setup().await;
    let types = app.get_json("/api/v1/meta/entry-types", Some(&cookie)).await.json();
    let dev = types.as_array().unwrap().iter().find(|t| t["value"] == "development").unwrap();
    assert_eq!(dev["manual"], true);
    assert!(types.as_array().unwrap().len() >= 30);
}

// ── today and meeting prep ──

#[tokio::test]
async fn today_returns_week_columns_and_active_priorities() {
    let (app, _uid, cookie) = setup().await;
    let r = app.get_json("/api/v1/today", Some(&cookie)).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let body = r.json();
    assert_eq!(body["days"].as_array().unwrap().len(), 5);
    assert_eq!(body["cycle"]["name"], "Cycle");
    assert!(body["priorities"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn today_without_cycle_is_404() {
    let app = TestApp::new().await;
    let uid = common::create_test_user(&app.pool).await;
    let cookie = app.login(uid).await;
    let r = app.get_json("/api/v1/today", Some(&cookie)).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn meeting_prep_save_uses_entry_week_and_copies_notes() {
    let (app, _uid, cookie) = setup().await;
    let id = create_entry(&app, &cookie, "Sync", &today()).await["id"].as_i64().unwrap();
    let put = app
        .json(
            "PUT",
            &format!("/api/v1/meetings/{id}/prep"),
            Some(json!({"notes": "Talk about CI", "meeting_goal": "Decide"})),
            Some(&cookie),
        )
        .await;
    assert_eq!(put.status, StatusCode::OK, "{}", put.body);
    assert_eq!(put.json()["note"]["notes"], "Talk about CI");
    assert_eq!(put.json()["entry"]["description"], "Talk about CI");
}

#[tokio::test]
async fn prep_draft_without_ai_key_is_a_clear_error() {
    let (app, _uid, cookie) = setup().await;
    let id = create_entry(&app, &cookie, "Sync", &today()).await["id"].as_i64().unwrap();
    let r = app
        .json("POST", &format!("/api/v1/meetings/{id}/prep/draft"), Some(json!({})), Some(&cookie))
        .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(r.json()["error"]["message"].as_str().unwrap().contains("AI not configured"));
}

// ── cycle bundle ──

#[tokio::test]
async fn bundle_contains_review_priorities_and_entries_by_week() {
    let (app, _uid, cookie) = setup().await;
    create_entry(&app, &cookie, "Shipped it", &today()).await;
    let cycle_id = app.get_json("/api/v1/me", Some(&cookie)).await.json()["active_cycle"]["id"]
        .as_i64()
        .unwrap();

    let r = app.get_json(&format!("/api/v1/cycles/{cycle_id}/bundle"), Some(&cookie)).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let b = r.json();
    assert_eq!(b["cycle"]["name"], "Cycle");
    assert_eq!(b["weeks"].as_array().unwrap().len(), 1);
    assert_eq!(b["weeks"][0]["entries"][0]["title"], "Shipped it");
    assert!(!b["review"]["sections"].as_array().unwrap().is_empty());
    assert!(b["priorities"]["goals"].is_array());
}

#[tokio::test]
async fn bundle_of_another_users_cycle_is_404() {
    let (app, _uid, _cookie) = setup().await;
    let other = common::create_other_user(&app.pool).await;
    let other_cookie = app.login(other).await;
    let id: i64 = sqlx::query_scalar("SELECT id FROM brag_phases LIMIT 1")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    let r = app.get_json(&format!("/api/v1/cycles/{id}/bundle"), Some(&other_cookie)).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}
