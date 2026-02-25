# Architecture

Brag Frog is a server-side rendered Rust web app. This document covers its internals for contributors and anyone extending the codebase.

**Tech stack:** Rust (Axum 0.8) + SQLite (sqlx) + Tera templates + HTMX + vanilla JS. No build step.

## Module Map

```
src/
  lib.rs                # Library crate: module re-exports + AppState struct
  main.rs               # Binary entry: config loading, server startup
  app.rs                # Composition root: state setup, router assembly, static page handlers
  db/
    mod.rs              # SQLite pool setup + migration runner (initial + incremental)
  kernel/               # Shared kernel — cross-cutting concerns
    config.rs           # Config from BRAGFROG_* env vars (+ .env via dotenvy)
    crypto.rs           # AES-256-GCM encrypt/decrypt; UserCrypto per-user wrapper
    error.rs            # AppError enum → HTTP status mapping (400/401/404/500)
    http.rs             # Shared HTTP client factory (30s timeout)
    middleware.rs       # security_headers, csrf_protection
    render.rs           # hx_redirect helper, markdown Tera filter
    serde_helpers.rs    # deserialize_optional_i64/string — HTML empty string → None
  identity/             # Authentication & user management
    auth/
      mod.rs            # Google OAuth flow (consent URL, token exchange, user upsert)
      middleware.rs     # require_auth middleware + AuthUser extractor
    model.rs            # User struct
    repo.rs             # User CRUD
    routes.rs           # Login, callback, logout, settings, level-guide
    clg.rs              # Career Level Guide levels from config/clg_levels.toml
  worklog/              # Work entry logging
    model.rs            # BragEntry, EntryType enum, category mapping
    repo.rs             # Entry CRUD, source lookup, soft-delete, classification
    routes.rs           # Quick-create, update, delete, view, classify, exclusions
    service.rs          # Week reassignment, classification result, exclusion logic
  objectives/           # Priorities & department goals
    model/              # Priority, DepartmentGoal structs + form types
    repo/               # Priority, DepartmentGoal persistence
    routes.rs           # Priorities page, CRUD endpoints
    service.rs          # Priority sorting, grouping by department goal
  cycle/                # Performance review cycles
    model/              # BragPhase, Week, Summary, CheckIn, Focus, Meeting, Stories, AI docs
    repo/               # Persistence for all cycle entities
    routes/             # Dashboard, logbook, trends, check-ins, summaries, export, phases
    service/            # Dashboard computation, insights analytics, phase validation
  integrations/         # External service sync
    mod.rs              # SyncService trait, SyncedEntry, ConnectionStatus, re-exports
    orchestrator.rs     # run_sync orchestration, entry persistence, phase window resolution
    factory.rs          # get_sync_service — maps service name to implementation
    validation.rs       # SSRF validation for user-provided base URLs
    model.rs            # IntegrationConfig, SyncLog
    repo.rs             # Integration persistence
    services_config.rs  # Org-specific defaults from config/services.toml
    sync_routes.rs      # POST /sync/{service} handlers
    integrations_routes.rs  # Integrations settings page + API
    github.rs           # GitHub PRs (authored + reviewed + merged)
    phabricator.rs      # Phabricator revisions (Conduit API)
    bugzilla.rs         # Bugzilla bugs (REST API, no token required)
    jira.rs             # Jira issues (JQL search)
    confluence.rs       # Confluence pages (CQL search)
    google_drive.rs     # Google Drive activity (OAuth, refresh tokens)
    google_calendar.rs  # Google Calendar events (OAuth, refresh tokens)
  ai/
    mod.rs              # AiClient — thin Anthropic Messages API wrapper (per-user)
    prompts.rs          # Prompt builder: phase context + section instruction from TOML
config/
  clg_levels.toml       # Career Level Guide levels (IC ladder)
  review_sections.toml  # Performance review sections + AI prompts
  services.toml         # Service URLs, default orgs, allowed_* sync filters
templates/
  base.html             # Layout: header, nav (Dashboard, Logbook, Goals, Prep, Review), panels
  pages/                # Full page templates (dashboard, logbook, goals, settings, etc.)
  components/           # HTMX partial fragments (entry_card, initiative_card, etc.)
  panels/               # Slide-over panel content (entry detail, initiative detail)
static/
  css/tokens.css        # Design tokens (colors, fonts, spacing)
  css/main.css          # Page layouts, header, forms
  css/components/       # Component-specific CSS (dashboard, goals, phases, panels, etc.)
  js/htmx.min.js        # Vendored HTMX 2.x
  js/app.js             # Minimal JS helpers (panel system, command palette, dropdowns)
migrations/
  001_initial.sql       # Full schema + indexes (frozen — never modify)
  002_user_settings_and_focus.sql  # User profile fields, weekly focus, meeting prep notes
tests/
  integration/
    common.rs           # Test helpers (in-memory pool, crypto, fixtures)
    db_tests.rs         # Integration tests (entry CRUD, cascades, week find-or-create)
    http_tests.rs       # HTTP route tests (page loads, auth, entry operations)
    sync_tests.rs       # Sync integration tests (entry creation, dedup, week mapping)
```

## Request Lifecycle

```
TCP connection
  → TcpListener (tokio)
    → axum::serve
      → security_headers middleware (CSP, X-Frame-Options, nosniff, Referrer-Policy)
        → SessionManagerLayer (tower-sessions + SQLite store)
          → Router dispatch
            → [protected routes only] csrf_protection middleware (rejects non-HTMX state-changing requests)
              → require_auth middleware (session check → 302 or hx-redirect)
                → AuthUser extractor (session → user_id + UserCrypto)
                  → Route handler
                    → Tera template render
                      → HTML response
```

Key details:
- **Security headers** are injected on every response via Axum middleware
- **CSRF protection** uses the `HX-Request` header as a lightweight CSRF token — cross-origin requests can't set custom headers without CORS preflight, which the server doesn't allow
- **Static files** are served directly via `tower_http::ServeDir` at `/static` and `/custom`

## Authentication

Google OAuth 2.0 with session-based auth:

1. **Login:** User clicks "Sign in" → redirect to Google consent screen with CSRF `state` token
2. **Callback:** Google redirects back with auth code → server exchanges for tokens → fetches user profile
3. **Domain check:** If `BRAGFROG_ALLOWED_DOMAIN` is set, verifies `hd` (hosted domain) claim matches
4. **Upsert:** User row created/updated in SQLite with Google `sub`, email, name, picture
5. **Session:** User ID stored in `tower-sessions` SQLite-backed session (12h expiry)
6. **AuthUser extractor:** Protected handlers use `AuthUser` as an Axum extractor — pulls `user_id` from session and creates a `UserCrypto` scoped to that user for token decryption

HTMX requests that fail auth get a `401` with `hx-redirect: /` header (not an HTML redirect).

### Google Drive / Calendar OAuth

These use separate OAuth flows with specific scopes (`drive.activity.readonly`, `calendar.events.readonly`). The refresh token is encrypted and stored in the user's integration config. On sync, the token is refreshed via `refresh_access_token()`.

## Database

### SQLite Setup
- WAL journal mode, foreign keys enabled, max 5 connections
- Database file at `BRAGFROG_DATABASE_PATH` (default: `bragfrog.db`), auto-created if missing
- All dates stored as `TEXT` in `YYYY-MM-DD` format

### Migration System
Migrations are numbered SQL files in `migrations/`.

- `001_initial.sql` — full schema baseline. **Never modify this file.** Runs once when `users` table doesn't exist.
- `002_user_settings_and_focus.sql` — user profile fields, weekly focus, meeting prep notes.

Incremental migrations (002+) are tracked in a `_migrations` table. The system is in `src/db/mod.rs`:

1. `run_migrations()` runs the initial schema if needed, then calls `run_incremental_migrations()`
2. Incremental migrations are registered in the `INCREMENTAL_MIGRATIONS` const array as `(id, sql)` tuples
3. Each migration is included at compile time via `include_str!` and only runs if its ID isn't in `_migrations`

To add a new migration:
1. Create `migrations/NNN_description.sql` with plain SQL statements
2. Add the entry to `INCREMENTAL_MIGRATIONS` in `src/db/mod.rs`
3. Integration tests run all migrations automatically via `db::run_migrations`

### Schema Relationships

```
User 1──N BragPhase 1──N Week 1──N BragEntry
                    1──N Goal 1──N KeyResult
                    1──N Initiative
                    1──N Summary

BragEntry  N──1 KeyResult   (optional FK: key_result_id)
BragEntry  N──1 Initiative  (optional FK: initiative_id)
KeyResult  N──1 Goal        (optional FK: goal_id)
Initiative N──N KeyResult   (via initiative_key_results join table)

Week 1──1 WeeklyFocus 1──N WeeklyFocusItem
Week 1──N WeeklyCheckin 1──N KrCheckinSnapshot
Week 1──N MeetingPrepNote
```

Hierarchy: **Goal → KeyResult → Entry** (via FKs). **Initiative → Entry** (via `initiative_id`). **Initiative ↔ KeyResult** (many-to-many).

Data flow: `WeeklyFocus` tracks what KRs/Initiatives the user is focusing on this week. `MeetingPrepNote` attaches prep notes to meeting entries. `WeeklyCheckin` captures weekly reflections with per-KR snapshots.

### Deduplication
Synced entries use `UNIQUE(source, source_id)` with `ON CONFLICT` upsert. Soft-deleted entries (user removed them) are skipped on re-sync.

### Encryption
API tokens are encrypted with AES-256-GCM before storage. A random 12-byte nonce is prepended to the ciphertext. The master key comes from `BRAGFROG_ENCRYPTION_KEY` (base64-encoded 32 bytes). `UserCrypto` is a per-user wrapper that provides `encrypt()`/`decrypt()` methods.

### Query Style
All queries use runtime string SQL (`query` / `query_as`), not the `query!` macro. SQL errors surface at runtime, not compile time. Models use `sqlx::FromRow` derive.

## Sync Service Architecture

### The SyncService Trait
```rust
#[async_trait]
pub trait SyncService: Send + Sync {
    async fn sync(&self, client, token, config, start_date, end_date) -> Result<Vec<SyncedEntry>>;
    async fn test_connection(&self, client, token, config) -> Result<ConnectionStatus>;
}
```

Uses `async_trait` for object safety (`Box<dyn SyncService>` dispatch). Each service (GitHub, Phabricator, Bugzilla, Atlassian, Google Drive, Google Calendar) implements this trait.

### Sync Orchestrator (`run_sync`)
1. Decrypts the user's API token for the service
2. Loads the active phase's date range
3. Injects `allowed_*` filters from `services.toml` into the service config
4. Calls `service.sync()` to fetch work items
5. For each `SyncedEntry`: finds/creates the correct `Week`, upserts the `BragEntry`
6. Skips soft-deleted entries (respects user removals)
7. Records a `SyncLog` with created/updated/unchanged/skipped counts

### SSRF Protection
`validate_base_url()` rejects user-provided service URLs that point to:
- Non-HTTPS schemes
- Localhost / loopback addresses
- Private IP ranges (RFC-1918, link-local, CGNAT)

## Template System

### Tera Loading
Templates are loaded once at startup via `Tera::new("templates/**/*.html")`. Template paths in `render()` calls must match the file path relative to `templates/` (e.g., `"pages/logbook.html"`, `"components/entry_card.html"`).

Custom Tera filters: `markdown` (pulldown-cmark + ammonia sanitization), `entry_type_label`, `source_label`.

### HTMX Integration
- `hx-boost="true"` on `<body>` intercepts all `<a>` and `<form>` submissions
- Entry CRUD returns HTML fragments for in-place `hx-swap="outerHTML"`
- Delete handlers return empty string (element removed from DOM)
- Auth failures on HTMX requests use `hx-redirect` header
- `hx-on::after-request` for post-action cleanup (form resets)

### Page Types
| Pattern | Returns | Example |
|---------|---------|---------|
| Full page | `render("pages/xxx.html", &ctx)` | Logbook, phases, settings |
| HTMX partial | `render("components/xxx.html", &ctx)` | Entry card, goal row |
| Redirect | `hx_redirect("/path")` | After phase create/delete |

All template contexts include `user` and `current_page`. Most also include `phase` (active phase).

## Config Overlay System

Three TOML config files are loaded at startup into `OnceLock` statics:

| File | Module | Purpose |
|------|--------|---------|
| `clg_levels.toml` | `clg.rs` | Career Level Guide levels (IC ladder) |
| `review_sections.toml` | `models/summary.rs` | Performance review sections + AI prompts |
| `services.toml` | `services_config.rs` | Service URLs, placeholders, allowed_* filters |

**Overlay:** The app checks `custom/{file}` first, falls back to `config/{file}`. The `custom/` directory is gitignored. This lets orgs customize config without modifying tracked files.

See [customization.md](customization.md) for the full customization guide.

## AI Integration

### Per-User Client
There is no global AI client. Each user configures their own Anthropic API key via the Integrations page. `AiClient` is instantiated per-request with the user's decrypted key and the model from `BRAGFROG_AI_MODEL` (default: `claude-sonnet-4-5-20250929`).

### Prompt Building (`ai/prompts.rs`)
`build_self_reflection_prompt()` assembles a prompt for one self-review section:

1. **Phase context** — entry statistics, goals with key results, entries grouped by goal, unlinked entries
2. **CLG context** (optional) — current level expectations, and if targeting promotion, next-level expectations
3. **Section instruction** — loaded from `review_sections.toml`, with optional promotion addendum

### Review Sections
Sections are defined in `config/review_sections.toml` and loaded at startup into `OnceLock<ReviewConfig>`. Access via `summary::section_slugs()`, `section_title()`, `section_question()`, `get_section()`. Templates check `has_ai` to show/hide AI buttons.

## Crate Structure

`lib.rs` + `main.rs` dual-crate setup:
- `lib.rs` declares all modules and exports `AppState`
- `main.rs` is the binary entry point that imports from `brag_frog::`
- This allows integration tests in `tests/` to import the library directly
