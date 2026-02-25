# CLAUDE.md — Brag Frog

## What is Brag Frog?

Daily workflow and check-in tool for engineering teams. Log work across GitHub, Phabricator, Bugzilla, Jira, Confluence, Google Drive/Calendar. Auto-prep 1:1s, set OKRs, do weekly check-ins, build impact stories, generate AI-powered self-review drafts.

**Tech stack:** Rust (Axum 0.8) + SQLite (sqlx) + Tera templates + HTMX + vanilla JS. No build step. SSR with HTMX for interactivity.

## Quick Start

```bash
cp .env.example .env   # fill in GOOGLE_CLIENT_ID, GOOGLE_CLIENT_SECRET, ENCRYPTION_KEY
cargo run              # http://localhost:8080
cargo test             # in-memory SQLite, no external deps
cargo clippy           # must pass before committing
```

## Project Structure

`src/`: `app.rs` (router + state + route assembly), `lib.rs` (module declarations), `main.rs` (entrypoint). Modules: `ai/`, `cycle/` (phases, weeks, dashboard, summaries, check-ins), `db/`, `identity/`, `integrations/` (external service sync), `kernel/` (config, crypto, errors, middleware), `objectives/` (priorities, department goals), `worklog/` (entries, entry types).

## Key Conventions

### Route Handlers
- Full pages: `Result<Html<String>, AppError>` → `state.templates.render("pages/xxx.html", &ctx)`
- HTMX partials: same return type → `"components/xxx.html"`, panels → `"panels/xxx.html"`
- `AuthUser` extractor must appear **before** `State(state)` in handler params
- Context always includes `user`, `current_page`, usually `phase`

### HTMX Patterns
- `hx-boost="true"` on `<body>` intercepts all navigation and form submissions
- Entry CRUD returns HTML fragments for `hx-swap="outerHTML"`, deletes return empty string
- Auth failures on HTMX requests use `hx-redirect` header (not HTML redirect)
- CSRF protection via `HX-Request` header check on state-changing requests

### Database
- SQLite WAL mode, foreign keys enabled, max 5 connections
- Migrations: `001_initial.sql` (full schema) + incremental migrations tracked in `_migrations` table
- New migrations: add SQL file to `migrations/`, register in `INCREMENTAL_MIGRATIONS` array in `src/db/mod.rs`
- All dates: `TEXT` in `YYYY-MM-DD` format. Dedup: `UNIQUE(source, source_id)` + upsert
- Runtime SQL only (`query`/`query_as`), not `query!` — SQL errors surface at runtime

## Gotchas

1. **Tera templates loaded at startup.** Paths in `render()` must match relative to `templates/`.
2. **Week creation is implicit.** Visiting dashboard/logbook auto-creates the Week record for the current ISO week.
3. **One active phase per user.** Creating/activating a phase deactivates the previous one. No active phase → renders `pages/no_phase.html`.
4. **Phase deletion cascades aggressively.** Removes all weeks, entries, goals, key results, summaries, initiatives, checkins, impact stories, and AI documents.
5. **Entry handlers return HTML**, not redirects. `update_entry`, `create_entry`, `view_entry` return rendered fragments for HTMX swap.
6. **Empty string form fields.** `deserialize_optional_i64` and `deserialize_optional_string` in `serde_helpers.rs` convert `""` → `None`.
7. **`hx-boost` on body** means all `<a>` and `<form>` are intercepted by HTMX. Plan accordingly for non-HTMX links.
8. **Summary sections are config-driven.** Defined in `review_sections.toml`, accessed via `summary::section_slugs()`, `section_title()`, `section_question()`.
9. **AI client is per-user.** No global client. Users provide their own Anthropic API key. Templates check `has_ai` to show/hide AI buttons.
10. **`lib.rs` + `main.rs` dual-crate.** `lib.rs` declares modules + `AppState`. `main.rs` imports from `brag_frog::`. Integration tests import the library directly.
11. **`async_trait` on `SyncService`** — required for object safety with `Box<dyn SyncService>`. Kept intentionally.
12. **CSS design tokens** in `static/css/tokens.css`. Primary color: Coral Red `#FF453F`. Light theme (`#F0EBE3` bg, `#000` text, `#FAF0E6` secondary bg). Load order: `tokens.css` → `main.css` → `components/*.css`.
13. **Visual style changes must be applied everywhere.** Three CSS surfaces: website (`website/css/`), sign-in page (`.login-page` in `static/css/main.css`), and app pages (`static/css/`).
14. **Design styles and fonts are fixed.** Keep colours and font styles. Can use different shades of existing colours, add one or two tones between fixed ones.
15. **Meeting rules apply only to `recurring_group` matches in SQL.** `title_contains` requires post-decryption filtering since titles are AES-256-GCM encrypted BLOBs.
16. **Encrypted fields in brag_entries.** Title, description, outcome_statement, decision_* fields are encrypted. Cannot do SQL LIKE — filter in application code after decryption.

## Style Guide

1. Never mention Co-Authored when committing code for the developer
2. Always write comments and documentation as a Staff Engineer, intended for other developers

## Further Reading

- [docs/architecture.md](docs/architecture.md) — module map, request lifecycle, database design, sync architecture, AI integration
- [docs/conventions.md](docs/conventions.md) — entry system, data model, KR measurement, panel system, navigation, config system
- [docs/getting-started.md](docs/getting-started.md) — developer setup, running tests, common dev tasks
- [docs/customization.md](docs/customization.md) — config overlays, career levels, review sections, branding
- [docs/self-hosting.md](docs/self-hosting.md) — Docker, bare metal, Fly.io, env vars reference
