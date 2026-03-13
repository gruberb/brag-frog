use rand::Rng;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;

use crate::kernel::crypto::Crypto;

/// Initializes the SQLite connection pool with foreign keys enabled,
/// create-if-missing, and a max of 5 connections.
///
/// Journal mode defaults to WAL for local disks. Set `SQLITE_JOURNAL_MODE=delete`
/// for FUSE/NFS-mounted volumes where WAL's shared-memory files cause stale handle errors.
pub async fn setup_pool(database_path: &str) -> SqlitePool {
    let journal_mode = match std::env::var("SQLITE_JOURNAL_MODE")
        .unwrap_or_default()
        .to_lowercase()
        .as_str()
    {
        "delete" => sqlx::sqlite::SqliteJournalMode::Delete,
        "truncate" => sqlx::sqlite::SqliteJournalMode::Truncate,
        _ => sqlx::sqlite::SqliteJournalMode::Wal,
    };

    let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", database_path))
        .expect("Invalid database path")
        .create_if_missing(true)
        .journal_mode(journal_mode)
        .foreign_keys(true);

    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .expect("Failed to connect to database")
}

/// Runs the initial migration if the database is empty, then applies any
/// incremental migrations (002+). Tracks applied migrations in `_migrations`.
/// Safe to call on every startup.
pub async fn run_migrations(pool: &SqlitePool) {
    let migration_sql = include_str!("../../migrations/001_initial.sql");

    // Check if migrations have been run by checking if the users table exists
    let table_exists: bool = sqlx::query_scalar(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='users'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if !table_exists {
        tracing::info!("Running initial migration...");
        for statement in migration_sql.split(';') {
            let stmt = statement.trim();
            if !stmt.is_empty() {
                sqlx::query(stmt).execute(pool).await.unwrap_or_else(|e| {
                    panic!("Migration failed on statement: {}\nError: {}", stmt, e)
                });
            }
        }
        tracing::info!("Migration complete.");
    }

    run_incremental_migrations(pool).await;
}

/// Incremental migration files included at compile time.
/// Add new entries here when creating migrations 003+.
const INCREMENTAL_MIGRATIONS: &[(&str, &str)] = &[
    (
        "002_priorities_redesign",
        include_str!("../../migrations/002_priorities_redesign.sql"),
    ),
    (
        "003_drop_legacy_tables",
        include_str!("../../migrations/003_drop_legacy_tables.sql"),
    ),
    (
        "004_remove_kr_from_priorities",
        include_str!("../../migrations/004_remove_kr_from_priorities.sql"),
    ),
    (
        "005_remove_category_from_department_goals",
        include_str!("../../migrations/005_remove_category_from_department_goals.sql"),
    ),
    (
        "006_add_meeting_goal",
        include_str!("../../migrations/006_add_meeting_goal.sql"),
    ),
    (
        "007_remove_description_from_priorities",
        include_str!("../../migrations/007_remove_description_from_priorities.sql"),
    ),
    (
        "008_people_aliases",
        include_str!("../../migrations/008_people_aliases.sql"),
    ),
    (
        "009_add_team_to_people_aliases",
        include_str!("../../migrations/009_add_team_to_people_aliases.sql"),
    ),
    (
        "010_priority_level_and_measurement",
        include_str!("../../migrations/010_priority_level_and_measurement.sql"),
    ),
    (
        "011_external_id_for_imports",
        include_str!("../../migrations/011_external_id_for_imports.sql"),
    ),
    (
        "012_fix_external_id_indexes",
        include_str!("../../migrations/012_fix_external_id_indexes.sql"),
    ),
    (
        "013_priority_tracking_and_tier",
        include_str!("../../migrations/013_priority_tracking_and_tier.sql"),
    ),
    (
        "014_priority_updates",
        include_str!("../../migrations/014_priority_updates.sql"),
    ),
];

/// Applies migrations beyond 001 that haven't been run yet.
/// Uses a `_migrations` tracking table to record which have been applied.
async fn run_incremental_migrations(pool: &SqlitePool) {
    // Create tracking table if it doesn't exist
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _migrations (
            id TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await
    .expect("Failed to create _migrations table");

    for &(id, sql) in INCREMENTAL_MIGRATIONS {
        let already_applied: bool =
            sqlx::query_scalar("SELECT COUNT(*) > 0 FROM _migrations WHERE id = ?")
                .bind(id)
                .fetch_one(pool)
                .await
                .unwrap_or(false);

        if already_applied {
            continue;
        }

        tracing::info!("Running migration {}...", id);
        for statement in sql.split(';') {
            let stmt = statement.trim();
            if !stmt.is_empty() {
                sqlx::query(stmt).execute(pool).await.unwrap_or_else(|e| {
                    panic!(
                        "Migration {} failed on statement: {}\nError: {}",
                        id, stmt, e
                    )
                });
            }
        }

        sqlx::query("INSERT INTO _migrations (id) VALUES (?)")
            .bind(id)
            .execute(pool)
            .await
            .unwrap_or_else(|e| panic!("Failed to record migration {}: {}", id, e));

        tracing::info!("Migration {} complete.", id);
    }
}

// Curated palette of distinct, accessible colors for priority badges.
const PRIORITY_COLORS: &[&str] = &[
    "#FF453F", "#E0439D", "#D94F04", "#0B8043", "#1A73E8", "#F4511E", "#8E24AA", "#00897B",
    "#C2185B", "#6D4C41", "#5C6BC0", "#00ACC1", "#7CB342", "#F9A825", "#546E7A",
];

fn random_priority_color() -> String {
    let mut rng = rand::rng();
    let idx = rng.random_range(0..PRIORITY_COLORS.len());
    PRIORITY_COLORS[idx].to_string()
}

/// Post-SQL data migrations that require application-level logic (encryption).
/// Called after crypto is initialized. Idempotent — tracks completion in `_migrations`.
pub async fn run_post_migrations(pool: &SqlitePool, crypto: &Crypto) {
    migrate_key_results_to_priorities(pool, crypto).await;
}

/// Migrates key_results rows into priorities. KR `name` is plaintext TEXT but
/// priorities.title must be an encrypted BLOB. Reads each KR, encrypts the name
/// with the owning user's crypto, and inserts into priorities.
async fn migrate_key_results_to_priorities(pool: &SqlitePool, crypto: &Crypto) {
    const MIGRATION_ID: &str = "002_kr_to_priorities_encryption";

    let already_applied: bool =
        sqlx::query_scalar("SELECT COUNT(*) > 0 FROM _migrations WHERE id = ?")
            .bind(MIGRATION_ID)
            .fetch_one(pool)
            .await
            .unwrap_or(false);

    if already_applied {
        return;
    }

    // Check if key_results table still exists (may have been dropped already)
    let kr_table_exists: bool = sqlx::query_scalar(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='key_results'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if !kr_table_exists {
        // Table already dropped — mark as done and return
        sqlx::query("INSERT INTO _migrations (id) VALUES (?)")
            .bind(MIGRATION_ID)
            .execute(pool)
            .await
            .unwrap_or_else(|e| panic!("Failed to record migration {}: {}", MIGRATION_ID, e));
        return;
    }

    #[derive(sqlx::FromRow)]
    struct KrRow {
        id: i64,
        user_id: i64,
        name: String,
        color: Option<String>,
        status: String,
        goal_id: Option<i64>,
        progress: i64,
        kr_type: String,
        direction: Option<String>,
        unit: Option<String>,
        baseline: Option<f64>,
        target: Option<f64>,
        current_value: Option<f64>,
        target_date: Option<String>,
        score: Option<f64>,
        created_at: String,
    }

    let krs = sqlx::query_as::<_, KrRow>("SELECT * FROM key_results")
        .fetch_all(pool)
        .await
        .unwrap_or_else(|e| panic!("Failed to read key_results: {}", e));

    if krs.is_empty() {
        tracing::info!("No key results to migrate.");
        sqlx::query("INSERT INTO _migrations (id) VALUES (?)")
            .bind(MIGRATION_ID)
            .execute(pool)
            .await
            .unwrap_or_else(|e| panic!("Failed to record migration {}: {}", MIGRATION_ID, e));
        return;
    }

    tracing::info!(
        "Migrating {} key results to priorities (encrypting names)...",
        krs.len()
    );

    let mut migrated = 0u64;
    let mut skipped = 0u64;

    for kr in &krs {
        let user_crypto = match crypto.for_user(kr.user_id) {
            Ok(uc) => uc,
            Err(e) => {
                tracing::warn!(
                    "Skipping KR {} (user {}): crypto init failed: {}",
                    kr.id,
                    kr.user_id,
                    e
                );
                skipped += 1;
                continue;
            }
        };

        let enc_title = match user_crypto.encrypt(&kr.name) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(
                    "Skipping KR {} (user {}): encryption failed: {}",
                    kr.id,
                    kr.user_id,
                    e
                );
                skipped += 1;
                continue;
            }
        };

        // Infer phase_id from goal_id -> goals.phase_id, or fall back to user's active phase
        let phase_id: Option<i64> = if let Some(goal_id) = kr.goal_id {
            sqlx::query_scalar("SELECT phase_id FROM goals WHERE id = ?")
                .bind(goal_id)
                .fetch_optional(pool)
                .await
                .unwrap_or(None)
        } else {
            None
        };

        let phase_id = match phase_id {
            Some(pid) => pid,
            None => {
                // Fall back to user's active phase
                let active: Option<i64> = sqlx::query_scalar(
                    "SELECT id FROM brag_phases WHERE user_id = ? AND is_active = 1",
                )
                .bind(kr.user_id)
                .fetch_optional(pool)
                .await
                .unwrap_or(None);

                match active {
                    Some(pid) => pid,
                    None => {
                        // Fall back to user's most recent phase
                        let recent: Option<i64> = sqlx::query_scalar(
                            "SELECT id FROM brag_phases WHERE user_id = ? ORDER BY start_date DESC LIMIT 1",
                        )
                        .bind(kr.user_id)
                        .fetch_optional(pool)
                        .await
                        .unwrap_or(None);

                        match recent {
                            Some(pid) => pid,
                            None => {
                                tracing::warn!(
                                    "Skipping KR {} (user {}): no phase found",
                                    kr.id,
                                    kr.user_id
                                );
                                skipped += 1;
                                continue;
                            }
                        }
                    }
                }
            }
        };

        // Map goal_id to department_goal_id (IDs are preserved in data migration)
        let dept_goal_id = kr.goal_id;

        let status = match kr.status.as_str() {
            "not_started" => "not_started",
            "in_progress" => "active",
            "on_hold" => "on_hold",
            "completed" => "completed",
            other => other,
        };

        let color = kr
            .color
            .clone()
            .unwrap_or_else(random_priority_color);

        let result = sqlx::query(
            r#"
            INSERT INTO priorities (
                phase_id, user_id, title, status, color, sort_order,
                department_goal_id, kr_type, direction, unit, baseline,
                target, current_value, target_date, score, progress,
                encryption_version, created_at
            )
            VALUES (?, ?, ?, ?, ?, 0, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?)
            "#,
        )
        .bind(phase_id)
        .bind(kr.user_id)
        .bind(&enc_title)
        .bind(status)
        .bind(&color)
        .bind(dept_goal_id)
        .bind(&kr.kr_type)
        .bind(&kr.direction)
        .bind(&kr.unit)
        .bind(kr.baseline)
        .bind(kr.target)
        .bind(kr.current_value)
        .bind(&kr.target_date)
        .bind(kr.score)
        .bind(kr.progress)
        .bind(&kr.created_at)
        .execute(pool)
        .await;

        match result {
            Ok(_) => {
                // Map entries that pointed to this KR via key_result_id
                let priority_id: Option<i64> =
                    sqlx::query_scalar("SELECT last_insert_rowid()")
                        .fetch_one(pool)
                        .await
                        .ok();

                if let Some(pid) = priority_id {
                    let _ = sqlx::query(
                        "UPDATE brag_entries SET priority_id = ? WHERE key_result_id = ? AND priority_id IS NULL",
                    )
                    .bind(pid)
                    .bind(kr.id)
                    .execute(pool)
                    .await;
                }

                migrated += 1;
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to insert priority for KR {} (user {}): {}",
                    kr.id,
                    kr.user_id,
                    e
                );
                skipped += 1;
            }
        }
    }

    tracing::info!(
        "KR migration complete: {} migrated, {} skipped.",
        migrated,
        skipped
    );

    sqlx::query("INSERT INTO _migrations (id) VALUES (?)")
        .bind(MIGRATION_ID)
        .execute(pool)
        .await
        .unwrap_or_else(|e| panic!("Failed to record migration {}: {}", MIGRATION_ID, e));
}
