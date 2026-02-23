# CLAUDE.md — Brag Frog

## What is Brag Frog?

Daily workflow and check-in tool for engineering teams. Log your work across GitHub, Phabricator, Bugzilla, Jira, Confluence, Google Drive/Calendar. Auto-prep 1:1s, set measurable OKRs, do weekly check-ins, build impact stories, and generate AI-powered self-review drafts.

**Tech stack:** Rust (Axum 0.8) + SQLite (sqlx) + Tera templates + HTMX + vanilla JS. No build step. SSR with HTMX for interactivity. Slide-over panel UX.

## Quick Start

```bash
cp .env.example .env   # fill in GOOGLE_CLIENT_ID, GOOGLE_CLIENT_SECRET, ENCRYPTION_KEY
cargo run              # http://localhost:8080
cargo test             # in-memory SQLite, no external deps
```

## Key Conventions

### Route Handlers
- Full pages: `Result<Html<String>, AppError>` → `state.templates.render("pages/xxx.html", &ctx)`
- HTMX partials: same return type → `"components/xxx.html"`
- Panel content: `"panels/xxx.html"` — loaded via `openPanelFromUrl()` JS helper
- `AuthUser` extractor must appear **before** `State(state)` in handler params
- Context always includes `user`, `current_page`, usually `phase`

### HTMX Patterns
- `hx-boost="true"` on `<body>` intercepts all navigation and form submissions
- Entry CRUD returns HTML fragments for `hx-swap="outerHTML"`, deletes return empty string
- Auth failures on HTMX requests use `hx-redirect` header (not HTML redirect)
- CSRF protection via `HX-Request` header check on state-changing requests
- Panel content loaded via `htmx.ajax('GET', url, {target: '#panel-body'})` from JS

### Database
- SQLite WAL mode, foreign keys enabled, max 5 connections
- Migrations: `001_initial.sql` (full schema, fresh DB) + incremental migrations tracked in `_migrations` table
- New migrations: add SQL file to `migrations/`, register in `INCREMENTAL_MIGRATIONS` array in `src/db/mod.rs`
- All dates: `TEXT` in `YYYY-MM-DD` format. Dedup: `UNIQUE(source, source_id)` + upsert
- Runtime SQL only (`query`/`query_as`), not `query!` — SQL errors surface at runtime

### Entry System
- Sources: `github`, `phabricator`, `bugzilla`, `jira`, `confluence`, `google_drive`, `google_calendar`, `manual`
- 35 entry types (see `EntryType::ALL` in `src/models/entry.rs`)
- Entries belong to a Week (which belongs to a Phase)
- Impact fields: `outcome_statement`, `evidence_urls`, `role`, `impact_tags`, `reach`, `complexity`
- Decision fields: `decision_alternatives`, `decision_reasoning`, `decision_outcome`
- Meeting fields: `meeting_role`, `recurring_group`

### Data Model
Hierarchy: **Goal → KeyResult → Entry** (via `key_result_id` → `goal_id` FKs). **Initiative → Entry** (via `initiative_id`). **Initiative ↔ KeyResult** (many-to-many via `initiative_key_results`).

Additional tables: `initiatives`, `initiative_key_results`, `weekly_checkins`, `kr_checkin_snapshots`, `impact_stories`, `story_entries`, `ai_documents`, `meeting_rules`, `entry_competencies`, `weekly_focus`, `weekly_focus_items`, `meeting_prep_notes`.

### KR Measurement System
- `kr_type`: `manual` (default), `numeric`, `boolean`, `milestone`
- `direction`: `increase`, `decrease`, `maintain` (for numeric)
- Score: auto-calculated 0.0–1.0 via `KeyResult::recalculate_score()`
- `progress` field kept in sync: `progress = (score * 100).round()`

### Panel System
- Slide-over panels for entry detail, initiative detail, KR updates
- JS: `openPanel(title, html)`, `openPanelFromUrl(title, url)`, `closePanel()`
- Escape key closes panel. Scrim click closes panel.
- Panel content loaded into `#panel-body` div

### Navigation
- Primary nav: Dashboard, Logbook, Goals, Prep, Review
- Secondary (avatar dropdown): Settings, Integrations, Level Guide, Trends, Export, Sync All, Sign Out
- `GET /` → redirects to `/dashboard`
- `/goals` is the OKR management page (goals, key results, initiatives, phase management)
- `/meeting-prep` is the meeting preparation page (notes per meeting)
- `/checkins` is the check-in history page
- `/integrations` is the service connections page (split from settings)
- `/settings` is the profile and preferences page
- `/review/:id` is the self-review page
- `GET /phases` route removed (old page merged into `/goals`)

### Config System
Three TOML files loaded at startup into `OnceLock` statics: `clg_levels.toml`, `review_sections.toml`, `services.toml`. App checks `custom/` first, falls back to `config/`. The `custom/` directory is gitignored.

## Gotchas

1. **Tera templates loaded at startup.** Paths in `render()` must match relative to `templates/` (e.g., `"pages/logbook.html"`).

2. **Week creation is implicit.** Visiting the dashboard/logbook auto-creates the Week record for the current ISO week.

3. **One active phase per user.** Creating/activating a phase deactivates the previous one. No active phase → renders `pages/no_phase.html`.

4. **Phase deletion cascades aggressively.** Removes all weeks, entries, goals, key results, summaries, initiatives, checkins, impact stories, and AI documents.

5. **Entry handlers return HTML**, not redirects. `update_entry`, `create_entry`, `view_entry` all return rendered fragments for HTMX swap.

6. **Empty string form fields.** `deserialize_optional_i64` and `deserialize_optional_string` in `serde_helpers.rs` convert `""` → `None`.

7. **`hx-boost` on body** means all `<a>` and `<form>` are intercepted by HTMX. Plan accordingly for non-HTMX links.

8. **Summary sections are config-driven.** Defined in `review_sections.toml`, accessed via `summary::section_slugs()`, `section_title()`, `section_question()`, `get_section()`.

9. **AI client is per-user.** No global client. Users provide their own Anthropic API key. Templates check `has_ai` to show/hide AI buttons. Model configurable via `BRAGFROG_AI_MODEL`.

10. **`lib.rs` + `main.rs` dual-crate.** `lib.rs` declares modules + `AppState`. `main.rs` imports from `brag_frog::`. Integration tests import the library directly.

11. **`async_trait` on `SyncService`** — required for object safety with `Box<dyn SyncService>`. Kept intentionally.

12. **CSS design tokens** in `static/css/tokens.css`. Primary color: Coral Red `#FF453F` (Mozilla New Products spot color). Light theme (`#F0EBE3` bg, `#000` text, `#FAF0E6` secondary bg). Load order: `tokens.css` → `main.css` → `components/*.css`.

13. **Visual style changes must be applied everywhere.** Three separate CSS surfaces exist: the website (`website/css/`), the sign-in page (`.login-page` in `static/css/main.css`), and the app pages (`static/css/`). When updating backgrounds, textures, colors, or other visual styles, always update all three.

14. **Design styles and fonts are fixed**. When redesigning the website and app, we must keep the colours and the font styles. We can play around with different shades of the existing colours, and add maybe one or two tones between the fixed ones.

15. **Meeting rules apply only to `recurring_group` matches in SQL.** The `title_contains` match type requires post-decryption application in app code since titles are encrypted.

16. **Encrypted fields in brag_entries.** All title, description, outcome_statement, decision_* fields are AES-256-GCM encrypted BLOBs. Cannot do SQL LIKE on them — filter in application code after decryption.

## Further Reading

- [docs/architecture.md](docs/architecture.md) — module map, request lifecycle, database design, sync architecture, AI integration
- [docs/getting-started.md](docs/getting-started.md) — developer setup, running tests, common dev tasks
- [docs/customization.md](docs/customization.md) — config overlays, career levels, review sections, branding
- [docs/self-hosting.md](docs/self-hosting.md) — Docker, bare metal, Fly.io, env vars reference
