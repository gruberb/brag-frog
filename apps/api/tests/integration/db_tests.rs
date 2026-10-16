use super::common;

use brag_frog::worklog::model::BragEntry;
use brag_frog::worklog::model::{CreateEntry, UpdateEntry};
use brag_frog::objectives::model::DepartmentGoal;
use brag_frog::cycle::model::{BragPhase, Week};
use brag_frog::review::model::Summary;

#[tokio::test]
async fn test_entry_crud() {
    let pool = common::test_pool().await;
    let crypto = common::test_crypto();
    let user_id = common::create_test_user(&pool).await;
    let user_crypto = common::test_user_crypto(&crypto, user_id);
    let phase_id = common::create_test_phase(&pool, user_id).await;

    let week = Week::find_or_create(&pool, phase_id, 1, 2025, "2025-01-06", "2025-01-12")
        .await
        .unwrap();

    let input = CreateEntry {
        week_id: week.id,
        priority_id: None,
        title: "Test Entry".to_string(),
        description: Some("A test entry".to_string()),
        entry_type: "other".to_string(),
        occurred_at: "2025-01-07".to_string(),
        teams: None,
        collaborators: None,
        source_url: None,
        reach: None,
        complexity: None,
        role: None,
    };

    // Create
    let entry = BragEntry::create(&pool, &input, user_id, &user_crypto)
        .await
        .unwrap();
    assert_eq!(entry.title, "Test Entry");
    assert_eq!(entry.description.as_deref(), Some("A test entry"));

    // Read
    let found = BragEntry::find_by_id(&pool, entry.id, user_id, &user_crypto)
        .await
        .unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().title, "Test Entry");

    // Update
    let update_input = brag_frog::worklog::model::UpdateEntry {
        priority_id: None,
        title: "Updated Entry".to_string(),
        description: Some("Updated desc".to_string()),
        entry_type: "meeting".to_string(),
        occurred_at: "2025-01-08".to_string(),
        teams: None,
        collaborators: None,
        source_url: None,
        reach: None,
        complexity: None,
        role: None,
    };
    let updated = BragEntry::update(&pool, entry.id, user_id, &update_input, None, &user_crypto)
        .await
        .unwrap();
    assert_eq!(updated.title, "Updated Entry");
    assert_eq!(updated.entry_type, "meeting");

    // Hard delete
    BragEntry::delete(&pool, entry.id, user_id).await.unwrap();
    let gone = BragEntry::find_by_id(&pool, entry.id, user_id, &user_crypto)
        .await
        .unwrap();
    assert!(gone.is_none());
}

#[tokio::test]
async fn test_soft_delete() {
    let pool = common::test_pool().await;
    let crypto = common::test_crypto();
    let user_id = common::create_test_user(&pool).await;
    let user_crypto = common::test_user_crypto(&crypto, user_id);
    let phase_id = common::create_test_phase(&pool, user_id).await;

    let week = Week::find_or_create(&pool, phase_id, 2, 2025, "2025-01-13", "2025-01-19")
        .await
        .unwrap();

    let input = CreateEntry {
        week_id: week.id,
        priority_id: None,
        title: "Soft Delete Me".to_string(),
        description: None,
        entry_type: "other".to_string(),
        occurred_at: "2025-01-14".to_string(),
        teams: None,
        collaborators: None,
        source_url: None,
        reach: None,
        complexity: None,
        role: None,
    };

    let entry = BragEntry::create(&pool, &input, user_id, &user_crypto)
        .await
        .unwrap();

    // Soft-delete
    BragEntry::soft_delete(&pool, entry.id, user_id)
        .await
        .unwrap();

    // Entry still exists in DB but excluded from list_for_phase
    let all = BragEntry::list_for_phase(&pool, phase_id, &user_crypto)
        .await
        .unwrap();
    assert!(all.iter().all(|e| e.id != entry.id));

    // But can still be found by id (deleted_at is set)
    let found = BragEntry::find_by_id(&pool, entry.id, user_id, &user_crypto)
        .await
        .unwrap();
    assert!(found.is_some());
    assert!(found.unwrap().deleted_at.is_some());
}

#[tokio::test]
async fn test_phase_cascade_delete() {
    let pool = common::test_pool().await;
    let crypto = common::test_crypto();
    let user_id = common::create_test_user(&pool).await;
    let user_crypto = common::test_user_crypto(&crypto, user_id);
    let phase_id = common::create_test_phase(&pool, user_id).await;

    // Create a week, entry, department goal, and summary
    let week = Week::find_or_create(&pool, phase_id, 3, 2025, "2025-01-20", "2025-01-26")
        .await
        .unwrap();

    let input = CreateEntry {
        week_id: week.id,
        priority_id: None,
        title: "Cascade Entry".to_string(),
        description: None,
        entry_type: "other".to_string(),
        occurred_at: "2025-01-21".to_string(),
        teams: None,
        collaborators: None,
        source_url: None,
        reach: None,
        complexity: None,
        role: None,
    };
    BragEntry::create(&pool, &input, user_id, &user_crypto)
        .await
        .unwrap();

    let _goal = common::create_test_department_goal(
        &pool,
        phase_id,
        user_id,
        "Cascade Goal",
        &user_crypto,
    )
    .await;

    Summary::upsert(
        &pool,
        phase_id,
        "goal_outcomes",
        "Test summary",
        None,
        None,
        &user_crypto,
    )
    .await
    .unwrap();

    // Delete phase
    BragPhase::delete(&pool, phase_id, user_id).await.unwrap();

    // Verify cascade
    let weeks = Week::list_for_phase(&pool, phase_id).await.unwrap();
    assert!(weeks.is_empty());

    let goals = DepartmentGoal::list_for_phase(&pool, phase_id, &user_crypto)
        .await
        .unwrap();
    assert!(goals.is_empty());

    let summaries = Summary::list_for_phase(&pool, phase_id, &user_crypto)
        .await
        .unwrap();
    assert!(summaries.is_empty());
}

#[tokio::test]
async fn test_week_find_or_create() {
    let pool = common::test_pool().await;
    let user_id = common::create_test_user(&pool).await;
    let phase_id = common::create_test_phase(&pool, user_id).await;

    let w1 = Week::find_or_create(&pool, phase_id, 5, 2025, "2025-01-27", "2025-02-02")
        .await
        .unwrap();
    let w2 = Week::find_or_create(&pool, phase_id, 5, 2025, "2025-01-27", "2025-02-02")
        .await
        .unwrap();
    assert_eq!(w1.id, w2.id, "find_or_create should be idempotent");

    let w3 = Week::find_or_create(&pool, phase_id, 6, 2025, "2025-02-03", "2025-02-09")
        .await
        .unwrap();
    assert_ne!(w1.id, w3.id, "different week should get different id");
}

#[tokio::test]
async fn test_priority_department_goal_hierarchy() {
    let pool = common::test_pool().await;
    let crypto = common::test_crypto();
    let user_id = common::create_test_user(&pool).await;
    let user_crypto = common::test_user_crypto(&crypto, user_id);
    let phase_id = common::create_test_phase(&pool, user_id).await;

    let week = Week::find_or_create(&pool, phase_id, 8, 2025, "2025-02-17", "2025-02-23")
        .await
        .unwrap();

    // Create a department goal
    let goal = common::create_test_department_goal(
        &pool,
        phase_id,
        user_id,
        "Ship OHTTP",
        &user_crypto,
    )
    .await;

    // Create a priority under the department goal
    let priority = common::create_test_priority(
        &pool,
        phase_id,
        user_id,
        "Viaduct component",
        Some(goal.id),
        &user_crypto,
    )
    .await;
    assert_eq!(priority.department_goal_id, Some(goal.id));

    // Create an entry linked to the priority
    let input = CreateEntry {
        week_id: week.id,
        priority_id: Some(priority.id),
        title: "PR for viaduct".to_string(),
        description: None,
        entry_type: "pr_authored".to_string(),
        occurred_at: "2025-02-18".to_string(),
        teams: None,
        collaborators: None,
        source_url: None,
        reach: None,
        complexity: None,
        role: None,
    };
    let entry = BragEntry::create(&pool, &input, user_id, &user_crypto)
        .await
        .unwrap();
    assert_eq!(entry.priority_id, Some(priority.id));

    // Verify filtering by department_goal_id finds the entry (through priority)
    let filtered = BragEntry::list_for_phase_filtered(
        &pool,
        phase_id,
        None,
        Some(goal.id),
        &[],
        None,
        None,
        &[],
        &user_crypto,
    )
    .await
    .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, entry.id);
}

#[tokio::test]
async fn test_filter_by_department_goal_id() {
    let pool = common::test_pool().await;
    let crypto = common::test_crypto();
    let user_id = common::create_test_user(&pool).await;
    let user_crypto = common::test_user_crypto(&crypto, user_id);
    let phase_id = common::create_test_phase(&pool, user_id).await;
    let week =
        common::create_test_week(&pool, phase_id, 10, 2025, "2025-03-03", "2025-03-09").await;

    let goal_a =
        common::create_test_department_goal(&pool, phase_id, user_id, "Goal A", &user_crypto)
            .await;
    let goal_b =
        common::create_test_department_goal(&pool, phase_id, user_id, "Goal B", &user_crypto)
            .await;
    let pri_a = common::create_test_priority(
        &pool,
        phase_id,
        user_id,
        "Priority under A",
        Some(goal_a.id),
        &user_crypto,
    )
    .await;
    let pri_b = common::create_test_priority(
        &pool,
        phase_id,
        user_id,
        "Priority under B",
        Some(goal_b.id),
        &user_crypto,
    )
    .await;

    let entry_a = common::create_test_entry(
        &pool,
        user_id,
        week.id,
        "Entry A",
        "other",
        "2025-03-04",
        Some(pri_a.id),
        &user_crypto,
    )
    .await;
    let _entry_b = common::create_test_entry(
        &pool,
        user_id,
        week.id,
        "Entry B",
        "other",
        "2025-03-05",
        Some(pri_b.id),
        &user_crypto,
    )
    .await;

    // Filter by goal A -> only entry A
    let filtered = BragEntry::list_for_phase_filtered(
        &pool,
        phase_id,
        None,
        Some(goal_a.id),
        &[],
        None,
        None,
        &[],
        &user_crypto,
    )
    .await
    .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, entry_a.id);

    // Filter by goal B -> only entry B
    let filtered = BragEntry::list_for_phase_filtered(
        &pool,
        phase_id,
        None,
        Some(goal_b.id),
        &[],
        None,
        None,
        &[],
        &user_crypto,
    )
    .await
    .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].title, "Entry B");
}

#[tokio::test]
async fn test_filter_by_priority_id() {
    let pool = common::test_pool().await;
    let crypto = common::test_crypto();
    let user_id = common::create_test_user(&pool).await;
    let user_crypto = common::test_user_crypto(&crypto, user_id);
    let phase_id = common::create_test_phase(&pool, user_id).await;
    let week =
        common::create_test_week(&pool, phase_id, 11, 2025, "2025-03-10", "2025-03-16").await;

    let pri1 = common::create_test_priority(
        &pool,
        phase_id,
        user_id,
        "Priority 1",
        None,
        &user_crypto,
    )
    .await;
    let pri2 = common::create_test_priority(
        &pool,
        phase_id,
        user_id,
        "Priority 2",
        None,
        &user_crypto,
    )
    .await;

    let e1 = common::create_test_entry(
        &pool,
        user_id,
        week.id,
        "E1",
        "other",
        "2025-03-11",
        Some(pri1.id),
        &user_crypto,
    )
    .await;
    let _e2 = common::create_test_entry(
        &pool,
        user_id,
        week.id,
        "E2",
        "meeting",
        "2025-03-12",
        Some(pri2.id),
        &user_crypto,
    )
    .await;

    let filtered = BragEntry::list_for_phase_filtered(
        &pool,
        phase_id,
        Some(pri1.id),
        None,
        &[],
        None,
        None,
        &[],
        &user_crypto,
    )
    .await
    .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, e1.id);
}

#[tokio::test]
async fn test_filter_by_both_goal_and_priority() {
    let pool = common::test_pool().await;
    let crypto = common::test_crypto();
    let user_id = common::create_test_user(&pool).await;
    let user_crypto = common::test_user_crypto(&crypto, user_id);
    let phase_id = common::create_test_phase(&pool, user_id).await;
    let week =
        common::create_test_week(&pool, phase_id, 12, 2025, "2025-03-17", "2025-03-23").await;

    let goal =
        common::create_test_department_goal(&pool, phase_id, user_id, "Goal X", &user_crypto)
            .await;
    let pri_in_goal = common::create_test_priority(
        &pool,
        phase_id,
        user_id,
        "Priority in goal",
        Some(goal.id),
        &user_crypto,
    )
    .await;
    let pri_other = common::create_test_priority(
        &pool,
        phase_id,
        user_id,
        "Priority other",
        None,
        &user_crypto,
    )
    .await;

    let _e1 = common::create_test_entry(
        &pool,
        user_id,
        week.id,
        "In goal priority",
        "other",
        "2025-03-18",
        Some(pri_in_goal.id),
        &user_crypto,
    )
    .await;
    let _e2 = common::create_test_entry(
        &pool,
        user_id,
        week.id,
        "Other priority",
        "other",
        "2025-03-19",
        Some(pri_other.id),
        &user_crypto,
    )
    .await;

    // Filter by both goal and the priority that doesn't belong to it -> 0 results
    let filtered = BragEntry::list_for_phase_filtered(
        &pool,
        phase_id,
        Some(pri_other.id),
        Some(goal.id),
        &[],
        None,
        None,
        &[],
        &user_crypto,
    )
    .await
    .unwrap();
    assert_eq!(
        filtered.len(),
        0,
        "Priority doesn't belong to goal, should return 0"
    );

    // Filter by both goal and matching priority -> 1 result
    let filtered = BragEntry::list_for_phase_filtered(
        &pool,
        phase_id,
        Some(pri_in_goal.id),
        Some(goal.id),
        &[],
        None,
        None,
        &[],
        &user_crypto,
    )
    .await
    .unwrap();
    assert_eq!(filtered.len(), 1);
}

#[tokio::test]
async fn test_filter_by_entry_type() {
    let pool = common::test_pool().await;
    let crypto = common::test_crypto();
    let user_id = common::create_test_user(&pool).await;
    let user_crypto = common::test_user_crypto(&crypto, user_id);
    let phase_id = common::create_test_phase(&pool, user_id).await;
    let week =
        common::create_test_week(&pool, phase_id, 13, 2025, "2025-03-24", "2025-03-30").await;

    let _e1 = common::create_test_entry(
        &pool,
        user_id,
        week.id,
        "Meeting 1",
        "meeting",
        "2025-03-25",
        None,
        &user_crypto,
    )
    .await;
    let _e2 = common::create_test_entry(
        &pool,
        user_id,
        week.id,
        "PR 1",
        "pr_authored",
        "2025-03-26",
        None,
        &user_crypto,
    )
    .await;
    let _e3 = common::create_test_entry(
        &pool,
        user_id,
        week.id,
        "Meeting 2",
        "meeting",
        "2025-03-27",
        None,
        &user_crypto,
    )
    .await;

    let types = vec!["meeting".to_string()];
    let filtered = BragEntry::list_for_phase_filtered(
        &pool,
        phase_id,
        None,
        None,
        &types,
        None,
        None,
        &[],
        &user_crypto,
    )
    .await
    .unwrap();
    assert_eq!(filtered.len(), 2);
    assert!(filtered.iter().all(|e| e.entry_type == "meeting"));
}

#[tokio::test]
async fn test_filter_by_source() {
    let pool = common::test_pool().await;
    let crypto = common::test_crypto();
    let user_id = common::create_test_user(&pool).await;
    let user_crypto = common::test_user_crypto(&crypto, user_id);
    let phase_id = common::create_test_phase(&pool, user_id).await;
    let week =
        common::create_test_week(&pool, phase_id, 14, 2025, "2025-03-31", "2025-04-06").await;

    // Manual entry (created via BragEntry::create which sets source="manual")
    let _e_manual = common::create_test_entry(
        &pool,
        user_id,
        week.id,
        "Manual Entry",
        "other",
        "2025-04-01",
        None,
        &user_crypto,
    )
    .await;

    // Synced entry
    let _e_gh = BragEntry::create_from_sync(
        &pool,
        week.id,
        "github",
        "gh-123",
        Some("https://github.com/test/pr/1"),
        "GitHub PR",
        None,
        "pr_authored",
        None,
        Some("test-repo"),
        "2025-04-02",
        None,
        None,
        None,
        None,
        None,
        None,
        &user_crypto,
    )
    .await
    .unwrap();

    let sources = vec!["github".to_string()];
    let filtered = BragEntry::list_for_phase_filtered(
        &pool,
        phase_id,
        None,
        None,
        &[],
        None,
        None,
        &sources,
        &user_crypto,
    )
    .await
    .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].title, "GitHub PR");

    let sources = vec!["manual".to_string()];
    let filtered = BragEntry::list_for_phase_filtered(
        &pool,
        phase_id,
        None,
        None,
        &[],
        None,
        None,
        &sources,
        &user_crypto,
    )
    .await
    .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].title, "Manual Entry");
}

#[tokio::test]
async fn test_filter_by_multiple_types() {
    let pool = common::test_pool().await;
    let crypto = common::test_crypto();
    let user_id = common::create_test_user(&pool).await;
    let user_crypto = common::test_user_crypto(&crypto, user_id);
    let phase_id = common::create_test_phase(&pool, user_id).await;
    let week =
        common::create_test_week(&pool, phase_id, 15, 2025, "2025-04-07", "2025-04-13").await;

    let _e1 = common::create_test_entry(
        &pool,
        user_id,
        week.id,
        "Meeting",
        "meeting",
        "2025-04-08",
        None,
        &user_crypto,
    )
    .await;
    let _e2 = common::create_test_entry(
        &pool,
        user_id,
        week.id,
        "Pairing",
        "pairing",
        "2025-04-09",
        None,
        &user_crypto,
    )
    .await;
    let _e3 = common::create_test_entry(
        &pool,
        user_id,
        week.id,
        "Other",
        "other",
        "2025-04-10",
        None,
        &user_crypto,
    )
    .await;

    let types = vec!["meeting".to_string(), "pairing".to_string()];
    let filtered = BragEntry::list_for_phase_filtered(
        &pool,
        phase_id,
        None,
        None,
        &types,
        None,
        None,
        &[],
        &user_crypto,
    )
    .await
    .unwrap();
    assert_eq!(filtered.len(), 2);
    let found_types: Vec<&str> = filtered.iter().map(|e| e.entry_type.as_str()).collect();
    assert!(found_types.contains(&"meeting"));
    assert!(found_types.contains(&"pairing"));
}

#[tokio::test]
async fn test_filter_returns_empty_for_no_match() {
    let pool = common::test_pool().await;
    let crypto = common::test_crypto();
    let user_id = common::create_test_user(&pool).await;
    let user_crypto = common::test_user_crypto(&crypto, user_id);
    let phase_id = common::create_test_phase(&pool, user_id).await;
    let week =
        common::create_test_week(&pool, phase_id, 16, 2025, "2025-04-14", "2025-04-20").await;

    let _e = common::create_test_entry(
        &pool,
        user_id,
        week.id,
        "Entry",
        "other",
        "2025-04-15",
        None,
        &user_crypto,
    )
    .await;

    // Non-existent department goal
    let filtered = BragEntry::list_for_phase_filtered(
        &pool,
        phase_id,
        None,
        Some(99999),
        &[],
        None,
        None,
        &[],
        &user_crypto,
    )
    .await
    .unwrap();
    assert!(filtered.is_empty());

    // Non-existent priority
    let filtered = BragEntry::list_for_phase_filtered(
        &pool,
        phase_id,
        Some(99999),
        None,
        &[],
        None,
        None,
        &[],
        &user_crypto,
    )
    .await
    .unwrap();
    assert!(filtered.is_empty());

    // Non-matching entry type
    let types = vec!["pr_authored".to_string()];
    let filtered = BragEntry::list_for_phase_filtered(
        &pool,
        phase_id,
        None,
        None,
        &types,
        None,
        None,
        &[],
        &user_crypto,
    )
    .await
    .unwrap();
    assert!(filtered.is_empty());
}

#[tokio::test]
async fn test_update_entry_date_changes_week() {
    let pool = common::test_pool().await;
    let crypto = common::test_crypto();
    let user_id = common::create_test_user(&pool).await;
    let user_crypto = common::test_user_crypto(&crypto, user_id);
    let phase_id = common::create_test_phase(&pool, user_id).await;

    let week1 =
        common::create_test_week(&pool, phase_id, 1, 2025, "2025-01-06", "2025-01-12").await;
    let week2 =
        common::create_test_week(&pool, phase_id, 2, 2025, "2025-01-13", "2025-01-19").await;

    let entry = common::create_test_entry(
        &pool,
        user_id,
        week1.id,
        "Move me",
        "other",
        "2025-01-07",
        None,
        &user_crypto,
    )
    .await;
    assert_eq!(entry.week_id, week1.id);

    // Update date to fall in week 2
    let update = UpdateEntry {
        priority_id: None,
        title: "Move me".to_string(),
        description: None,
        entry_type: "other".to_string(),
        occurred_at: "2025-01-14".to_string(),
        teams: None,
        collaborators: None,
        source_url: None,
        reach: None,
        complexity: None,
        role: None,
    };
    let updated = BragEntry::update(
        &pool,
        entry.id,
        user_id,
        &update,
        Some(week2.id),
        &user_crypto,
    )
    .await
    .unwrap();
    assert_eq!(updated.week_id, week2.id);
    assert_eq!(updated.occurred_at, "2025-01-14");
}

#[tokio::test]
async fn test_entry_sync_upsert_preserves_priority() {
    let pool = common::test_pool().await;
    let crypto = common::test_crypto();
    let user_id = common::create_test_user(&pool).await;
    let user_crypto = common::test_user_crypto(&crypto, user_id);
    let phase_id = common::create_test_phase(&pool, user_id).await;
    let week =
        common::create_test_week(&pool, phase_id, 17, 2025, "2025-04-21", "2025-04-27").await;

    let priority = common::create_test_priority(
        &pool,
        phase_id,
        user_id,
        "My Priority",
        None,
        &user_crypto,
    )
    .await;

    // Create entry from sync (no priority_id)
    let synced = BragEntry::create_from_sync(
        &pool,
        week.id,
        "github",
        "gh-upsert-1",
        None,
        "Original PR",
        None,
        "pr_authored",
        None,
        None,
        "2025-04-22",
        None,
        None,
        None,
        None,
        None,
        None,
        &user_crypto,
    )
    .await
    .unwrap();
    assert_eq!(synced.priority_id, None);

    // Manually assign a priority
    sqlx::query("UPDATE brag_entries SET priority_id = ? WHERE id = ?")
        .bind(priority.id)
        .bind(synced.id)
        .execute(&pool)
        .await
        .unwrap();

    // Re-sync (upsert) with updated title
    let re_synced = BragEntry::create_from_sync(
        &pool,
        week.id,
        "github",
        "gh-upsert-1",
        None,
        "Updated PR Title",
        None,
        "pr_authored",
        None,
        None,
        "2025-04-22",
        None,
        None,
        None,
        None,
        None,
        None,
        &user_crypto,
    )
    .await
    .unwrap();

    // Same entry (upserted), title updated
    assert_eq!(re_synced.id, synced.id);
    assert_eq!(re_synced.title, "Updated PR Title");

    // priority_id should be preserved (upsert doesn't touch it)
    let found = BragEntry::find_by_id(&pool, synced.id, user_id, &user_crypto)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        found.priority_id,
        Some(priority.id),
        "priority_id should be preserved after sync upsert"
    );
}

#[tokio::test]
async fn test_filter_excludes_soft_deleted() {
    let pool = common::test_pool().await;
    let crypto = common::test_crypto();
    let user_id = common::create_test_user(&pool).await;
    let user_crypto = common::test_user_crypto(&crypto, user_id);
    let phase_id = common::create_test_phase(&pool, user_id).await;
    let week =
        common::create_test_week(&pool, phase_id, 18, 2025, "2025-04-28", "2025-05-04").await;

    let e1 = common::create_test_entry(
        &pool,
        user_id,
        week.id,
        "Keep",
        "other",
        "2025-04-29",
        None,
        &user_crypto,
    )
    .await;
    let e2 = common::create_test_entry(
        &pool,
        user_id,
        week.id,
        "Delete me",
        "other",
        "2025-04-30",
        None,
        &user_crypto,
    )
    .await;

    BragEntry::soft_delete(&pool, e2.id, user_id).await.unwrap();

    let filtered = BragEntry::list_for_phase_filtered(
        &pool,
        phase_id,
        None,
        None,
        &[],
        None,
        None,
        &[],
        &user_crypto,
    )
    .await
    .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, e1.id);
}

#[tokio::test]
async fn test_filter_date_range() {
    let pool = common::test_pool().await;
    let crypto = common::test_crypto();
    let user_id = common::create_test_user(&pool).await;
    let user_crypto = common::test_user_crypto(&crypto, user_id);
    let phase_id = common::create_test_phase(&pool, user_id).await;
    let week =
        common::create_test_week(&pool, phase_id, 19, 2025, "2025-05-05", "2025-05-11").await;

    let _e1 = common::create_test_entry(
        &pool,
        user_id,
        week.id,
        "Early",
        "other",
        "2025-05-05",
        None,
        &user_crypto,
    )
    .await;
    let e2 = common::create_test_entry(
        &pool,
        user_id,
        week.id,
        "Middle",
        "other",
        "2025-05-07",
        None,
        &user_crypto,
    )
    .await;
    let _e3 = common::create_test_entry(
        &pool,
        user_id,
        week.id,
        "Late",
        "other",
        "2025-05-11",
        None,
        &user_crypto,
    )
    .await;

    // Only entries in the middle of the week
    let filtered = BragEntry::list_for_phase_filtered(
        &pool,
        phase_id,
        None,
        None,
        &[],
        Some("2025-05-06"),
        Some("2025-05-09"),
        &[],
        &user_crypto,
    )
    .await
    .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, e2.id);

    // All entries in the full range
    let filtered = BragEntry::list_for_phase_filtered(
        &pool,
        phase_id,
        None,
        None,
        &[],
        Some("2025-05-05"),
        Some("2025-05-11"),
        &[],
        &user_crypto,
    )
    .await
    .unwrap();
    assert_eq!(filtered.len(), 3);
}
