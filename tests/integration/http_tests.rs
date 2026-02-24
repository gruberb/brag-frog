use super::common;

use axum::http::StatusCode;
use tower::ServiceExt;

// ── Auth & Navigation ──

#[tokio::test]
async fn test_unauthenticated_redirect() {
    let app = common::TestApp::new().await;

    // GET /logbook without session → redirect to landing (/)
    let resp = app.get("/logbook", None).await;
    assert_eq!(resp.status, StatusCode::SEE_OTHER);
    let location = resp.headers.get("location").unwrap().to_str().unwrap();
    assert_eq!(location, "/");
}

#[tokio::test]
async fn test_logbook_page_loads() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let _phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    let resp = app.get("/logbook", Some(&cookie)).await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(
        resp.body.contains("Logbook"),
        "Page should contain 'Logbook'"
    );
}

#[tokio::test]
async fn test_logbook_no_phase_shows_no_phase_page() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    // No phase created
    let cookie = app.login(user_id).await;

    let resp = app.get("/logbook", Some(&cookie)).await;
    assert_eq!(resp.status, StatusCode::OK);
    // Should render the no-phase page
    assert!(
        resp.body.contains("phase") || resp.body.contains("Phase") || resp.body.contains("cycle"),
        "Should show no-phase content"
    );
}

// ── Quick-add entry ──

#[tokio::test]
async fn test_create_entry_via_quick_add() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let _phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    let body = "title=My+Test+Entry&entry_type=meeting&occurred_at=2025-03-15";
    let resp = app.post_form("/entries/quick", body, Some(&cookie)).await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(
        resp.body.contains("My Test Entry"),
        "Response should contain entry title"
    );

    // Verify in DB
    let user_crypto = app.crypto.for_user(user_id).unwrap();
    let entries =
        brag_frog::entries::model::BragEntry::list_for_phase(&app.pool, _phase_id, &user_crypto)
            .await
            .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].title, "My Test Entry");
    assert_eq!(entries[0].entry_type, "meeting");
}

#[tokio::test]
async fn test_create_entry_with_priority() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    let user_crypto = app.crypto.for_user(user_id).unwrap();
    let priority = common::create_test_priority(
        &app.pool,
        phase_id,
        user_id,
        "Ship feature",
        None,
        &user_crypto,
    )
    .await;

    let body = format!(
        "title=PR+for+feature&entry_type=pr_authored&occurred_at=2025-03-15&priority_id={}",
        priority.id
    );
    let resp = app.post_form("/entries/quick", &body, Some(&cookie)).await;
    assert_eq!(resp.status, StatusCode::OK);

    let entries =
        brag_frog::entries::model::BragEntry::list_for_phase(&app.pool, phase_id, &user_crypto)
            .await
            .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].priority_id, Some(priority.id));
}

#[tokio::test]
async fn test_create_entry_with_teams_and_collaborators() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let _phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    let body = "title=Team+Work&entry_type=meeting&occurred_at=2025-03-15&teams=Backend,Platform&collaborators=Alice,Bob";
    let resp = app.post_form("/entries/quick", body, Some(&cookie)).await;
    assert_eq!(resp.status, StatusCode::OK);

    let user_crypto = app.crypto.for_user(user_id).unwrap();
    let entries =
        brag_frog::entries::model::BragEntry::list_for_phase(&app.pool, _phase_id, &user_crypto)
            .await
            .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].teams.as_deref(), Some("Backend,Platform"));
    assert_eq!(entries[0].collaborators.as_deref(), Some("Alice,Bob"));
}

// ── Entry view/edit ──

#[tokio::test]
async fn test_view_entry_card() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    let user_crypto = app.crypto.for_user(user_id).unwrap();
    let week =
        common::create_test_week(&app.pool, phase_id, 10, 2025, "2025-03-03", "2025-03-09").await;
    let entry = common::create_test_entry(
        &app.pool,
        user_id,
        week.id,
        "View Me",
        "meeting",
        "2025-03-04",
        None,
        &user_crypto,
    )
    .await;

    let resp = app
        .get(&format!("/entries/{}/view", entry.id), Some(&cookie))
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(
        resp.body.contains("View Me"),
        "Response should contain entry title"
    );
}

#[tokio::test]
async fn test_update_entry() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    let user_crypto = app.crypto.for_user(user_id).unwrap();
    let week =
        common::create_test_week(&app.pool, phase_id, 10, 2025, "2025-03-03", "2025-03-09").await;
    let entry = common::create_test_entry(
        &app.pool,
        user_id,
        week.id,
        "Original Title",
        "meeting",
        "2025-03-04",
        None,
        &user_crypto,
    )
    .await;

    let body = "title=Updated+Title&entry_type=meeting&occurred_at=2025-03-04";
    let resp = app
        .put_form(&format!("/entries/{}", entry.id), body, Some(&cookie))
        .await;
    assert_eq!(resp.status, StatusCode::OK);

    // Verify in DB
    let updated = brag_frog::entries::model::BragEntry::find_by_id(
        &app.pool,
        entry.id,
        user_id,
        &user_crypto,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(updated.title, "Updated Title");
}

#[tokio::test]
async fn test_update_entry_type() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    let user_crypto = app.crypto.for_user(user_id).unwrap();
    let week =
        common::create_test_week(&app.pool, phase_id, 10, 2025, "2025-03-03", "2025-03-09").await;
    let entry = common::create_test_entry(
        &app.pool,
        user_id,
        week.id,
        "Change Type",
        "meeting",
        "2025-03-04",
        None,
        &user_crypto,
    )
    .await;

    let body = "title=Change+Type&entry_type=presentation&occurred_at=2025-03-04";
    let resp = app
        .put_form(&format!("/entries/{}", entry.id), body, Some(&cookie))
        .await;
    assert_eq!(resp.status, StatusCode::OK);

    let updated = brag_frog::entries::model::BragEntry::find_by_id(
        &app.pool,
        entry.id,
        user_id,
        &user_crypto,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(updated.entry_type, "presentation");
}

#[tokio::test]
async fn test_update_entry_priority() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    let user_crypto = app.crypto.for_user(user_id).unwrap();
    let week =
        common::create_test_week(&app.pool, phase_id, 10, 2025, "2025-03-03", "2025-03-09").await;
    let entry = common::create_test_entry(
        &app.pool,
        user_id,
        week.id,
        "Assign Priority",
        "meeting",
        "2025-03-04",
        None,
        &user_crypto,
    )
    .await;
    assert_eq!(entry.priority_id, None);

    let priority = common::create_test_priority(
        &app.pool,
        phase_id,
        user_id,
        "Priority for update",
        None,
        &user_crypto,
    )
    .await;

    let body = format!(
        "title=Assign+Priority&entry_type=meeting&occurred_at=2025-03-04&priority_id={}",
        priority.id
    );
    let resp = app
        .put_form(&format!("/entries/{}", entry.id), &body, Some(&cookie))
        .await;
    assert_eq!(resp.status, StatusCode::OK);

    let updated = brag_frog::entries::model::BragEntry::find_by_id(
        &app.pool,
        entry.id,
        user_id,
        &user_crypto,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(updated.priority_id, Some(priority.id));
}

// ── Entry deletion ──

#[tokio::test]
async fn test_delete_entry() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    let user_crypto = app.crypto.for_user(user_id).unwrap();
    let week =
        common::create_test_week(&app.pool, phase_id, 10, 2025, "2025-03-03", "2025-03-09").await;
    let entry = common::create_test_entry(
        &app.pool,
        user_id,
        week.id,
        "Delete Me",
        "meeting",
        "2025-03-04",
        None,
        &user_crypto,
    )
    .await;

    let resp = app
        .delete(&format!("/entries/{}", entry.id), Some(&cookie))
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.body.is_empty(), "Delete should return empty body");

    // Manual entries are hard-deleted
    let gone = brag_frog::entries::model::BragEntry::find_by_id(
        &app.pool,
        entry.id,
        user_id,
        &user_crypto,
    )
    .await
    .unwrap();
    assert!(gone.is_none());
}

#[tokio::test]
async fn test_delete_entry_not_found() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let _phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    let resp = app.delete("/entries/99999", Some(&cookie)).await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND);
}

// ── Goals/Priority integration ──

#[tokio::test]
async fn test_logbook_shows_priorities_in_dropdown() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    let user_crypto = app.crypto.for_user(user_id).unwrap();
    let goal = common::create_test_department_goal(
        &app.pool,
        phase_id,
        user_id,
        "Ship OHTTP",
        &user_crypto,
    )
    .await;
    let _priority = common::create_test_priority(
        &app.pool,
        phase_id,
        user_id,
        "Viaduct component",
        Some(goal.id),
        &user_crypto,
    )
    .await;

    let resp = app.get("/logbook", Some(&cookie)).await;
    assert_eq!(resp.status, StatusCode::OK);
    // The logbook page should contain the priority title
    assert!(
        resp.body.contains("Viaduct component"),
        "Should show priority in dropdown"
    );
}

#[tokio::test]
async fn test_entry_card_shows_type_label() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    let user_crypto = app.crypto.for_user(user_id).unwrap();
    let week =
        common::create_test_week(&app.pool, phase_id, 10, 2025, "2025-03-03", "2025-03-09").await;
    let entry = common::create_test_entry(
        &app.pool,
        user_id,
        week.id,
        "Team standup",
        "meeting",
        "2025-03-04",
        None,
        &user_crypto,
    )
    .await;

    let resp = app
        .get(&format!("/entries/{}/view", entry.id), Some(&cookie))
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    // The entry_type_label filter should render "Meeting" from "meeting"
    assert!(
        resp.body.contains("Meeting"),
        "Should display human-readable type label"
    );
}

// ── Settings ──

#[tokio::test]
async fn test_settings_page_loads() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let _phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    let resp = app.get("/settings", Some(&cookie)).await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(
        resp.body.contains("Settings") || resp.body.contains("settings"),
        "Page should contain 'Settings'"
    );
}

// ── CSRF protection ──

#[tokio::test]
async fn test_csrf_rejects_post_without_hx_header() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let _phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    // Send POST without HX-Request header (simulating direct form post)
    let req = axum::http::Request::builder()
        .uri("/entries/quick")
        .method("POST")
        .header("content-type", "application/x-www-form-urlencoded")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(
            "title=test&entry_type=meeting&occurred_at=2025-03-15",
        ))
        .unwrap();

    let resp = app.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ── Dashboard ──

#[tokio::test]
async fn test_dashboard_page_loads() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let _phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    let resp = app.get("/dashboard", Some(&cookie)).await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(
        resp.body.contains("Dashboard"),
        "Page should contain 'Dashboard'"
    );
}

#[tokio::test]
async fn test_dashboard_no_phase_shows_no_phase_page() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let cookie = app.login(user_id).await;

    let resp = app.get("/dashboard", Some(&cookie)).await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(
        resp.body.contains("phase") || resp.body.contains("Phase") || resp.body.contains("cycle"),
        "Should show no-phase content"
    );
}

// ── Goals page ──

#[tokio::test]
async fn test_priorities_page_loads() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let _phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    let resp = app.get("/priorities", Some(&cookie)).await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.body.contains("Priorities"), "Page should contain 'Priorities'");
}

// ── Trends page ──

#[tokio::test]
async fn test_trends_page_loads() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let _phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    let resp = app.get("/trends", Some(&cookie)).await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.body.contains("Trends"), "Page should contain 'Trends'");
}

// ── Check-in page ──

#[tokio::test]
async fn test_checkin_page_loads() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    // Create a week for the checkin
    let week = brag_frog::review::model::Week::find_or_create(
        &app.pool,
        phase_id,
        9,
        2025,
        "2025-02-24",
        "2025-03-02",
    )
    .await
    .unwrap();

    let resp = app
        .get(&format!("/checkin/{}", week.id), Some(&cookie))
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(
        resp.body.contains("Check-in") || resp.body.contains("check-in"),
        "Page should contain 'Check-in'"
    );
}

// ── Integrations page ──

#[tokio::test]
async fn test_integrations_page_loads() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let _phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    let resp = app.get("/integrations", Some(&cookie)).await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(
        resp.body.contains("Integrations") || resp.body.contains("Services"),
        "Page should contain 'Integrations' or 'Services'"
    );
}

// ── Check-in history page ──

#[tokio::test]
async fn test_checkins_list_page_loads() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let _phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    let resp = app.get("/checkins", Some(&cookie)).await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(
        resp.body.contains("Check-in History") || resp.body.contains("No check-ins"),
        "Page should contain check-in history content"
    );
}

// ── Meeting prep panel ──

#[tokio::test]
async fn test_meeting_prep_page_removed() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let _phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    // Standalone meeting-prep page no longer exists
    let resp = app.get("/meeting-prep", Some(&cookie)).await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND);
}

// ── Goals page with department goals ──

#[tokio::test]
async fn test_priorities_page_shows_department_goal() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    let user_crypto = app.crypto.for_user(user_id).unwrap();
    let _goal = common::create_test_department_goal(
        &app.pool,
        phase_id,
        user_id,
        "Edit Me Goal",
        &user_crypto,
    )
    .await;

    let resp = app.get("/priorities", Some(&cookie)).await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(
        resp.body.contains("Edit Me Goal"),
        "Priorities page should show department goal title"
    );
}

// ── Priority creation returns HTML fragment ──

#[tokio::test]
async fn test_priority_create_returns_html() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let _phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    let body = "title=Test+Priority&status=not_started";
    let resp = app
        .post_form("/priorities/create", body, Some(&cookie))
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.body.contains("Test Priority"));
}

// ── Settings save with new profile fields ──

#[tokio::test]
async fn test_settings_save_profile_fields() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let _phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    let body = "role=&display_name=Alice&team=Platform&manager_name=Bob&skip_level_name=&direct_reports=&timezone=US%2FPacific&week_start=monday";
    let resp = app.post_form("/settings", body, Some(&cookie)).await;

    let hx_redirect = resp.headers.get("hx-redirect").unwrap().to_str().unwrap();
    assert_eq!(hx_redirect, "/settings");

    // Verify saved
    let user = brag_frog::identity::model::User::find_by_id(&app.pool, user_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(user.display_name.as_deref(), Some("Alice"));
    assert_eq!(user.team.as_deref(), Some("Platform"));
    assert_eq!(user.manager_name.as_deref(), Some("Bob"));
    assert_eq!(user.timezone.as_deref(), Some("US/Pacific"));
}
