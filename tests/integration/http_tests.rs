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

#[tokio::test]
async fn test_review_page_shows_single_lattice_answer() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let phase_id: i64 = sqlx::query_scalar(
        "INSERT INTO brag_phases (user_id, name, start_date, end_date, is_active) VALUES (?, '2026 H1', '2026-01-01', '2026-06-30', 1) RETURNING id",
    )
    .bind(user_id)
    .fetch_one(&app.pool)
    .await
    .unwrap();
    let crypto = app.crypto.for_user(user_id).unwrap();
    let goal =
        common::create_test_department_goal(&app.pool, phase_id, user_id, "Review goals", &crypto)
            .await;
    common::create_test_priority(
        &app.pool,
        phase_id,
        user_id,
        "Improve review summaries",
        Some(goal.id),
        &crypto,
    )
    .await;
    let cookie = app.login(user_id).await;

    let resp = app
        .get(&format!("/review/{}", phase_id), Some(&cookie))
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(
        resp.body
            .contains("Describe 1-2 examples of your work so far this year")
    );
    assert!(resp.body.contains("The outcome or progress made"));
    assert!(resp.body.contains("Required: Write your response"));
    assert!(resp.body.contains("Department goals"));
    assert!(resp.body.contains("Review goals"));
    assert!(resp.body.contains("summary-goal-card"));
    assert!(!resp.body.contains("summary-goal-select"));
    assert!(resp.body.contains("summary-answer-textarea"));
    assert_eq!(resp.body.matches("summary-answer-textarea").count(), 1);
    assert!(resp.body.contains("Q2 2026"));
    assert!(resp.body.contains("Mid-year Review"));
    assert!(!resp.body.contains("Q1 2026"));
    assert!(!resp.body.contains("Check-in"));
    assert!(!resp.body.contains("quarterly-checkin"));
    assert!(!resp.body.contains("+ New Example"));
}

#[tokio::test]
async fn test_review_save_renders_markdown_and_jira_issue_links() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let phase_id = common::create_test_phase(&app.pool, user_id).await;
    let crypto = app.crypto.for_user(user_id).unwrap();
    common::create_test_department_goal(&app.pool, phase_id, user_id, "Review goals", &crypto)
        .await;
    let week =
        common::create_test_week(&app.pool, phase_id, 22, 2026, "2026-05-25", "2026-05-31").await;

    brag_frog::worklog::model::BragEntry::create_from_sync(
        &app.pool,
        week.id,
        "jira",
        "jira-DISCO-4260",
        Some("https://jira.example/browse/DISCO-4260"),
        "Adding retry mechanism",
        None,
        "jira_task",
        Some("Done"),
        None,
        "2026-05-27",
        None,
        None,
        None,
        None,
        None,
        None,
        &crypto,
    )
    .await
    .unwrap();

    let cookie = app.login(user_id).await;
    let content =
        "Resolved DISCO-4260 with **retry handling**. Follow-up DISCO-9999. Review period H1-2026.";
    let body = format!("content={}", urlencoding::encode(content));
    let resp = app
        .post_form(
            &format!("/review/{}/save/impact_examples", phase_id),
            &body,
            Some(&cookie),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.body.contains("summary-answer-preview"));
    assert!(resp.body.contains("<strong>retry handling</strong>"));
    assert!(
        resp.body
            .contains(r#"href="https://jira.example/browse/DISCO-4260""#)
    );
    assert!(resp.body.contains(">DISCO-4260</a>"));
    assert!(
        resp.body
            .contains(r#"href="https://your-org.atlassian.net/browse/DISCO-9999""#)
    );
    assert!(!resp.body.contains("browse/H1-2026"));
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
        brag_frog::worklog::model::BragEntry::list_for_phase(&app.pool, _phase_id, &user_crypto)
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
        brag_frog::worklog::model::BragEntry::list_for_phase(&app.pool, phase_id, &user_crypto)
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
        brag_frog::worklog::model::BragEntry::list_for_phase(&app.pool, _phase_id, &user_crypto)
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
    let updated = brag_frog::worklog::model::BragEntry::find_by_id(
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

    let updated = brag_frog::worklog::model::BragEntry::find_by_id(
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

    let updated = brag_frog::worklog::model::BragEntry::find_by_id(
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
    let gone = brag_frog::worklog::model::BragEntry::find_by_id(
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
    assert!(
        resp.body.contains("Priorities"),
        "Page should contain 'Priorities'"
    );
}

// ── Trends page ──

#[tokio::test]
async fn test_trends_page_shows_requested_metrics_and_department_goal_activity() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let phase_id = common::create_test_phase(&app.pool, user_id).await;
    let crypto = app.crypto.for_user(user_id).unwrap();
    let world_cup_goal = common::create_test_department_goal(
        &app.pool,
        phase_id,
        user_id,
        "Ship Soccer World Cup",
        &crypto,
    )
    .await;
    let mars_goal = common::create_test_department_goal(
        &app.pool,
        phase_id,
        user_id,
        "Integrate MARS with Merino",
        &crypto,
    )
    .await;
    let world_cup_priority = common::create_test_priority(
        &app.pool,
        phase_id,
        user_id,
        "Prepare merino for world cup deployment",
        Some(world_cup_goal.id),
        &crypto,
    )
    .await;
    let mars_priority = common::create_test_priority(
        &app.pool,
        phase_id,
        user_id,
        "Switch from Remote Settings to MARS",
        Some(mars_goal.id),
        &crypto,
    )
    .await;
    let unassigned_priority = common::create_test_priority(
        &app.pool,
        phase_id,
        user_id,
        "Lead Rust book club",
        None,
        &crypto,
    )
    .await;
    let week =
        common::create_test_week(&app.pool, phase_id, 2, 2025, "2025-01-06", "2025-01-12").await;

    common::create_test_entry(
        &app.pool,
        user_id,
        week.id,
        "World Cup merge",
        "pr_merged",
        "2025-01-06",
        Some(world_cup_priority.id),
        &crypto,
    )
    .await;
    common::create_test_entry(
        &app.pool,
        user_id,
        week.id,
        "World Cup review",
        "pr_reviewed",
        "2025-01-07",
        Some(world_cup_priority.id),
        &crypto,
    )
    .await;
    common::create_test_entry(
        &app.pool,
        user_id,
        week.id,
        "World Cup ticket",
        "jira_completed",
        "2025-01-08",
        Some(world_cup_priority.id),
        &crypto,
    )
    .await;
    common::create_test_entry(
        &app.pool,
        user_id,
        week.id,
        "World Cup docs",
        "design_doc",
        "2025-01-09",
        Some(world_cup_priority.id),
        &crypto,
    )
    .await;
    common::create_test_entry(
        &app.pool,
        user_id,
        week.id,
        "MARS docs",
        "drive_edited",
        "2025-01-10",
        Some(mars_priority.id),
        &crypto,
    )
    .await;
    common::create_test_entry(
        &app.pool,
        user_id,
        week.id,
        "Book club notes",
        "document",
        "2025-01-11",
        Some(unassigned_priority.id),
        &crypto,
    )
    .await;
    common::create_test_entry(
        &app.pool,
        user_id,
        week.id,
        "Planning meeting",
        "meeting",
        "2025-01-12",
        None,
        &crypto,
    )
    .await;
    brag_frog::worklog::model::BragEntry::create_from_sync(
        &app.pool,
        week.id,
        "jira",
        "jira-WC-1",
        Some("https://jira.example/browse/WC-1"),
        "Closed Jira story",
        None,
        "jira_story",
        Some("Done"),
        None,
        "2025-01-12",
        None,
        None,
        None,
        None,
        None,
        None,
        &crypto,
    )
    .await
    .unwrap();
    brag_frog::worklog::model::BragEntry::create_from_sync(
        &app.pool,
        week.id,
        "jira",
        "jira-WC-2",
        Some("https://jira.example/browse/WC-2"),
        "Closed Jira task",
        None,
        "jira_task",
        Some("Closed"),
        None,
        "2025-01-12",
        None,
        None,
        None,
        None,
        None,
        None,
        &crypto,
    )
    .await
    .unwrap();
    brag_frog::worklog::model::BragEntry::create_from_sync(
        &app.pool,
        week.id,
        "jira",
        "jira-WC-3",
        Some("https://jira.example/browse/WC-3"),
        "Active Jira epic",
        None,
        "jira_epic",
        Some("In Progress"),
        None,
        "2025-01-12",
        None,
        None,
        None,
        None,
        None,
        None,
        &crypto,
    )
    .await
    .unwrap();
    let cookie = app.login(user_id).await;

    let resp = app.get("/trends", Some(&cookie)).await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.body.contains("Trends"), "Page should contain 'Trends'");
    assert!(resp.body.contains("Total Activities"));
    assert!(resp.body.contains("PRs Merged"));
    assert!(resp.body.contains("PRs Reviewed"));
    assert!(resp.body.contains("Jira Tickets Closed"));
    assert!(resp.body.contains("Docs Worked On"));
    assert!(resp.body.contains("Meetings"));
    assert!(resp.body.contains(
        r#"<span class="report-stat-card-count">10</span><span class="report-stat-card-label">Total Activities</span>"#
    ));
    assert!(resp.body.contains(
        r#"<span class="report-stat-card-count">1</span><span class="report-stat-card-label">PRs Merged</span>"#
    ));
    assert!(resp.body.contains(
        r#"<span class="report-stat-card-count">1</span><span class="report-stat-card-label">PRs Reviewed</span>"#
    ));
    assert!(resp.body.contains(
        r#"<span class="report-stat-card-count">3</span><span class="report-stat-card-label">Jira Tickets Closed</span>"#
    ));
    assert!(resp.body.contains(
        r#"<span class="report-stat-card-count">3</span><span class="report-stat-card-label">Docs Worked On</span>"#
    ));
    assert!(resp.body.contains("Ship Soccer World Cup"));
    assert!(
        resp.body
            .contains("Prepare merino for world cup deployment")
    );
    assert!(resp.body.contains("Integrate MARS with Merino"));
    assert!(resp.body.contains("Switch from Remote Settings to MARS"));
    assert!(resp.body.contains("No Department Goal"));
    assert!(resp.body.contains("Unlinked Entries"));

    let world_cup_pos = resp.body.find("Ship Soccer World Cup").unwrap();
    let mars_pos = resp.body.find("Integrate MARS with Merino").unwrap();
    assert!(
        world_cup_pos < mars_pos,
        "Department goals should be sorted by most activity"
    );
}

// ── Removed legacy reflections routes ──

#[tokio::test]
async fn test_weekly_checkin_page_removed() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let _phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    let resp = app.get("/checkin/1", Some(&cookie)).await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND);
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

#[tokio::test]
async fn test_checkins_list_page_removed() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let _phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    let resp = app.get("/checkins", Some(&cookie)).await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND);
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

// ── Priority creation redirects via HX-Redirect ──

#[tokio::test]
async fn test_priority_create_returns_html() {
    let app = common::TestApp::new().await;
    let user_id = common::create_test_user(&app.pool).await;
    let _phase_id = common::create_test_phase(&app.pool, user_id).await;
    let cookie = app.login(user_id).await;

    let body = "title=Test+Priority&status=not_started";
    let resp = app.post_form("/priorities", body, Some(&cookie)).await;
    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.headers.get("HX-Redirect").unwrap(), "/priorities");
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
