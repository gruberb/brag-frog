# CLAUDE.md: Brag Frog

## What is Brag Frog?

Daily workflow and check-in tool for engineering teams. Log work across GitHub, Phabricator, Bugzilla, Jira, Confluence, Google Drive/Calendar. Auto-prep 1:1s, set priorities, build saved reports, generate AI-powered self-review drafts.

**Tech stack:** moon monorepo. `apps/api`: Rust (Axum 0.8) + SQLite (sqlx), JSON API under `/api/v1`. `apps/web`: React 19 + TypeScript + Vite + React Router 7 + TanStack Query + Tailwind v4 + BlockNote editor. The API serves the built web app in production.

## Quick Start

```bash
cp .env.example .env     # fill in BRAGFROG_GOOGLE_CLIENT_ID, BRAGFROG_GOOGLE_CLIENT_SECRET, BRAGFROG_ENCRYPTION_KEY
npm ci                   # installs the web workspace (Node 24, pinned in .prototools)
moon run api:dev         # API on http://localhost:8080 (runs from apps/api)
moon run web:dev         # Vite on http://localhost:5173, proxies /api and /auth to :8080
moon run :lint :test :build   # everything CI runs (see .github/workflows/ci.yml, `moon ci`)
moon run api:types       # regenerate TypeScript types after changing an exported Rust DTO
```

Single projects: `moon run api:test`, `moon run api:lint` (clippy `-D warnings`, must pass before committing), `moon run web:lint`, `moon run web:typecheck`, `moon run web:test`.

## Project Structure

```
apps/api/        Rust crate `brag-frog` (lib + bin), config/, migrations/, tests/
apps/web/        React app, see apps/web/README.md
.moon/           moon workspace + toolchains; moon.yml in each app defines tasks
Cargo.toml       Cargo workspace (members = apps/api)
package.json     npm workspace (members = apps/web)
custom/          operator overrides (gitignored)
website/         landing page (separate from the app)
```

`apps/api/src/`: `app.rs` (state, router, web app serving), `lib.rs` (modules + `AppState`), `main.rs`. Modules: `api/` (JSON handlers, DTOs, one file per area), `ai/` (client, prompts), `cycle/` (cycles, weeks, meeting prep, trends, logbook insights), `reflections/` (legacy check-in data), `review/` (summaries, reports, AI documents), `db/`, `identity/` (OAuth redirects, users, career levels), `integrations/` (external service sync), `kernel/` (config, crypto, errors, middleware), `objectives/` (priorities, department goals), `worklog/` (entries, entry types).

`apps/web/src/`: `app/` (router, routes, provider), `components/` (ui, layout, editor), `features/` (auth, cycles, entries, integrations, logbook, priorities, review, settings, today, trends), `hooks/ lib/ stores/ utils/ config/ testing/`, `types/api/` (generated).

## Key Conventions

### API handlers
- Return `ApiResult<T>` (`Result<Json<T>, ApiError>`). Errors serialize as `{"error":{"code","message"}}`.
- Use the `ApiAuth` extractor (`api/auth.rs`) before `State(state)` in handler params.
- Handlers stay thin: reuse repo and service code of the module, translate to DTOs in `api/dto.rs`.
- Register routes in `api::router()` (`apps/api/src/api/mod.rs`). They sit behind `csrf_protection` and `require_auth_api`.
- Only `/auth/*` (OAuth redirects, `identity/routes.rs`), `/healthz` and `/api/v1/public-config` live outside the session check.
- CSRF: every state-changing request must send `X-Requested-With: brag-frog` (the web client does it via `lib/api-client.ts`, header name in `config/env.ts`).

### Generated types
- DTOs marked `#[derive(TS)] #[ts(export)]` are exported to `apps/web/src/types/api/` by `moon run api:types`. Never edit those files by hand.
- `api:types-check` fails if the generated files differ from what is committed.

### Web app
- Dependency flow is one way: shared (`components hooks lib stores types utils config`) then `features` then `app`. A feature never imports another feature; compose features in `src/app/routes`. ESLint (`import/no-restricted-paths`) enforces it.
- Server state goes through TanStack Query, query keys live next to the feature's `api/` code. No barrel files.
- Route paths come from `src/config/paths.ts`.

### Database
- SQLite WAL mode, foreign keys enabled, max 5 connections
- Migrations: `001_initial.sql` (full schema) + incremental migrations tracked in `_migrations`. Latest: `025_reports.sql`.
- New migrations: add the SQL file to `apps/api/migrations/`, register it in `INCREMENTAL_MIGRATIONS` in `apps/api/src/db/mod.rs`
- All dates: `TEXT` in `YYYY-MM-DD` format. Dedup: `UNIQUE(source, source_id)` + upsert
- Runtime SQL only (`query`/`query_as`), not `query!`: SQL errors surface at runtime

### Config and environment
- `BRAGFROG_CONFIG_DIR` (default `config`), `BRAGFROG_CUSTOM_DIR` (default `custom`), `BRAGFROG_WEB_DIST` (default `../web/dist`), all relative to the working directory (`apps/api` under moon).
- TOML overlay: `custom/{file}` wins over `config/{file}` for `clg_levels.toml`, `review_sections.toml`, `services.toml`.

## Gotchas

1. **Week creation is implicit.** Creating an entry or opening the Today view auto-creates the Week record for the current ISO week.
2. **One active cycle per user.** Creating/activating a cycle deactivates the previous one. `POST /cycles` creates an inactive cycle; activation is a separate call.
3. **Cycle deletion cascades aggressively.** Removes all weeks, entries, priorities, summaries, reports and AI documents of that cycle.
4. **Removed features keep their data.** Quarterly check-ins, contribution examples, status updates and last-week reports have no UI or write path. Old rows stay in the database and are returned read-only by `GET /api/v1/cycles/{id}/archive`.
5. **Summary sections are config-driven.** Defined in `review_sections.toml`, accessed via `summary::section_slugs()`, `section_title()`, `section_question()`.
6. **AI client is per-user.** No global client. Users provide their own Anthropic API key (stored as the `claude` integration). `has_ai` in `GET /api/v1/me` shows or hides AI buttons.
7. **`lib.rs` + `main.rs` dual-crate.** `lib.rs` declares modules + `AppState`. `main.rs` imports from `brag_frog::`. Integration tests in `apps/api/tests/integration` import the library directly.
8. **`async_trait` on `SyncService`** is required for object safety with `Box<dyn SyncService>`. Kept intentionally.
9. **Web app is served from `BRAGFROG_WEB_DIST`.** Unknown paths return `index.html` (client-side routes), unknown `/api/*` paths return a JSON 404. Without a built `dist/` the API runs but serves no UI; use `moon run web:dev` for development.
10. **`/custom` serves only `tokens.css`.** The custom dir also holds operator config including AI prompts and must never be a static file tree. A missing `tokens.css` returns an empty stylesheet.
11. **Design tokens** live in `apps/web/src/app/app.css` (Tailwind `@theme static`). Primary color: Coral Red `#FF453F`. `custom/tokens.css` is linked after the app styles and overrides them. Mozilla Headline and Mozilla Text fonts are in `apps/web/public/fonts`.
12. **Visual style changes must be applied everywhere.** Two CSS surfaces: the web app (`apps/web/src/app/app.css`, which includes the sign-in page) and the landing page (`website/css/`).
13. **Design styles and fonts are fixed.** Keep colours and font styles. Can use different shades of existing colours, add one or two tones between fixed ones. `apps/web/src/app/shell.test.tsx` fails on other `font-family` values.
14. **Meeting rules apply only to `recurring_group` matches in SQL.** `title_contains` requires post-decryption filtering since titles are AES-256-GCM encrypted BLOBs.
15. **Encrypted fields in brag_entries.** Title, description, outcome_statement, decision_* fields are encrypted. Cannot do SQL LIKE: filter in application code after decryption.
16. **Reports are snapshots.** `reports` stores filters, a stats snapshot and optional AI text. There is no edit endpoint; regenerate by creating a new report.

## Style Guide

1. Never mention Co-Authored when committing code for the developer
2. Always write comments and documentation as a Staff Engineer, intended for other developers
3. Never just remove UI or just handlers, refactor, delete and add features end-to-end (.env, db, handlers, logic, UI etc.)

## Further Reading

- [docs/architecture.md](docs/architecture.md): module map, request lifecycle, database design, sync architecture, web app, AI integration
- [docs/conventions.md](docs/conventions.md): entry system, data model, API and web conventions, config system
- [docs/getting-started.md](docs/getting-started.md): developer setup, running tests, common dev tasks
- [docs/customization.md](docs/customization.md): config overlays, career levels, review sections, branding
- [docs/self-hosting.md](docs/self-hosting.md): Docker, bare metal, Fly.io, env vars reference
- [apps/web/README.md](apps/web/README.md): web app structure and API mapping
