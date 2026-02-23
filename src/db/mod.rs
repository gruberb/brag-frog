use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;

/// Initializes the SQLite connection pool with WAL journal mode, foreign keys enabled,
/// create-if-missing, and a max of 5 connections. Panics on failure.
pub async fn setup_pool(database_path: &str) -> SqlitePool {
    let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", database_path))
        .expect("Invalid database path")
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
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
const INCREMENTAL_MIGRATIONS: &[(&str, &str)] = &[];

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
