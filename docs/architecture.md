# Architecture

Brag Frog is a moon monorepo: a Rust JSON API (`apps/api`) and a React single-page app (`apps/web`). In production the API serves the built web app, so one process and one container run everything. This document covers the internals for contributors.

**Tech stack:** Rust (Axum 0.8) + SQLite (sqlx) for the API. React 19 + TypeScript + Vite + React Router 7 + TanStack Query + Tailwind v4 + BlockNote for the web app.

## Repository Layout

```
apps/api/        Rust crate `brag-frog`
apps/web/        React app
.moon/           moon workspace.yml (projects) and toolchains.yml
.prototools      pins moon and Node 24
Cargo.toml       Cargo workspace (members = apps/api)
package.json     npm workspace (members = apps/web)
Dockerfile       multi-stage: node (web build), rust (API build), debian (runtime)
.github/workflows/ci.yml       `moon ci` on pushes to main and pull requests
.github/workflows/release.yml  builds and pushes the image to ghcr.io on `v*` tags
```

Tasks are defined in `apps/api/moon.yml` (`dev`, `build`, `lint`, `test`, `types`, `types-check`) and `apps/web/moon.yml` (`dev`, `build`, `lint`, `typecheck`, `test`).

## Module Map (`apps/api`)

```
src/
  lib.rs                # Library crate: module re-exports + AppState
  main.rs               # Binary entry: config, server startup, hourly sync loop
  app.rs                # Composition root: state setup, router, web app serving, /custom
  api/                  # JSON API under /api/v1 (see "API" below)
    mod.rs              # public_router() and router(): all route registration
    auth.rs             # require_auth_api middleware + ApiAuth extractor (JSON 401)
    error.rs            # ApiError, ApiResult: {"error":{"code","message"}}
    dto.rs              # Shared wire types (UserDto, CycleDto, EntryDto, ...)
    me.rs               # /healthz, /public-config, /me, /auth/logout
    cycles.rs entries.rs today.rs priorities.rs review.rs reports.rs
    trends.rs bundle.rs integrations.rs sync.rs settings.rs config.rs
    facets.rs           # Short-lived per-cycle cache for filter dropdown values
    scope.rs            # Resolves and ownership-checks the cycle for a request
  db/
    mod.rs              # SQLite pool setup + migration runner (initial + incremental + post-SQL)
  kernel/               # Cross-cutting concerns
    config.rs           # Config from BRAGFROG_* env vars (+ .env via dotenvy)
    crypto.rs           # AES-256-GCM encrypt/decrypt; UserCrypto per-user wrapper
    error.rs            # AppError enum
    http.rs             # Shared HTTP client factory (30s timeout)
    middleware.rs       # security_headers, csrf_protection
    serde_helpers.rs    # deserialize_optional_i64/string
  identity/             # Authentication & user management
    auth/               # Google OAuth flow, require_auth, AuthUser extractor
    oauth_state.rs      # OAuth state tokens
    routes.rs           # /auth/login, /auth/callback, /auth/connect/google_*
    model.rs repo.rs    # User, PeopleAlias persistence
    clg.rs              # Career levels from clg_levels.toml
  worklog/              # BragEntry, EntryType, repo, week reassignment, exclusions
  objectives/           # Priorities, department goals, priority updates, Lattice CSV import
  cycle/                # Cycles (BragPhase), weeks, meeting prep notes, dashboard, trends, logbook insights
  review/               # Summaries (review sections), reports, AI documents, report stats
  archive.rs            # Read-only access to retired features' tables (check-ins, examples, status updates, last-week reports)
  integrations/         # External service sync
    mod.rs              # SyncService trait, SyncedEntry, ConnectionStatus
    orchestrator.rs     # run_sync, entry persistence, cycle window resolution
    background.rs       # spawn_sync_all / spawn_sync_service, hourly_sync_loop
    hard_sync.rs        # Hard sync: clears soft-deletes, removes entries the source no longer returns
    sync_status.rs      # In-memory per-user sync status polled by the web app
    catalog.rs          # Per-service labels, settings fields, config JSON helpers
    factory.rs          # get_sync_service: service name to implementation
    validation.rs       # SSRF validation for user-provided base URLs
    services_config.rs  # Org defaults from services.toml
    github.rs phabricator.rs bugzilla.rs jira.rs confluence.rs atlassian.rs
    google_drive.rs google_calendar.rs
  ai/
    mod.rs              # AiClient: thin Anthropic Messages API wrapper (per-user)
    helpers.rs          # get_ai_client, has_ai_for_user
    prompts.rs          # Self-review, meeting prep and report prompt builders
config/
  clg_levels.toml       # Career levels (IC ladder)
  review_sections.toml  # Self-review sections + AI prompts
  services.toml         # Service URLs, default orgs, allowed_* sync filters
migrations/             # 001_initial.sql ... 025_reports.sql
tests/integration/      # api_*_tests.rs (HTTP), db_tests.rs, sync_tests.rs, common.rs
```

## API

All handlers live in `src/api/`. They are thin: they call the repo and service code of the owning module and translate to DTOs. Routes:

| Area | Routes |
|------|--------|
| Public | `GET /healthz`, `GET /api/v1/public-config` |
| Session | `GET /api/v1/me`, `POST /api/v1/auth/logout` |
| Cycles | `GET/POST /cycles`, `DELETE /cycles/{id}`, `POST /cycles/{id}/activate`, `GET /cycles/{id}/bundle`, `GET /cycles/{id}/archive` |
| Entries | `GET/POST /entries`, `GET /entries/facets`, `POST /entries/bulk`, `GET/PUT/DELETE /entries/{id}`, `POST /entries/{id}/classify`, `POST /entries/{id}/exclude`, `GET /meta/entry-types` |
| Today | `GET /today`, `GET/PUT /meetings/{entry_id}/prep`, `POST /meetings/{entry_id}/prep/draft` |
| Priorities | `GET/POST /priorities`, `POST /priorities/import`, `PUT/DELETE /priorities/{id}`, `GET/POST /priorities/{id}/updates`, `POST /goals`, `PUT/DELETE /goals/{id}` |
| Review | `GET /cycles/{id}/review`, `PUT /cycles/{id}/review/{slug}`, `POST /cycles/{id}/review/{slug}/draft` |
| Reports | `GET/POST /reports`, `GET/DELETE /reports/{id}` |
| Other | `GET /config`, `GET /trends`, `GET/POST` settings under `/settings/profile` and `/settings/people` |
| Integrations | `GET /integrations`, `PUT/DELETE /integrations/{service}`, `POST /integrations/{service}/test`, `DELETE /integrations/{service}/excluded/{id}`, `POST /sync`, `GET /sync/status`, `POST /sync/{service}` |

(Paths are relative to `/api/v1`. `apps/api/src/api/mod.rs` is the source of truth.) Unknown `/api/*` paths return a JSON 404.

Wire types derive `ts_rs::TS`. `moon run api:types` exports them to `apps/web/src/types/api/` (export dir set in `.cargo/config.toml`), and `api:types-check` fails when the committed files are stale.

## Request Lifecycle

```
TCP connection
  → TcpListener (tokio)
    → axum::serve
      → CompressionLayer (gzip)
        → security_headers middleware (CSP, X-Frame-Options, nosniff, Referrer-Policy)
          → SessionManagerLayer (tower-sessions + SQLite store)
            → Router dispatch
              ├─ /auth/*, /healthz, /api/v1/public-config   (no session required)
              ├─ /api/v1/*
              │    → csrf_protection (state-changing requests need X-Requested-With: brag-frog)
              │      → require_auth_api (session check, JSON 401)
              │        → ApiAuth extractor (user_id + UserCrypto)
              │          → handler → JSON
              ├─ /custom/tokens.css                          (only this file of the custom dir)
              └─ anything else → web app (ServeDir on BRAGFROG_WEB_DIST, fallback index.html)
```

Key details:
- **Security headers** are injected on every response.
- **CSRF protection** uses a custom request header. Cross-origin pages cannot set it without a CORS preflight, which the server never allows. Requests with `Sec-Fetch-Site: cross-site` are always refused.
- **Cache policy:** `/assets/*` and `/fonts/*` are immutable (hashed), everything else is `no-cache` so a new release is picked up.
- **Same origin:** the SPA and the API share one origin and one session cookie, so there is no CORS and no token handling in the browser. In development, Vite proxies `/api`, `/auth`, `/custom` and `/healthz` to the API.

## Authentication

Google OAuth 2.0 with session-based auth. The OAuth redirects stay server-side (`identity/routes.rs`):

1. **Login:** the sign-in page links to `/auth/login`, which redirects to the Google consent screen with a CSRF `state` token
2. **Callback:** Google redirects to `/auth/callback` with an auth code; the server exchanges it for tokens and fetches the user profile
3. **Domain check:** If `BRAGFROG_ALLOWED_DOMAIN` is set, verifies the `hd` (hosted domain) claim matches
4. **Upsert:** User row created/updated in SQLite with Google `sub`, email, name, picture
5. **Session:** User ID stored in a `tower-sessions` SQLite-backed session (12h inactivity expiry, `HttpOnly`, `SameSite=Lax`, `Secure` when `BRAGFROG_BASE_URL` is https)
6. **ApiAuth extractor:** protected handlers pull `user_id` from the session and get a `UserCrypto` scoped to that user for token decryption

Unauthenticated API requests get a JSON `401`; the web app reacts by redirecting to `/signin`. `POST /api/v1/auth/logout` clears the session.

### Google Drive / Calendar OAuth

These use separate flows (`/auth/connect/google_drive`, `/auth/connect/google_calendar`) with specific scopes (`drive.activity.readonly`, `calendar.events.readonly`). The refresh token is encrypted and stored in the user's integration config. On sync, the token is refreshed via `refresh_access_token()`.

## Database

### SQLite Setup
- WAL journal mode, foreign keys enabled, max 5 connections
- Database file at `BRAGFROG_DATABASE_PATH` (default: `bragfrog.db`), auto-created if missing. `SQLITE_JOURNAL_MODE` overrides WAL for FUSE/NFS volumes
- All dates stored as `TEXT` in `YYYY-MM-DD` format

### Migration System
Migrations are numbered SQL files in `apps/api/migrations/`.

- `001_initial.sql`: full schema baseline. **Never modify this file.** Runs once when `users` table doesn't exist.
- `002` to `025` are incremental (latest: `025_reports.sql`).

Incremental migrations (002+) are tracked in a `_migrations` table. The system is in `apps/api/src/db/mod.rs`. Data migrations that need encryption run afterwards in `run_post_migrations`:

1. `run_migrations()` runs the initial schema if needed, then calls `run_incremental_migrations()`
2. Incremental migrations are registered in the `INCREMENTAL_MIGRATIONS` const array as `(id, sql)` tuples
3. Each migration is included at compile time via `include_str!` and only runs if its ID isn't in `_migrations`

To add a new migration:
1. Create `apps/api/migrations/NNN_description.sql` with plain SQL statements
2. Add the entry to `INCREMENTAL_MIGRATIONS` in `apps/api/src/db/mod.rs`
3. Integration tests run all migrations automatically via `db::run_migrations`

### Schema Relationships

```
User 1──N BragPhase (cycle) 1──N Week 1──N BragEntry
                            1──N DepartmentGoal 1──N Priority 1──N PriorityUpdate
                            1──N Summary (one per review section)
                            1──N Report
                            1──N AiDocument

BragEntry N──1 Priority  (optional FK: priority_id)
Week 1──N MeetingPrepNote
User 1──N PeopleAlias
```

Hierarchy: **DepartmentGoal → Priority → Entry** (via `priority_id` and `department_goal_id`).

Legacy tables with no write path (`quarterly_checkins`, `contribution_examples`, `status_updates`, `last_week_reports`) keep old data. `GET /api/v1/cycles/{id}/archive` returns it read-only.

Data flow: synced and manual entries feed Today, the Logbook, saved reports, meeting prep, trends and self-review drafts.

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
2. Loads the active cycle's date range
3. Injects `allowed_*` filters from `services.toml` into the service config
4. Calls `service.sync()` to fetch work items
5. For each `SyncedEntry`: finds/creates the correct `Week`, upserts the `BragEntry`
6. Skips soft-deleted entries (respects user removals) and refresh-only items without a stored row
7. Deletes pending requests (`pr_review_requested`) the service no longer returns
8. Records a `SyncLog` with created/updated/deleted/unchanged/skipped counts

Syncs run as background tasks (`integrations/background.rs`): `POST /api/v1/sync` and `POST /api/v1/sync/{service}` return immediately, and the web app polls `GET /api/v1/sync/status`. `main.rs` also starts `hourly_sync_loop` for all enabled integrations.

### SSRF Protection
`validate_base_url()` rejects user-provided service URLs that point to:
- Non-HTTPS schemes
- Localhost / loopback addresses
- Private IP ranges (RFC-1918, link-local, CGNAT)

## Web App (`apps/web`)

React 19 SPA built with Vite. See [apps/web/README.md](../apps/web/README.md) for the full guide.

- **Routing:** React Router 7. Views: Today, Logbook, Priorities, Review (plus a read-only archive per cycle), Trends, Settings (profile, cycles, people, integrations, AI; opened from the gear icon), sign-in, privacy and terms. Paths are in `src/config/paths.ts`.
- **Data:** TanStack Query against `/api/v1`. `src/lib/api-client.ts` adds the CSRF header and turns a 401 into a redirect to sign-in.
- **Structure (bulletproof-react):** `app/` composes `features/*`, which use shared `components`, `hooks`, `lib`, `stores`, `types`, `utils`. ESLint (`import/no-restricted-paths`) forbids upward imports and feature-to-feature imports; `src/testing/architecture.test.ts` proves the rule fires.
- **Types:** `src/types/api` is generated from the Rust DTOs and never edited by hand.
- **Editor:** BlockNote, lazy-loaded on the Review view only. Review sections and reports store `content` with a `content_format` (`markdown` for old text, `blocknote` after the first save).
- **Export:** Markdown, PDF (`@react-pdf/renderer`) and DOCX (`docx`) are generated in the browser from `GET /cycles/{id}/bundle`; the PDF and DOCX libraries load only when a download starts.
- **Tests:** Vitest + Testing Library + MSW. The same MSW handlers (`src/testing/mocks`) back `npm run dev:mock`, which runs the UI without a backend.
- **Styling:** Tailwind v4 with design tokens in `src/app/app.css`. `/custom/tokens.css` is linked after the app styles so operators can override tokens.

## Config Overlay System

Three TOML config files are loaded at startup into `OnceLock` statics (the former `assessment_templates.toml`, `checkin_sections.toml` and `rating_scale.toml` are gone):

| File | Module | Purpose |
|------|--------|---------|
| `clg_levels.toml` | `identity/clg.rs` | Career levels (IC ladder), shown in the Review view |
| `review_sections.toml` | `review/model/summary.rs` | Self-review sections + AI prompts |
| `services.toml` | `integrations/services_config.rs` | Service URLs, placeholders, allowed_* filters |

**Overlay:** `app::config_path` checks `{BRAGFROG_CUSTOM_DIR}/{file}` first and falls back to `{BRAGFROG_CONFIG_DIR}/{file}`. The custom directory is gitignored. Only `tokens.css` from it is ever served over HTTP (`/custom/tokens.css`), because the TOML files contain AI prompts. `GET /api/v1/config` exposes the parts the web app needs.

See [customization.md](customization.md) for the full customization guide.

## AI Integration

### Per-User Client
There is no global AI client. Each user configures their own Anthropic API key via Settings, AI tab (stored as the `claude` integration). `AiClient` is instantiated per-request with the user's decrypted key and the model from `BRAGFROG_AI_MODEL` (default: `claude-opus-4-7`).

### Prompt Building (`ai/prompts.rs`)
`build_self_reflection_prompt()` assembles a prompt for one self-review section (`POST /cycles/{id}/review/{slug}/draft`):

1. **Cycle context**: entry statistics, department goals with priorities, entries grouped by priority, unlinked entries
2. **Career level context** (optional): current level expectations, and if targeting promotion, next-level expectations
3. **Section instruction**: loaded from `review_sections.toml`, with optional promotion addendum

The same module builds the meeting prep prompt (`POST /meetings/{entry_id}/prep/draft`) and the report narrative prompt (`build_report_prompt`, used when creating a report).

### Review Sections
Sections are defined in `apps/api/config/review_sections.toml` and loaded at startup into `OnceLock<ReviewConfig>`. Access via `summary::section_slugs()`, `section_title()`, `section_question()`, `get_section()`. The web app reads `has_ai` from `GET /api/v1/me` to show or hide AI buttons. The review editor saves a section with `PUT /cycles/{id}/review/{slug}`.

## Crate Structure

`lib.rs` + `main.rs` dual-crate setup in `apps/api`:
- `lib.rs` declares all modules and exports `AppState`
- `main.rs` is the binary entry point that imports from `brag_frog::`
- This allows integration tests in `apps/api/tests/` to import the library directly
