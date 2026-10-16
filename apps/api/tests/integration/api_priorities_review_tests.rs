//! JSON contract tests for priorities, goals, review and reports.

use super::common::{self, TestApp};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;

/// A signed-in user with an active cycle.
struct Fixture {
    app: TestApp,
    user_id: i64,
    cycle_id: i64,
    cookie: String,
}

impl Fixture {
    async fn new() -> Self {
        let app = TestApp::new().await;
        let user_id = common::create_test_user(&app.pool).await;
        let cycle_id = common::create_test_phase(&app.pool, user_id).await;
        let cookie = app.login(user_id).await;
        Self {
            app,
            user_id,
            cycle_id,
            cookie,
        }
    }

    async fn get(&self, path: &str) -> common::TestResponse {
        self.app.get_json(path, Some(&self.cookie)).await
    }

    async fn send(&self, method: &str, path: &str, body: Value) -> common::TestResponse {
        self.app
            .json(method, path, Some(body), Some(&self.cookie))
            .await
    }

    /// A second user with their own cycle, goal and priority, to probe ownership checks.
    async fn other_user_rows(&self) -> (i64, i64, i64) {
        let other = common::create_other_user(&self.app.pool).await;
        let cycle = common::create_test_phase(&self.app.pool, other).await;
        let crypto = self.app.crypto.for_user(other).unwrap();
        let goal =
            common::create_test_department_goal(&self.app.pool, cycle, other, "Theirs", &crypto)
                .await;
        let prio =
            common::create_test_priority(&self.app.pool, cycle, other, "Theirs", None, &crypto)
                .await;
        (cycle, goal.id, prio.id)
    }
}

// ── Priorities and goals ────────────────────────────────────────────────────

#[tokio::test]
async fn priorities_require_a_session() {
    let app = TestApp::new().await;
    let resp = app.get_json("/api/v1/priorities", None).await;
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
    assert_eq!(resp.json()["error"]["code"], "unauthorized");
}

#[tokio::test]
async fn priorities_list_groups_by_goal_with_counts() {
    let f = Fixture::new().await;
    let goal = f
        .send("POST", "/api/v1/goals", json!({"title": "Ship search"}))
        .await;
    assert_eq!(goal.status, StatusCode::CREATED);
    let goal_id = goal.json()["id"].as_i64().unwrap();

    let linked = f
        .send(
            "POST",
            "/api/v1/priorities",
            json!({"title": "Index rewrite", "status": "active", "department_goal_id": goal_id}),
        )
        .await;
    assert_eq!(linked.status, StatusCode::CREATED);
    let linked_id = linked.json()["id"].as_i64().unwrap();
    f.send(
        "POST",
        "/api/v1/priorities",
        json!({"title": "Loose end", "status": "completed"}),
    )
    .await;

    // One entry and one update on the linked priority drive entry_count and latest_update.
    let crypto = f.app.crypto.for_user(f.user_id).unwrap();
    let week =
        common::create_test_week(&f.app.pool, f.cycle_id, 3, 2025, "2025-01-13", "2025-01-19")
            .await;
    common::create_test_entry(
        &f.app.pool,
        f.user_id,
        week.id,
        "Did a thing",
        "development",
        "2025-01-14",
        Some(linked_id),
        &crypto,
    )
    .await;
    let upd = f
        .send(
            "POST",
            &format!("/api/v1/priorities/{linked_id}/updates"),
            json!({"tracking_status": "on_track", "comment": "going well"}),
        )
        .await;
    assert_eq!(upd.status, StatusCode::CREATED);

    let body = f.get("/api/v1/priorities").await.json();
    assert_eq!(body["cycle_id"], f.cycle_id);
    assert_eq!(body["counts"]["total"], 2);
    assert_eq!(body["counts"]["active"], 1);
    assert_eq!(body["counts"]["completed"], 1);

    let group = &body["goals"][0];
    assert_eq!(group["goal"]["title"], "Ship search");
    assert_eq!(group["priorities"][0]["title"], "Index rewrite");
    assert_eq!(group["priorities"][0]["entry_count"], 1);
    assert_eq!(
        group["priorities"][0]["latest_update"]["comment"],
        "going well"
    );
    assert_eq!(group["priorities"][0]["tracking_status"], "on_track");
    assert_eq!(body["unassigned"][0]["title"], "Loose end");
    assert!(body["unassigned"][0]["latest_update"].is_null());
}

#[tokio::test]
async fn priority_put_patches_only_sent_fields() {
    let f = Fixture::new().await;
    let created = f
        .send(
            "POST",
            "/api/v1/priorities",
            json!({"title": "Keep me", "scope": "large", "description": "details", "tier": "team"}),
        )
        .await
        .json();
    let id = created["id"].as_i64().unwrap();

    let updated = f
        .send(
            "PUT",
            &format!("/api/v1/priorities/{id}"),
            json!({"status": "on_hold", "description": null}),
        )
        .await;
    assert_eq!(updated.status, StatusCode::OK);
    let body = updated.json();
    assert_eq!(body["status"], "on_hold");
    assert_eq!(body["title"], "Keep me", "absent fields keep their value");
    assert_eq!(body["scope"], "large");
    assert!(body["description"].is_null(), "null clears the field");
}

#[tokio::test]
async fn priority_validation_errors_are_422() {
    let f = Fixture::new().await;
    let blank = f
        .send("POST", "/api/v1/priorities", json!({"title": "  "}))
        .await;
    assert_eq!(blank.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(blank.json()["error"]["code"], "invalid_input");

    let bad_status = f
        .send(
            "POST",
            "/api/v1/priorities",
            json!({"title": "x", "status": "nope"}),
        )
        .await;
    assert_eq!(bad_status.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn completing_a_goal_completes_its_priorities() {
    let f = Fixture::new().await;
    let goal_id = f
        .send("POST", "/api/v1/goals", json!({"title": "G"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let pid = f
        .send(
            "POST",
            "/api/v1/priorities",
            json!({"title": "P", "status": "active", "department_goal_id": goal_id}),
        )
        .await
        .json()["id"]
        .as_i64()
        .unwrap();

    let resp = f
        .send(
            "PUT",
            &format!("/api/v1/goals/{goal_id}"),
            json!({"status": "completed"}),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.json()["status"], "completed");

    let body = f.get("/api/v1/priorities").await.json();
    assert_eq!(body["goals"][0]["priorities"][0]["id"], pid);
    assert_eq!(body["goals"][0]["priorities"][0]["status"], "completed");
}

#[tokio::test]
async fn deleting_a_goal_unassigns_its_priorities() {
    let f = Fixture::new().await;
    let goal_id = f
        .send("POST", "/api/v1/goals", json!({"title": "G"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    f.send(
        "POST",
        "/api/v1/priorities",
        json!({"title": "P", "department_goal_id": goal_id}),
    )
    .await;

    let del = f
        .app
        .json(
            "DELETE",
            &format!("/api/v1/goals/{goal_id}"),
            None,
            Some(&f.cookie),
        )
        .await;
    assert_eq!(del.status, StatusCode::NO_CONTENT);

    let body = f.get("/api/v1/priorities").await.json();
    assert!(body["goals"].as_array().unwrap().is_empty());
    assert_eq!(body["unassigned"][0]["title"], "P");
}

#[tokio::test]
async fn other_users_ids_give_404() {
    let f = Fixture::new().await;
    let (other_cycle, other_goal, other_prio) = f.other_user_rows().await;

    assert_eq!(
        f.get(&format!("/api/v1/priorities?cycle_id={other_cycle}"))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        f.get(&format!("/api/v1/priorities/{other_prio}/updates"))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    for (method, path) in [
        ("PUT", format!("/api/v1/priorities/{other_prio}")),
        ("DELETE", format!("/api/v1/priorities/{other_prio}")),
        ("PUT", format!("/api/v1/goals/{other_goal}")),
        ("DELETE", format!("/api/v1/goals/{other_goal}")),
        ("POST", format!("/api/v1/priorities/{other_prio}/updates")),
    ] {
        let resp = f
            .send(method, &path, json!({"title": "hijack", "comment": "x"}))
            .await;
        assert_eq!(resp.status, StatusCode::NOT_FOUND, "{method} {path}");
        assert_eq!(resp.json()["error"]["code"], "not_found");
    }
    let create_in_other = f
        .send(
            "POST",
            "/api/v1/priorities",
            json!({"title": "x", "cycle_id": other_cycle}),
        )
        .await;
    assert_eq!(create_in_other.status, StatusCode::NOT_FOUND);

    // Attaching to another user's goal is refused too.
    let attach = f
        .send(
            "POST",
            "/api/v1/priorities",
            json!({"title": "x", "department_goal_id": other_goal}),
        )
        .await;
    assert_eq!(attach.status, StatusCode::NOT_FOUND);

    // Their rows are untouched.
    let crypto = f.app.crypto.for_user(f.user_id).unwrap();
    assert!(
        brag_frog::objectives::model::Priority::find_by_id(
            &f.app.pool,
            other_prio,
            f.user_id,
            &crypto
        )
        .await
        .unwrap()
        .is_none()
    );
}

#[tokio::test]
async fn empty_update_is_422() {
    let f = Fixture::new().await;
    let id = f
        .send("POST", "/api/v1/priorities", json!({"title": "P"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let resp = f
        .send(
            "POST",
            &format!("/api/v1/priorities/{id}/updates"),
            json!({}),
        )
        .await;
    assert_eq!(resp.status, StatusCode::UNPROCESSABLE_ENTITY);
}

fn multipart(csv: Option<&str>) -> (String, Body) {
    let boundary = "XBOUNDARYX";
    let mut body = String::new();
    if let Some(csv) = csv {
        body.push_str(&format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"csv_file\"; filename=\"g.csv\"\r\nContent-Type: text/csv\r\n\r\n{csv}\r\n"
        ));
    }
    body.push_str(&format!("--{boundary}--\r\n"));
    (
        format!("multipart/form-data; boundary={boundary}"),
        Body::from(body),
    )
}

async fn import(f: &Fixture, csv: Option<&str>) -> common::TestResponse {
    let (content_type, body) = multipart(csv);
    let req = Request::builder()
        .uri("/api/v1/priorities/import")
        .method("POST")
        .header("cookie", &f.cookie)
        .header("X-Requested-With", "brag-frog")
        .header("content-type", content_type)
        .body(body)
        .unwrap();
    let resp = f.app.app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    common::TestResponse {
        status,
        headers,
        body: String::from_utf8_lossy(&bytes).to_string(),
    }
}

#[tokio::test]
async fn lattice_import_reports_counts_as_data() {
    let f = Fixture::new().await;
    let csv = "Goal name,Goal type,Status,Goal ID\n\
               Company goal,Department,On track,D1\n\
               My work,Individual,On track,P1\n\
               ,Individual,On track,P2\n";
    let first = import(&f, Some(csv)).await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    let body = first.json();
    assert_eq!(body["created"], 2);
    assert_eq!(body["updated"], 0);
    assert_eq!(body["skipped"], 1);
    assert_eq!(body["errors"].as_array().unwrap().len(), 1);

    // Re-importing the same file updates by external id instead of duplicating.
    let again = import(&f, Some(csv)).await.json();
    assert_eq!(again["created"], 0);
    assert_eq!(again["updated"], 2);
}

#[tokio::test]
async fn lattice_import_without_file_is_422() {
    let f = Fixture::new().await;
    let resp = import(&f, None).await;
    assert_eq!(resp.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(resp.json()["error"]["code"], "invalid_input");
}

// ── Review ──────────────────────────────────────────────────────────────────

const SECTION: &str = "impact_examples";

#[tokio::test]
async fn review_lists_every_configured_section_with_entry_count() {
    let f = Fixture::new().await;
    let crypto = f.app.crypto.for_user(f.user_id).unwrap();
    let week =
        common::create_test_week(&f.app.pool, f.cycle_id, 3, 2025, "2025-01-13", "2025-01-19")
            .await;
    common::create_test_entry(
        &f.app.pool,
        f.user_id,
        week.id,
        "One",
        "development",
        "2025-01-14",
        None,
        &crypto,
    )
    .await;

    let resp = f
        .get(&format!("/api/v1/cycles/{}/review", f.cycle_id))
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    assert_eq!(body["entry_count"], 1);
    let sections = body["sections"].as_array().unwrap();
    assert_eq!(
        sections.len(),
        brag_frog::review::model::review_config().sections.len()
    );
    let s = sections.iter().find(|s| s["slug"] == SECTION).unwrap();
    assert!(s["content"].is_null());
    assert_eq!(s["content_format"], "blocknote");
    assert!(s["title"].as_str().is_some_and(|t| !t.is_empty()));
}

#[tokio::test]
async fn review_section_save_round_trips_content_and_format() {
    let f = Fixture::new().await;
    let path = format!("/api/v1/cycles/{}/review/{SECTION}", f.cycle_id);

    let saved = f
        .send(
            "PUT",
            &path,
            json!({"content": "[{\"type\":\"paragraph\"}]", "content_format": "blocknote"}),
        )
        .await;
    assert_eq!(saved.status, StatusCode::OK);
    assert_eq!(saved.json()["content_format"], "blocknote");

    // A second save replaces the first (one row per section).
    f.send(
        "PUT",
        &path,
        json!({"content": "# Plain", "content_format": "markdown"}),
    )
    .await;
    let body = f
        .get(&format!("/api/v1/cycles/{}/review", f.cycle_id))
        .await
        .json();
    let s = body["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["slug"] == SECTION)
        .unwrap()
        .clone();
    assert_eq!(s["content"], "# Plain");
    assert_eq!(s["content_format"], "markdown");
    assert!(s["generated_at"].is_string());

    // The content is encrypted at rest.
    let raw: Vec<u8> = sqlx::query_scalar("SELECT content FROM summaries WHERE phase_id = ?")
        .bind(f.cycle_id)
        .fetch_one(&f.app.pool)
        .await
        .unwrap();
    assert!(!String::from_utf8_lossy(&raw).contains("Plain"));
}

#[tokio::test]
async fn review_section_rejects_bad_input() {
    let f = Fixture::new().await;
    let bad_format = f
        .send(
            "PUT",
            &format!("/api/v1/cycles/{}/review/{SECTION}", f.cycle_id),
            json!({"content": "x", "content_format": "html"}),
        )
        .await;
    assert_eq!(bad_format.status, StatusCode::UNPROCESSABLE_ENTITY);

    let unknown = f
        .send(
            "PUT",
            &format!("/api/v1/cycles/{}/review/nope", f.cycle_id),
            json!({"content": "x", "content_format": "markdown"}),
        )
        .await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn review_endpoints_404_for_another_users_cycle() {
    let f = Fixture::new().await;
    let (other_cycle, _, _) = f.other_user_rows().await;
    assert_eq!(
        f.get(&format!("/api/v1/cycles/{other_cycle}/review"))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        f.get(&format!("/api/v1/cycles/{other_cycle}/archive"))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    let put = f
        .send(
            "PUT",
            &format!("/api/v1/cycles/{other_cycle}/review/{SECTION}"),
            json!({"content": "x", "content_format": "markdown"}),
        )
        .await;
    assert_eq!(put.status, StatusCode::NOT_FOUND);
    let draft = f
        .send(
            "POST",
            &format!("/api/v1/cycles/{other_cycle}/review/{SECTION}/draft"),
            json!({}),
        )
        .await;
    assert_eq!(draft.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn review_draft_without_ai_is_a_clear_400() {
    let f = Fixture::new().await;
    let resp = f
        .send(
            "POST",
            &format!("/api/v1/cycles/{}/review/{SECTION}/draft", f.cycle_id),
            json!({}),
        )
        .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    let body = resp.json();
    assert_eq!(body["error"]["code"], "bad_request");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("AI not configured")
    );
}

#[tokio::test]
async fn archive_returns_retired_data_with_linked_entry_titles() {
    use brag_frog::reflections::model::SaveQuarterlyCheckin;
    use brag_frog::review::model::{ContributionExample, CreateContributionExample};

    let f = Fixture::new().await;
    let crypto = f.app.crypto.for_user(f.user_id).unwrap();
    let week =
        common::create_test_week(&f.app.pool, f.cycle_id, 3, 2025, "2025-01-13", "2025-01-19")
            .await;
    let entry = common::create_test_entry(
        &f.app.pool,
        f.user_id,
        week.id,
        "Linked work",
        "development",
        "2025-01-14",
        None,
        &crypto,
    )
    .await;

    brag_frog::reflections::model::QuarterlyCheckin::upsert(
        &f.app.pool,
        f.cycle_id,
        f.user_id,
        &SaveQuarterlyCheckin {
            quarter: "Q1".into(),
            year: 2025,
            highlights_impact: Some("Big win".into()),
            learnings_adjustments: None,
            growth_development: None,
            support_feedback: None,
            looking_ahead: None,
        },
        &crypto,
    )
    .await
    .unwrap();
    let example = ContributionExample::create(
        &f.app.pool,
        f.cycle_id,
        f.user_id,
        &CreateContributionExample {
            title: "Search launch".into(),
            outcome: None,
            behaviors: None,
            impact_level: Some("team".into()),
            learnings: None,
            assessment_type: None,
        },
        &crypto,
    )
    .await
    .unwrap();
    ContributionExample::link_entry(&f.app.pool, example.id, entry.id)
        .await
        .unwrap();

    let body = f
        .get(&format!("/api/v1/cycles/{}/archive", f.cycle_id))
        .await
        .json();
    assert_eq!(body["checkins"][0]["highlights_impact"], "Big win");
    assert_eq!(body["contribution_examples"][0]["title"], "Search launch");
    assert_eq!(
        body["contribution_examples"][0]["entries"][0]["title"],
        "Linked work"
    );
    assert!(body["status_updates"].as_array().unwrap().is_empty());
    assert!(body["last_week_reports"].as_array().unwrap().is_empty());
}

// ── Reports ─────────────────────────────────────────────────────────────────

impl Fixture {
    /// Two priorities and four entries in January 2025 (cycle runs 2025-01-01 to 2025-06-30).
    async fn seed_report_data(&self) -> (i64, i64) {
        let crypto = self.app.crypto.for_user(self.user_id).unwrap();
        let a = common::create_test_priority(
            &self.app.pool,
            self.cycle_id,
            self.user_id,
            "Search",
            None,
            &crypto,
        )
        .await;
        let b = common::create_test_priority(
            &self.app.pool,
            self.cycle_id,
            self.user_id,
            "Ads",
            None,
            &crypto,
        )
        .await;
        let week = common::create_test_week(
            &self.app.pool,
            self.cycle_id,
            3,
            2025,
            "2025-01-13",
            "2025-01-19",
        )
        .await;
        for (title, date, prio) in [
            ("Index shards", "2025-01-14", Some(a.id)),
            ("Query planner", "2025-01-15", Some(a.id)),
            ("Ad auction", "2025-01-16", Some(b.id)),
            ("Misc", "2025-01-17", None),
        ] {
            common::create_test_entry(
                &self.app.pool,
                self.user_id,
                week.id,
                title,
                "development",
                date,
                prio,
                &crypto,
            )
            .await;
        }
        (a.id, b.id)
    }

    async fn create_report(&self, body: Value) -> common::TestResponse {
        self.send("POST", "/api/v1/reports", body).await
    }
}

#[tokio::test]
async fn reports_require_a_session() {
    let app = TestApp::new().await;
    assert_eq!(
        app.get_json("/api/v1/reports", None).await.status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn report_snapshot_has_stats_key_entries_and_default_title() {
    let f = Fixture::new().await;
    let (search, _) = f.seed_report_data().await;

    let resp = f
        .create_report(json!({"cycle_id": f.cycle_id, "filters": {}}))
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.body);
    let body = resp.json();
    assert_eq!(body["title"], "All projects · Jan 1 – Jun 30, 2025");
    assert_eq!(body["entry_count"], 4);
    assert_eq!(body["stats"]["entries"], 4);
    assert_eq!(body["stats"]["per_week"][0]["week_start"], "2025-01-13");
    assert_eq!(body["stats"]["per_week"][0]["count"], 4);
    let projects = body["stats"]["per_project"].as_array().unwrap();
    assert_eq!(projects[0]["name"], "Search");
    assert_eq!(projects[0]["priority_id"], search);
    assert_eq!(projects[0]["count"], 2);
    assert_eq!(body["key_entries"].as_array().unwrap().len(), 4);
    assert_eq!(body["narrative"]["content"], "");
    assert!(body["ai_error"].is_null());

    // Stored encrypted.
    let raw: Vec<u8> = sqlx::query_scalar("SELECT title FROM reports WHERE id = ?")
        .bind(body["id"].as_i64().unwrap())
        .fetch_one(&f.app.pool)
        .await
        .unwrap();
    assert!(!String::from_utf8_lossy(&raw).contains("All projects"));
}

#[tokio::test]
async fn report_filters_narrow_the_selection() {
    let f = Fixture::new().await;
    let (search, _) = f.seed_report_data().await;

    let by_priority = f
        .create_report(json!({"cycle_id": f.cycle_id, "filters": {"priority_id": search}}))
        .await
        .json();
    assert_eq!(by_priority["entry_count"], 2);
    assert_eq!(by_priority["title"], "Search · Jan 1 – Jun 30, 2025");
    assert_eq!(by_priority["filters"]["priority_id"], search);

    let by_text = f
        .create_report(json!({"cycle_id": f.cycle_id, "title": "  Mine  ", "filters": {"q": "auction", "from": "2025-01-15", "to": "2025-01-31"}}))
        .await
        .json();
    assert_eq!(by_text["entry_count"], 1);
    assert_eq!(by_text["title"], "Mine");
    assert_eq!(by_text["key_entries"][0]["title"], "Ad auction");

    let by_range = f
        .create_report(
            json!({"cycle_id": f.cycle_id, "filters": {"from": "2025-01-16", "to": "2025-01-16"}}),
        )
        .await
        .json();
    assert_eq!(by_range["entry_count"], 1);
}

#[tokio::test]
async fn report_with_empty_selection_is_422_and_saves_nothing() {
    let f = Fixture::new().await;
    f.seed_report_data().await;

    let resp = f
        .create_report(json!({"cycle_id": f.cycle_id, "filters": {"q": "does not exist"}}))
        .await;
    assert_eq!(resp.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(resp.json()["error"]["code"], "empty_selection");

    let list = f
        .get(&format!("/api/v1/reports?cycle_id={}", f.cycle_id))
        .await
        .json();
    assert!(list.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn report_rejects_bad_filters() {
    let f = Fixture::new().await;
    f.seed_report_data().await;
    for filters in [
        json!({"from": "yesterday"}),
        json!({"from": "2025-02-01", "to": "2025-01-01"}),
    ] {
        let resp = f
            .create_report(json!({"cycle_id": f.cycle_id, "filters": filters}))
            .await;
        assert_eq!(resp.status, StatusCode::UNPROCESSABLE_ENTITY, "{filters}");
        assert_eq!(resp.json()["error"]["code"], "invalid_input");
    }
}

#[tokio::test]
async fn report_with_ai_not_configured_is_still_created() {
    let f = Fixture::new().await;
    f.seed_report_data().await;

    let resp = f
        .create_report(json!({"cycle_id": f.cycle_id, "with_ai": true}))
        .await;
    assert_eq!(resp.status, StatusCode::CREATED);
    let body = resp.json();
    assert_eq!(body["narrative"]["content"], "");
    assert!(body["model_used"].is_null());
    assert!(
        body["ai_error"]
            .as_str()
            .unwrap()
            .contains("AI not configured")
    );

    // The report exists and later reads no longer carry the creation-time error.
    let id = body["id"].as_i64().unwrap();
    let fetched = f.get(&format!("/api/v1/reports/{id}")).await.json();
    assert!(fetched["ai_error"].is_null());
}

#[tokio::test]
async fn reports_list_newest_first_get_and_delete() {
    let f = Fixture::new().await;
    f.seed_report_data().await;
    let first = f
        .create_report(json!({"cycle_id": f.cycle_id, "title": "First"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let second = f
        .create_report(json!({"cycle_id": f.cycle_id, "title": "Second"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();

    let list = f
        .get(&format!("/api/v1/reports?cycle_id={}", f.cycle_id))
        .await
        .json();
    let titles: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["title"].as_str().unwrap())
        .collect();
    assert_eq!(titles, ["Second", "First"]);
    assert_eq!(list[0]["entry_count"], 4);
    assert_eq!(list[0]["id"], second);
    assert!(list[0].get("stats").is_none(), "list items stay light");

    // Without cycle_id the active cycle is used.
    assert_eq!(
        f.get("/api/v1/reports")
            .await
            .json()
            .as_array()
            .unwrap()
            .len(),
        2
    );

    let one = f.get(&format!("/api/v1/reports/{first}")).await;
    assert_eq!(one.status, StatusCode::OK);
    assert_eq!(one.json()["stats"]["entries"], 4);

    let del = f
        .app
        .json(
            "DELETE",
            &format!("/api/v1/reports/{first}"),
            None,
            Some(&f.cookie),
        )
        .await;
    assert_eq!(del.status, StatusCode::NO_CONTENT);
    assert_eq!(
        f.get(&format!("/api/v1/reports/{first}")).await.status,
        StatusCode::NOT_FOUND
    );
    let again = f
        .app
        .json(
            "DELETE",
            &format!("/api/v1/reports/{first}"),
            None,
            Some(&f.cookie),
        )
        .await;
    assert_eq!(again.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn stats_are_a_snapshot_that_ignores_later_edits() {
    let f = Fixture::new().await;
    f.seed_report_data().await;
    let id = f
        .create_report(json!({"cycle_id": f.cycle_id}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();

    sqlx::query("UPDATE brag_entries SET deleted_at = datetime('now')")
        .execute(&f.app.pool)
        .await
        .unwrap();
    let body = f.get(&format!("/api/v1/reports/{id}")).await.json();
    assert_eq!(body["stats"]["entries"], 4);
    assert_eq!(body["key_entries"].as_array().unwrap().len(), 4);
}

#[tokio::test]
async fn reports_enforce_ownership_on_every_id() {
    let f = Fixture::new().await;
    f.seed_report_data().await;
    let (other_cycle, other_goal, other_prio) = f.other_user_rows().await;

    // Another user's report.
    let other_user: i64 = sqlx::query_scalar("SELECT user_id FROM brag_phases WHERE id = ?")
        .bind(other_cycle)
        .fetch_one(&f.app.pool)
        .await
        .unwrap();
    let other_crypto = f.app.crypto.for_user(other_user).unwrap();
    let theirs = brag_frog::review::model::Report::create(
        &f.app.pool,
        other_user,
        other_cycle,
        &brag_frog::review::model::NewReport {
            title: "Theirs",
            filters_json: "{}",
            content: "",
            content_format: "markdown",
            stats_json: "{}",
            model_used: None,
        },
        &other_crypto,
    )
    .await
    .unwrap();
    assert_eq!(
        f.get(&format!("/api/v1/reports/{}", theirs.id))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    let del = f
        .app
        .json(
            "DELETE",
            &format!("/api/v1/reports/{}", theirs.id),
            None,
            Some(&f.cookie),
        )
        .await;
    assert_eq!(del.status, StatusCode::NOT_FOUND);
    let still: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reports WHERE id = ?")
        .bind(theirs.id)
        .fetch_one(&f.app.pool)
        .await
        .unwrap();
    assert_eq!(still, 1);

    // Another user's cycle, priority and goal.
    assert_eq!(
        f.get(&format!("/api/v1/reports?cycle_id={other_cycle}"))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    for body in [
        json!({"cycle_id": other_cycle}),
        json!({"cycle_id": f.cycle_id, "filters": {"priority_id": other_prio}}),
        json!({"cycle_id": f.cycle_id, "filters": {"goal_id": other_goal}}),
    ] {
        let resp = f.create_report(body.clone()).await;
        assert_eq!(resp.status, StatusCode::NOT_FOUND, "{body}");
    }
}

#[tokio::test]
async fn deleting_a_cycle_deletes_its_reports() {
    let f = Fixture::new().await;
    f.seed_report_data().await;
    f.create_report(json!({"cycle_id": f.cycle_id})).await;

    brag_frog::cycle::model::BragPhase::delete(&f.app.pool, f.cycle_id, f.user_id)
        .await
        .unwrap();
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reports")
        .fetch_one(&f.app.pool)
        .await
        .unwrap();
    assert_eq!(left, 0);
}
