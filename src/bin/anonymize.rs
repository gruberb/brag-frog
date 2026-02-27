/// Anonymizer binary for producing screenshot-safe database copies.
///
/// Reads the production database, copies it, and replaces all PII (names, emails,
/// teams, encrypted content) with deterministic fake data. The same real value
/// always maps to the same fake value for consistency across tables.
///
/// Usage: cargo run --bin anonymize
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use brag_frog::kernel::crypto::Crypto;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};

// ---------------------------------------------------------------------------
// Word lists for deterministic fake data
// ---------------------------------------------------------------------------

const FIRST_NAMES: &[&str] = &[
    "Ada", "Bjorn", "Camille", "Dmitri", "Elena", "Farid", "Greta", "Hiro",
    "Ingrid", "Jules", "Kira", "Liam", "Maren", "Niko", "Olga", "Pavel",
    "Quinn", "Rosa", "Sven", "Talia", "Uri", "Vera", "Wren", "Xander",
    "Yara", "Zane", "Amara", "Bryn", "Cora", "Dex",
];

const LAST_NAMES: &[&str] = &[
    "Aldrin", "Beckett", "Chen", "Dubois", "Engel", "Frost", "Garcia",
    "Holm", "Ivanov", "Jansen", "Kim", "Larsen", "Moss", "Nilsen",
    "Okoye", "Park", "Reyes", "Singh", "Torres", "Ueda", "Voss",
    "Walker", "Xu", "Yamada", "Zhang", "Abara", "Bloom", "Cruz",
    "Dahl", "Emery",
];

const TEAM_NAMES: &[&str] = &[
    "Aurora", "Nebula", "Horizon", "Quasar", "Vertex", "Pinnacle",
    "Cascade", "Meridian", "Solstice", "Zenith",
];

const TECH_WORDS: &[&str] = &[
    "pipeline", "refactor", "deploy", "service", "config", "module",
    "handler", "schema", "query", "index", "cache", "metric",
    "widget", "render", "parser", "router", "buffer", "daemon",
    "socket", "broker", "filter", "driver", "cluster", "shard",
    "beacon", "signal", "stream", "bridge", "kernel", "digest",
];

const LOREM_WORDS: &[&str] = &[
    "lorem", "ipsum", "dolor", "sit", "amet", "consectetur", "adipiscing",
    "elit", "sed", "tempor", "incididunt", "labore", "dolore", "magna",
    "aliqua", "enim", "minim", "veniam", "quis", "nostrud", "exercitation",
    "ullamco", "laboris", "nisi", "aliquip", "commodo", "consequat",
    "duis", "aute", "irure", "voluptate", "velit", "esse", "cillum",
    "fugiat", "nulla", "pariatur", "excepteur", "sint", "occaecat",
];

// ---------------------------------------------------------------------------
// Deterministic hashing helpers
// ---------------------------------------------------------------------------

fn hash_str(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

fn fake_first(s: &str) -> &'static str {
    FIRST_NAMES[(hash_str(s) % FIRST_NAMES.len() as u64) as usize]
}

fn fake_last(s: &str) -> &'static str {
    // Use a different seed so first+last aren't correlated
    LAST_NAMES[(hash_str(&format!("last:{s}")) % LAST_NAMES.len() as u64) as usize]
}

fn fake_full_name(s: &str) -> String {
    format!("{} {}", fake_first(s), fake_last(s))
}

fn fake_email(name: &str) -> String {
    let first = fake_first(name).to_lowercase();
    let last = fake_last(name).to_lowercase();
    format!("{first}.{last}@example.com")
}

fn fake_team(s: &str) -> String {
    TEAM_NAMES[(hash_str(s) % TEAM_NAMES.len() as u64) as usize].to_string()
}

/// Replace words longer than 3 chars with tech jargon, keep short words and punctuation.
fn fake_title(original: &str) -> String {
    original
        .split_whitespace()
        .map(|word| {
            let alpha: String = word.chars().filter(|c| c.is_alphanumeric()).collect();
            if alpha.len() <= 3 {
                word.to_string()
            } else {
                let replacement = TECH_WORDS
                    [(hash_str(word.to_lowercase().as_str()) % TECH_WORDS.len() as u64) as usize];
                // Preserve trailing punctuation
                let trailing: String = word.chars().rev().take_while(|c| !c.is_alphanumeric()).collect::<String>().chars().rev().collect();
                format!("{replacement}{trailing}")
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Generate lorem-style text roughly matching the original's length.
fn fake_longform(original: &str) -> String {
    let target_len = original.len();
    let mut result = String::with_capacity(target_len + 50);
    let mut i = hash_str(original) as usize;
    let mut sentence_pos = 0;

    while result.len() < target_len {
        let word = LOREM_WORDS[i % LOREM_WORDS.len()];
        if sentence_pos == 0 {
            // Capitalize first word of sentence
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                result.push(first.to_uppercase().next().unwrap_or(first));
                result.extend(chars);
            }
        } else {
            result.push_str(word);
        }
        sentence_pos += 1;

        if sentence_pos > 8 && i.is_multiple_of(5) {
            result.push('.');
            result.push(' ');
            sentence_pos = 0;
        } else {
            result.push(' ');
        }
        i += 1;
    }

    result.truncate(target_len);
    result.trim().to_string()
}

/// Anonymize a comma-separated list of names.
fn fake_name_list(original: &str) -> String {
    original
        .split(',')
        .map(|name| fake_full_name(name.trim()))
        .collect::<Vec<_>>()
        .join(", ")
}

// ---------------------------------------------------------------------------
// Database helpers
// ---------------------------------------------------------------------------

async fn open_pool(path: &Path) -> Result<SqlitePool, Box<dyn std::error::Error>> {
    let opts = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(false);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await?;
    Ok(pool)
}

// ---------------------------------------------------------------------------
// Plaintext anonymization
// ---------------------------------------------------------------------------

async fn anonymize_users(pool: &SqlitePool) -> Result<u64, Box<dyn std::error::Error>> {
    let rows = sqlx::query("SELECT id, email, name FROM users")
        .fetch_all(pool)
        .await?;

    let mut count = 0u64;
    for row in &rows {
        let id: i64 = row.get("id");
        let email: String = row.get("email");
        let name: String = row.get("name");

        let anon_name = fake_full_name(&name);
        let anon_display = anon_name.clone();
        let anon_email = fake_email(&email);
        let anon_team = fake_team(&email);
        let anon_manager = fake_full_name(&format!("mgr:{name}"));
        let anon_skip = fake_full_name(&format!("skip:{name}"));

        sqlx::query(
            "UPDATE users SET
                name = ?1, email = ?2, display_name = ?3, team = ?4,
                manager_name = ?5, skip_level_name = ?6, direct_reports = NULL,
                avatar_url = NULL, google_id = ?7
             WHERE id = ?8",
        )
        .bind(&anon_name)
        .bind(&anon_email)
        .bind(&anon_display)
        .bind(&anon_team)
        .bind(&anon_manager)
        .bind(&anon_skip)
        .bind(format!("anon-google-id-{id}"))
        .bind(id)
        .execute(pool)
        .await?;
        count += 1;
    }
    Ok(count)
}

async fn anonymize_people_aliases(pool: &SqlitePool) -> Result<u64, Box<dyn std::error::Error>> {
    let rows = sqlx::query("SELECT id, email, display_name FROM people_aliases")
        .fetch_all(pool)
        .await?;

    let mut count = 0u64;
    for row in &rows {
        let id: i64 = row.get("id");
        let email: String = row.get("email");
        let display_name: String = row.get("display_name");

        let anon_name = fake_full_name(&display_name);
        let anon_email = fake_email(&email);
        let anon_team = fake_team(&email);

        sqlx::query(
            "UPDATE people_aliases SET email = ?1, display_name = ?2, team = ?3 WHERE id = ?4",
        )
        .bind(&anon_email)
        .bind(&anon_name)
        .bind(&anon_team)
        .bind(id)
        .execute(pool)
        .await?;
        count += 1;
    }
    Ok(count)
}

async fn anonymize_meeting_rules(pool: &SqlitePool) -> Result<u64, Box<dyn std::error::Error>> {
    let rows = sqlx::query("SELECT id, person_name FROM meeting_rules WHERE person_name IS NOT NULL")
        .fetch_all(pool)
        .await?;

    let mut count = 0u64;
    for row in &rows {
        let id: i64 = row.get("id");
        let person_name: String = row.get("person_name");

        sqlx::query("UPDATE meeting_rules SET person_name = ?1 WHERE id = ?2")
            .bind(fake_full_name(&person_name))
            .bind(id)
            .execute(pool)
            .await?;
        count += 1;
    }
    Ok(count)
}

async fn anonymize_integration_configs(
    pool: &SqlitePool,
) -> Result<u64, Box<dyn std::error::Error>> {
    let result = sqlx::query(
        "UPDATE integration_configs SET encrypted_token = NULL, config_json = NULL, last_sync_error = NULL, is_enabled = 0",
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

async fn anonymize_plaintext_urls(pool: &SqlitePool) -> Result<u64, Box<dyn std::error::Error>> {
    let mut total = 0u64;

    // brag_entries: source_url, evidence_urls, repository
    let r = sqlx::query(
        "UPDATE brag_entries SET
            source_url = CASE WHEN source_url IS NOT NULL THEN 'https://example.com/item/' || id END,
            evidence_urls = CASE WHEN evidence_urls IS NOT NULL THEN 'https://example.com/evidence/' || id END,
            repository = CASE WHEN repository IS NOT NULL THEN 'acme/project-' || (id % 10) END,
            source_id = CASE WHEN source_id IS NOT NULL THEN 'anon-' || id END",
    )
    .execute(pool)
    .await?;
    total += r.rows_affected();

    // meeting_prep_notes: doc_urls
    let r = sqlx::query(
        "UPDATE meeting_prep_notes SET doc_urls = CASE WHEN doc_urls IS NOT NULL THEN 'https://example.com/doc/' || id END",
    )
    .execute(pool)
    .await?;
    total += r.rows_affected();

    // weekly_focus: link_1, link_2, link_3
    let r = sqlx::query(
        "UPDATE weekly_focus SET
            link_1 = CASE WHEN link_1 IS NOT NULL THEN 'https://example.com/link/' || id END,
            link_2 = CASE WHEN link_2 IS NOT NULL THEN 'https://example.com/link/' || id || '-2' END,
            link_3 = CASE WHEN link_3 IS NOT NULL THEN 'https://example.com/link/' || id || '-3' END",
    )
    .execute(pool)
    .await?;
    total += r.rows_affected();

    // ai_documents: title (plaintext TEXT column)
    let r = sqlx::query("UPDATE ai_documents SET title = 'AI Document ' || id")
        .execute(pool)
        .await?;
    total += r.rows_affected();

    // sync_logs: error messages may contain PII
    let r = sqlx::query("UPDATE sync_logs SET error_message = NULL")
        .execute(pool)
        .await?;
    total += r.rows_affected();

    // meeting_rules: match_value may contain calendar event titles or names
    let r = sqlx::query(
        "UPDATE meeting_rules SET match_value = 'match-' || id WHERE match_type = 'title_contains'",
    )
    .execute(pool)
    .await?;
    total += r.rows_affected();

    // brag_entries: recurring_group may contain real meeting names
    let r = sqlx::query(
        "UPDATE brag_entries SET recurring_group = 'recurring-' || (id % 20) WHERE recurring_group IS NOT NULL",
    )
    .execute(pool)
    .await?;
    total += r.rows_affected();

    // ai_documents: recurring_group
    let r = sqlx::query(
        "UPDATE ai_documents SET recurring_group = 'recurring-' || (id % 20) WHERE recurring_group IS NOT NULL",
    )
    .execute(pool)
    .await?;
    total += r.rows_affected();

    Ok(total)
}

// ---------------------------------------------------------------------------
// Encrypted field anonymization
// ---------------------------------------------------------------------------

/// Describes a BLOB column that needs decrypt → anonymize → re-encrypt.
struct EncryptedColumn {
    table: &'static str,
    column: &'static str,
    /// How to generate fake content: "title" (short tech jargon) or "longform" (lorem).
    style: ContentStyle,
}

enum ContentStyle {
    Title,
    Longform,
    NameList,
}

const ENCRYPTED_COLUMNS: &[EncryptedColumn] = &[
    // brag_entries
    EncryptedColumn { table: "brag_entries", column: "title", style: ContentStyle::Title },
    EncryptedColumn { table: "brag_entries", column: "description", style: ContentStyle::Longform },
    EncryptedColumn { table: "brag_entries", column: "collaborators", style: ContentStyle::NameList },
    EncryptedColumn { table: "brag_entries", column: "teams", style: ContentStyle::Title },
    EncryptedColumn { table: "brag_entries", column: "outcome_statement", style: ContentStyle::Longform },
    EncryptedColumn { table: "brag_entries", column: "decision_alternatives", style: ContentStyle::Longform },
    EncryptedColumn { table: "brag_entries", column: "decision_reasoning", style: ContentStyle::Longform },
    EncryptedColumn { table: "brag_entries", column: "decision_outcome", style: ContentStyle::Longform },
    // priorities
    EncryptedColumn { table: "priorities", column: "title", style: ContentStyle::Title },
    EncryptedColumn { table: "priorities", column: "impact_narrative", style: ContentStyle::Longform },
    // department_goals
    EncryptedColumn { table: "department_goals", column: "title", style: ContentStyle::Title },
    EncryptedColumn { table: "department_goals", column: "description", style: ContentStyle::Longform },
    // weekly_focus
    EncryptedColumn { table: "weekly_focus", column: "title", style: ContentStyle::Title },
    // weekly_checkins
    EncryptedColumn { table: "weekly_checkins", column: "proud_of", style: ContentStyle::Longform },
    EncryptedColumn { table: "weekly_checkins", column: "learned", style: ContentStyle::Longform },
    EncryptedColumn { table: "weekly_checkins", column: "wants_to_change", style: ContentStyle::Longform },
    EncryptedColumn { table: "weekly_checkins", column: "frustrations", style: ContentStyle::Longform },
    EncryptedColumn { table: "weekly_checkins", column: "notes", style: ContentStyle::Longform },
    EncryptedColumn { table: "weekly_checkins", column: "highlights_impact", style: ContentStyle::Longform },
    EncryptedColumn { table: "weekly_checkins", column: "learnings_adjustments", style: ContentStyle::Longform },
    EncryptedColumn { table: "weekly_checkins", column: "growth_development", style: ContentStyle::Longform },
    EncryptedColumn { table: "weekly_checkins", column: "support_feedback", style: ContentStyle::Longform },
    EncryptedColumn { table: "weekly_checkins", column: "looking_ahead", style: ContentStyle::Longform },
    // quarterly_checkins
    EncryptedColumn { table: "quarterly_checkins", column: "highlights_impact", style: ContentStyle::Longform },
    EncryptedColumn { table: "quarterly_checkins", column: "learnings_adjustments", style: ContentStyle::Longform },
    EncryptedColumn { table: "quarterly_checkins", column: "growth_development", style: ContentStyle::Longform },
    EncryptedColumn { table: "quarterly_checkins", column: "support_feedback", style: ContentStyle::Longform },
    EncryptedColumn { table: "quarterly_checkins", column: "looking_ahead", style: ContentStyle::Longform },
    // contribution_examples
    EncryptedColumn { table: "contribution_examples", column: "title", style: ContentStyle::Title },
    EncryptedColumn { table: "contribution_examples", column: "outcome", style: ContentStyle::Longform },
    EncryptedColumn { table: "contribution_examples", column: "behaviors", style: ContentStyle::Longform },
    EncryptedColumn { table: "contribution_examples", column: "learnings", style: ContentStyle::Longform },
    // meeting_prep_notes
    EncryptedColumn { table: "meeting_prep_notes", column: "notes", style: ContentStyle::Longform },
    EncryptedColumn { table: "meeting_prep_notes", column: "meeting_goal", style: ContentStyle::Longform },
    // summaries
    EncryptedColumn { table: "summaries", column: "content", style: ContentStyle::Longform },
    EncryptedColumn { table: "summaries", column: "prompt_used", style: ContentStyle::Longform },
    // ai_documents
    EncryptedColumn { table: "ai_documents", column: "content", style: ContentStyle::Longform },
    EncryptedColumn { table: "ai_documents", column: "prompt_used", style: ContentStyle::Longform },
    // goals (legacy, may still have data)
    EncryptedColumn { table: "goals", column: "title", style: ContentStyle::Title },
    EncryptedColumn { table: "goals", column: "description", style: ContentStyle::Longform },
    // initiatives (legacy)
    EncryptedColumn { table: "initiatives", column: "title", style: ContentStyle::Title },
    EncryptedColumn { table: "initiatives", column: "description", style: ContentStyle::Longform },
    // impact_stories (legacy)
    EncryptedColumn { table: "impact_stories", column: "title", style: ContentStyle::Title },
    EncryptedColumn { table: "impact_stories", column: "situation", style: ContentStyle::Longform },
    EncryptedColumn { table: "impact_stories", column: "actions", style: ContentStyle::Longform },
    EncryptedColumn { table: "impact_stories", column: "result", style: ContentStyle::Longform },
    // kr_checkin_snapshots
    EncryptedColumn { table: "kr_checkin_snapshots", column: "blockers", style: ContentStyle::Longform },
    EncryptedColumn { table: "kr_checkin_snapshots", column: "next_week_bet", style: ContentStyle::Longform },
    // annual_alignment
    EncryptedColumn { table: "annual_alignment", column: "top_outcomes", style: ContentStyle::Longform },
    EncryptedColumn { table: "annual_alignment", column: "why_it_matters", style: ContentStyle::Longform },
    EncryptedColumn { table: "annual_alignment", column: "success_criteria", style: ContentStyle::Longform },
    EncryptedColumn { table: "annual_alignment", column: "learning_goals", style: ContentStyle::Longform },
    EncryptedColumn { table: "annual_alignment", column: "support_needed", style: ContentStyle::Longform },
];

/// Resolves which user_id owns a row, depending on the table's schema.
/// Tables either have a direct user_id column, or reach it through phase_id or week_id.
fn user_id_join(table: &str) -> &'static str {
    match table {
        "brag_entries" => {
            "JOIN weeks w ON t.week_id = w.id JOIN brag_phases bp ON w.phase_id = bp.id"
        }
        "weekly_checkins" | "meeting_prep_notes" | "weekly_focus" => {
            // These have a direct user_id column
            ""
        }
        "priorities" | "quarterly_checkins" | "annual_alignment" | "ai_documents" => {
            // Direct user_id column
            ""
        }
        "kr_checkin_snapshots" => {
            "JOIN weekly_checkins wc ON t.checkin_id = wc.id"
        }
        _ => {
            // department_goals, contribution_examples, summaries, goals, initiatives,
            // impact_stories — go through phase_id
            "JOIN brag_phases bp ON t.phase_id = bp.id"
        }
    }
}

fn user_id_expr(table: &str) -> &'static str {
    match table {
        "brag_entries" => "bp.user_id",
        "kr_checkin_snapshots" => "wc.user_id",
        "weekly_checkins" | "meeting_prep_notes" | "weekly_focus" | "priorities"
        | "quarterly_checkins" | "annual_alignment" | "ai_documents" => "t.user_id",
        _ => "bp.user_id",
    }
}

async fn anonymize_encrypted_columns(
    pool: &SqlitePool,
    crypto: &Crypto,
) -> Result<u64, Box<dyn std::error::Error>> {
    let mut total = 0u64;

    // Cache UserCrypto instances per user_id
    let mut user_crypto_cache: HashMap<i64, brag_frog::kernel::crypto::UserCrypto> = HashMap::new();

    for col_def in ENCRYPTED_COLUMNS {
        let join = user_id_join(col_def.table);
        let uid_expr = user_id_expr(col_def.table);

        // Check if the table exists (legacy tables may have been dropped)
        let table_check = sqlx::query(&format!(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='{}'",
            col_def.table
        ))
        .fetch_optional(pool)
        .await?;

        if table_check.is_none() {
            continue;
        }

        let query = format!(
            "SELECT t.id, t.{col} AS blob_data, {uid} AS user_id FROM {tbl} t {join} WHERE t.{col} IS NOT NULL",
            col = col_def.column,
            tbl = col_def.table,
            uid = uid_expr,
            join = join,
        );

        let rows = match sqlx::query(&query).fetch_all(pool).await {
            Ok(rows) => rows,
            Err(e) => {
                // Column may not exist in older schemas
                eprintln!(
                    "  Skipping {}.{}: {}",
                    col_def.table, col_def.column, e
                );
                continue;
            }
        };

        let mut col_count = 0u64;
        for row in &rows {
            let id: i64 = row.get("id");
            let blob: Vec<u8> = row.get("blob_data");
            let user_id: i64 = row.get("user_id");

            let uc = user_crypto_cache
                .entry(user_id)
                .or_insert_with(|| crypto.for_user(user_id).expect("Failed to derive user key"));

            let plaintext = match uc.decrypt(&blob) {
                Ok(text) => text,
                Err(e) => {
                    eprintln!(
                        "  Decrypt failed for {}.{} id={}: {}",
                        col_def.table, col_def.column, id, e
                    );
                    continue;
                }
            };

            if plaintext.is_empty() {
                continue;
            }

            let fake = match col_def.style {
                ContentStyle::Title => fake_title(&plaintext),
                ContentStyle::Longform => fake_longform(&plaintext),
                ContentStyle::NameList => fake_name_list(&plaintext),
            };

            let encrypted = uc.encrypt(&fake).expect("Encryption should not fail");

            let update_sql = format!(
                "UPDATE {} SET {} = ?1 WHERE id = ?2",
                col_def.table, col_def.column
            );
            sqlx::query(&update_sql)
                .bind(&encrypted)
                .bind(id)
                .execute(pool)
                .await?;
            col_count += 1;
        }

        if col_count > 0 {
            println!("  {}.{}: {} rows", col_def.table, col_def.column, col_count);
            total += col_count;
        }
    }

    Ok(total)
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    // Read config
    let encryption_key = std::env::var("BRAGFROG_ENCRYPTION_KEY")
        .map_err(|_| "BRAGFROG_ENCRYPTION_KEY not set in .env")?;
    let db_path = std::env::var("BRAGFROG_DATABASE_PATH").unwrap_or_else(|_| "bragfrog.db".into());

    let source = PathBuf::from(&db_path);
    if !source.exists() {
        return Err(format!("Database not found: {}", source.display()).into());
    }

    // Build output path: bragfrog.db → bragfrog_anon.db
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("db");
    let ext = source
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("db");
    let dest = source.with_file_name(format!("{stem}_anon.{ext}"));

    // Copy database
    println!("Copying {} → {}", source.display(), dest.display());
    std::fs::copy(&source, &dest)?;

    // Also copy WAL and SHM if they exist (for clean state)
    let wal = source.with_extension(format!("{ext}-wal"));
    let shm = source.with_extension(format!("{ext}-shm"));
    if wal.exists() {
        std::fs::copy(&wal, dest.with_extension(format!("{ext}-wal")))?;
    }
    if shm.exists() {
        std::fs::copy(&shm, dest.with_extension(format!("{ext}-shm")))?;
    }

    let pool = open_pool(&dest).await?;
    let crypto = Crypto::new(&encryption_key)?;

    // Checkpoint WAL into the main DB file so we have all data
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&pool)
        .await?;

    println!("\n--- Anonymizing plaintext fields ---");

    let n = anonymize_users(&pool).await?;
    println!("  users: {} rows", n);

    let n = anonymize_people_aliases(&pool).await?;
    println!("  people_aliases: {} rows", n);

    let n = anonymize_meeting_rules(&pool).await?;
    println!("  meeting_rules: {} rows", n);

    let n = anonymize_integration_configs(&pool).await?;
    println!("  integration_configs: {} rows", n);

    let n = anonymize_plaintext_urls(&pool).await?;
    println!("  plaintext URLs/IDs: {} rows", n);

    println!("\n--- Anonymizing encrypted fields ---");
    let n = anonymize_encrypted_columns(&pool, &crypto).await?;
    println!("  Total encrypted fields: {} values", n);

    // Final checkpoint
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&pool)
        .await?;

    pool.close().await;

    // Clean up WAL/SHM files from the anon copy
    let anon_wal = dest.with_extension(format!("{ext}-wal"));
    let anon_shm = dest.with_extension(format!("{ext}-shm"));
    let _ = std::fs::remove_file(&anon_wal);
    let _ = std::fs::remove_file(&anon_shm);

    println!("\nDone! Anonymized database: {}", dest.display());
    println!("Start with: BRAGFROG_DATABASE_PATH={} cargo run", dest.display());

    Ok(())
}
