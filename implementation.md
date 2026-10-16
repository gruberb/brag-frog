# Brag Frog redesign: implementation plan

Source: RFD "Redesign Brag Frog" (2026-10-16). Mocks: [`mocks/index.html`](mocks/index.html).
Audience: a lower-tier model or contributor executing milestone by milestone. Follow the order. Each milestone is one PR and ends with acceptance checks that must pass before the next one starts.

## 0. Summary of decisions

| Topic | Decision |
|---|---|
| Repo | Monorepo in `brag-frog`, managed by **moon v2** (+ proto for toolchains). Cargo workspace + npm workspace. |
| Layout | `apps/api` (existing Rust crate, moved) and `apps/web` (new React SPA). |
| Frontend | React 19, TypeScript, Vite, React Router 7, TanStack Query 5, Tailwind v4 (`@theme`), Radix primitives, react-hook-form + zod, BlockNote (lazy, editor routes only). Structure follows [bulletproof-react](https://github.com/alan2207/bulletproof-react). |
| API | JSON under `/api/v1`. Same session cookie, same origin, no CORS. Types generated from Rust with `ts-rs`. |
| Serving | Axum serves `apps/web/dist` plus the API from one container. |
| Removed features | the old Reports tabs (status update, last-week report), quarterly check-ins, contribution examples, level guide page, review guide page. **Reports is a button in the Logbook** that turns the current filters into a saved, downloadable report (no Reports nav item). Their tables stay in the DB (read-only archive), only routes and UI go. |
| Kept | Meeting prep (Today shows "prep ready"), integrations and sync, AI drafts (non-streaming first), people aliases, Lattice import, `custom/` overlay. |
| Deploy | Stay on **Cloud Run**. `brag-frog-mozilla` builds `FROM` a pinned upstream image instead of copying `templates/` and `static/`. |
| Backend rule | Backend logic does not change except: delete HTML routes, add JSON routes on top of the existing repo/service layer, fix the performance problems listed in section 4. |

Deviation from the first plan draft: legacy tables are **not** migrated or dropped. They stay and are read through an archive endpoint. This is lower risk and keeps rollback to v5.x possible because all schema changes are additive.

## 1. Conventions for the executor

- Read `CLAUDE.md`, `docs/architecture.md`, `docs/conventions.md` before touching code.
- Commit messages: conventional style. **Never add `Co-Authored-By` or AI attribution** (project rule). Ticket ids go in commit messages and PR descriptions only.
- Comments explain why, not what. No ticket ids in code.
- Never remove only UI or only handlers: when removing a feature, remove route + repo + model + config + UI + tests together (project style guide #3). Exception: legacy tables, see above.
- Colours and fonts are fixed (CLAUDE.md gotchas 12 to 14). Use the existing token values. You may add shades between existing tones.
- After each milestone: `moon run :lint :test :build` (from M0 on), `cargo clippy --workspace -- -D warnings`, update `CHANGELOG.md` `[Unreleased]`.
- If a step says "verify", run the command and paste the result into the PR description. Do not assume.
- Do not touch `*.db*` files in the repo root. They are local data (gitignored).

## 2. Target layout

```
brag-frog/
  .moon/workspace.yml        # projects: api, web
  .moon/toolchains.yml       # rust + node (keys: verify with `moon init`, v2 uses toolchains.yml)
  .prototools                # moon, node = "24", rust = "1.93.0"
  Cargo.toml                 # [workspace] members = ["apps/api"], resolver = "3"
  Cargo.lock
  package.json               # { "private": true, "workspaces": ["apps/web"], "engines": {"node": ">=24"} }
  package-lock.json
  moon.yml                   # root tasks (docker, types:check)
  Dockerfile                 # multi-stage: node -> rust -> debian
  docker-compose.yml
  apps/
    api/                     # git mv of: src/ migrations/ config/ tests/ Cargo.toml(crate)
      moon.yml
    web/
      moon.yml  package.json  vite.config.ts  tsconfig*.json  eslint.config.js  index.html
      public/fonts/          # copy of static/fonts (Mozilla Headline/Text)
      src/
        app/                 # main.tsx, app.tsx, provider.tsx, router.tsx, routes/*.tsx
        components/          # ui/, layout/, editor/ (BlockNote wrapper), evidence-chip
        config/              # env.ts, paths.ts
        features/            # auth cycles today logbook priorities review trends settings entries
        hooks/  lib/  stores/  testing/  types/  utils/
  docs/  mocks/  website/    # website/ (marketing, Vercel) is untouched
```

Rules (enforced by ESLint `import/no-restricted-paths`, M3):
- Flow is one way: shared (`components hooks lib stores types utils config`) then `features` then `app`.
- A feature never imports another feature. Compose features in `src/app/routes/*` (pass slots as props, for example the evidence rail into the review page).
- No barrel files, import files directly.
- Generated API types live in `src/types/api/` and are never edited by hand.

## 3. Milestone map

```
M0 monorepo ──► M1 API foundation ──► M2 domain endpoints (a..e) ─────────────┐
      │                  │                                                      ├─► M5 cutover ─► M6 release/CI ─► M7 deploy (mozilla) ─► M8 optional perf
      └──► M3 web foundation (needs M1 types) ──► M4 features (needs matching M2 part)┘
```
After M1 and M3, tracks A (M2) and B (M4) can run in parallel per domain. Each M4 feature is built against MSW handlers first, then switched to the real API when its M2 part has merged.

Measure first (M0 step 5): do not skip. It tells you how much of the slowness is SQLite on GCS FUSE versus SSR/decryption.

---

## M0. Monorepo skeleton (no behaviour change)

Goal: same app, new layout, builds in CI.

1. Install tools: `proto install` (moon, node 24, rust 1.93.0 via `.prototools`). Moon is already on this machine via proto.
2. Move the crate with history:
   ```bash
   mkdir -p apps/api apps/web
   git mv src migrations config tests apps/api/
   git mv Cargo.toml apps/api/Cargo.toml
   ```
   Create root `Cargo.toml`:
   ```toml
   [workspace]
   members = ["apps/api"]
   resolver = "3"
   ```
   Keep `Cargo.lock` at the root (`cargo build` regenerates the path if needed).
3. Fix paths that assumed the repo root. These resolve relative to the process working directory or at compile time:
   - `include_str!` of migrations in `apps/api/src/db/mod.rs` (relative to the source file, so `../../migrations/...` usually still works because the move is consistent: verify by compiling).
   - Runtime dirs in `apps/api/src/app.rs`: `config_path()` (line ~38), `ServeDir::new("static")`, `ServeDir::new("custom")`, Tera glob `templates/**/*.html` (line ~71).
   - Done differently than first written: `templates/` and `static/` also move into `apps/api/` in M0 (so the process working directory `apps/api` keeps resolving `config/`, `templates/`, `static/` unchanged) and are deleted from there in M5. `custom/` stays gitignored at the repo root locally; symlink it as `apps/api/custom` or set `BRAGFROG_CUSTOM_DIR`.
   - Make them configurable with defaults equal to today's values: add to `kernel/config.rs` `config_dir` (`BRAGFROG_CONFIG_DIR`, default `config`), `custom_dir` (`BRAGFROG_CUSTOM_DIR`, default `custom`), `web_dist` (`BRAGFROG_WEB_DIST`, default `apps/web/dist`). Keep `templates/` and `static/` at repo root until M5 (they are deleted there).
   - `tests/integration/common.rs` loads configs from `config/`: point at `apps/api/config` using `env!("CARGO_MANIFEST_DIR")`.
4. moon config. Create `.moon/workspace.yml` with `projects: { api: apps/api, web: apps/web }`, `.moon/toolchains.yml` enabling rust and node/npm. Moon v2 key names: run `moon init` in a scratch dir, or read https://moonrepo.dev/docs/guides/rust/handbook, and copy exact keys. Do not guess.
   `apps/api/moon.yml`:
   ```yaml
   tasks:
     build: { command: 'cargo build --release -p brag-frog' }
     lint:  { command: 'cargo clippy -p brag-frog --all-targets -- -D warnings' }
     test:  { command: 'cargo test -p brag-frog' }
     dev:   { command: 'cargo run -p brag-frog', local: true }
   ```
   Do not cache `target/` as a moon output (too large; moon handbook warns against it).
   `apps/web/moon.yml` is added in M3.
5. **Baseline measurement.** Record in `docs/perf-baseline.md` (new file) the p50/p95 of these routes, local and on the current Cloud Run deployment (`curl -s -o /dev/null -w '%{time_total}\n'` x 20, with a session cookie): `/dashboard`, `/logbook`, `/logbook/entries?search=x`, `/trends`, `/review/{phase}`. Also note entry count. Re-measure at M5 and M7.
6. `Dockerfile`: update COPY paths (`apps/api/...`) only. Build must still pass: `docker build .`.
7. `.github/workflows/ci.yml` (new): checkout, `proto` setup (use `moonrepo/setup-toolchain@v0`), `moon ci`. Cache cargo registry.
8. Quick security fix (separate small PR, do it now): `ServeDir::new("custom")` is mounted outside auth and exposes `custom/*.toml` including AI prompts. Replace with a single explicit route for `/custom/tokens.css` (existing handler `custom_tokens_css`). Add a test: `GET /custom/review_sections.toml` returns 404 when unauthenticated.

Acceptance: `moon run api:lint api:test api:build` green; `docker build .` green; app runs with `moon run api:dev` and `/dashboard` renders; baseline doc committed.

---

## M1. API foundation

Goal: everything the JSON endpoints need, with no domain endpoints yet except `/api/v1/me`, `/api/v1/public-config`, `/healthz`.

Files (all new under `apps/api/src/api/`): `mod.rs` (router `api::router(state) -> Router<AppState>`), `error.rs`, `middleware.rs`, `dto/` (one file per domain, added in M2).

1. **Error type.** `ApiError(AppError)` implementing `IntoResponse` with body `{"error":{"code":"not_found","message":"..."}}` and the same status codes as `AppError` (`kernel/error.rs:35-70`). Map: Unauthorized 401 `unauthorized`, NotFound 404 `not_found`, validation 422 `invalid_input`, forbidden 403 `forbidden`, other 500 `internal` (log the cause, return a generic message). Handlers return `Result<Json<T>, ApiError>`. Old HTML handlers keep `AppError` until M5.
2. **Auth middleware.** `require_auth_api`: no session gives 401 JSON `{"error":{"code":"unauthorized"...}}` (never a 302). Reuse `AuthUser` extractor (`identity/auth/middleware.rs:76-97`), unchanged.
3. **CSRF.** Extend `csrf_protection` (`kernel/middleware.rs:33-55`): for paths starting `/api/` require header `X-Requested-With: brag-frog` on non-GET/HEAD/OPTIONS. Keep the `HX-Request` rule for legacy paths until M5. No CORS layer is added, so cross-origin pages cannot send the custom header (preflight fails). Also reject requests whose `Sec-Fetch-Site` is `cross-site` when present.
4. **Routes.**
   - `GET /healthz` (unauthenticated, 200 `ok`, no DB). Used by Cloud Run startup probe.
   - `GET /api/v1/public-config` (unauthenticated): `{instance_name, allowed_domain_hint, public_only}`.
   - `GET /api/v1/me`: `{user, active_cycle, has_ai, public_only}`. `has_ai` computed once here (replaces the per-page `has_ai_for_user` query).
   - `POST /api/v1/auth/logout`: 204 and clears the session. `GET /auth/login` and `GET /auth/callback` stay as server redirects (OAuth must stay server-side). The callback's final redirect target changes from `/` to `/today`.
5. **Types generation.** Add dev-dependency/dependency `ts-rs` (feature `chrono-impl`). Derive `#[derive(TS)] #[ts(export, export_to = "../../web/src/types/api/")]` on every DTO. Add task `api:types` running `cargo test -p brag-frog export_bindings` (ts-rs exports during tests) and root task `types:check` that runs it and `git diff --exit-code apps/web/src/types/api`. CI runs `types:check`.
6. **Performance primitives** (used by M2):
   - `spawn_blocking` helper `decrypt_many(crypto, rows)` for large lists (AES-GCM is CPU work, currently inline in async handlers). If `UserCrypto` is not `Clone`, wrap in `Arc`.
   - Projection `BragEntrySummary` (title, teams, collaborators, no description/decision fields). List endpoints decrypt only these. Detail decrypts everything.
   - Facets cache: `DashMap<i64, (Instant, Arc<Facets>)>` in `AppState` (teams, collaborators), TTL 60 s, invalidated on entry write and sync completion. Replaces `distinct_teams_for_phase` / `distinct_collaborators_for_phase` running per page (`worklog/repo.rs:446-495`).
   - Migration `024_perf_and_content_format.sql` (add to `INCREMENTAL_MIGRATIONS` in `db/mod.rs`): `CREATE INDEX IF NOT EXISTS idx_entries_week_occurred ON brag_entries(week_id, occurred_at) WHERE deleted_at IS NULL;`, `ALTER TABLE summaries ADD COLUMN content_format TEXT NOT NULL DEFAULT 'markdown';`. Additive only.
   - Migration `025_reports.sql`: `reports(id INTEGER PK, user_id, phase_id REFERENCES brag_phases ON DELETE CASCADE, title BLOB(enc), filters_json TEXT /*the exact Logbook filter set used, same keys as GET /entries*/, content BLOB(enc) /*BlockNote JSON of the narrative*/, content_format TEXT DEFAULT 'blocknote', stats_json BLOB(enc) /*snapshot of numbers at generation time*/, model_used TEXT, generated_at TEXT, updated_at TEXT)` plus index `(user_id, phase_id, generated_at DESC)`. Add `DELETE FROM reports WHERE phase_id = ?` to the phase cascade (`cycle/repo/mod.rs:163`). Stats are stored as a snapshot so an old report does not change when entries are edited later.
7. Tests: JSON contract helper in `tests/integration/common.rs` (`get_json`, `post_json`, adds `X-Requested-With`). Tests: 401 JSON when logged out, CSRF header required, `/me` shape, `/healthz`.

Acceptance: `moon run api:test` green with the new tests; `GET /api/v1/me` returns 401 JSON without a cookie and the user with one; `types:check` passes.

---

## M2. Domain endpoints

Each sub-milestone adds handlers under `apps/api/src/api/<domain>.rs`, DTOs with `TS` derive, and JSON contract tests. Reuse the existing repo and service functions listed; do not duplicate SQL. The HTML routes keep working until M5.

All paths are under `/api/v1`. JSON in, JSON out. Dates are `YYYY-MM-DD` strings. Errors use the M1 shape.

### M2a. Cycles, settings, config, people

| Method + path | Behavior | Reuse |
|---|---|---|
| `GET /cycles` | list user's cycles with entry/priority counts | `BragPhase::list_for_user` |
| `POST /cycles` `{name,start_date,end_date,activate}` | create; activating deactivates the previous (gotcha 3) | `cycle/routes/phases.rs` logic |
| `POST /cycles/{id}/activate` | | same |
| `DELETE /cycles/{id}` `{confirm_name}` | cascade delete (gotcha 4); require exact name | `BragPhase::delete` (`cycle/repo/mod.rs:163`) |
| `GET /cycles/{id}/bundle` | JSON for download: cycle, goals, priorities with updates, weeks with entries, review sections | combine repo calls; one decrypt pass |
| `GET /settings/profile`, `PUT /settings/profile` | JSON body, real booleans (not "on"/absent form values) | `identity/routes.rs:159,210` |
| `GET/POST /settings/people`, `DELETE /settings/people/{id}` | | `PeopleAlias` repo |
| `GET /config` | UI config only: review section titles/questions/`form_*`/`focus_priority*`, services metadata (`token_url`, placeholders, `note`, default orgs), CLG levels (`id,title,one_liner,problems,people`), `instance_name`. **Never** return `prompt`, `prompt_with_clg`, `ai_prompt`, `promotion_addendum`. | `config/*.toml` statics |

### M2b. Entries, today, meeting prep

Entry JSON (`teams` and `collaborators` are arrays in the API, comma-strings in the DB; split on `,` and trim at the boundary):

```ts
type Entry = { id; week_id; source; source_id; source_url; title; description; entry_type; status;
  repository; occurred_at; teams: string[]; collaborators: string[]; outcome_statement; role; reach;
  complexity; priority_id: number|null; meeting_role; recurring_group; start_time; end_time;
  evidence_urls: string[]; impact_tags: string[];
  decision: {alternatives; reasoning; outcome}|null; created_at; updated_at }
```

| Method + path | Behavior | Reuse / note |
|---|---|---|
| `GET /entries?cycle_id&from&to&source&priority_id&goal_id&type&reach&complexity&role&q&team&person&no_priority&no_team&cursor&limit=50` | `{items: EntrySummary[], next_cursor, total_matching}`. SQL filters first, then in-memory filters for encrypted fields on the already-narrowed set. Order `(occurred_at DESC, id DESC)`, cursor encodes that pair. **One** decrypt pass (the SSR logbook does two). | `BragEntry::list_for_phase_filtered` (`worklog/repo.rs:85`), `apply_in_memory_filters` (`cycle/service/logbook_insights.rs:104`), `PeopleAlias::alias_map` |
| `GET /entries/facets?cycle_id` | `{teams, people, sources, counts}` from the cache | M1 facets cache |
| `GET /entries/{id}` | full entry | `view_entry` logic |
| `POST /entries` | create manual entry (replaces `/entries/quick`) | `quick_create_entry` (`worklog/routes.rs:142`) |
| `PUT /entries/{id}`, `DELETE /entries/{id}` | hard delete manual, soft delete synced (unchanged) | `routes.rs:68,87` |
| `POST /entries/bulk` `{ids, patch}` | | `bulk_update_entries` (`:307`) |
| `POST /entries/{id}/classify` | meeting role/teams, optional rule | `classify_entry` (`:237`) |
| `POST /entries/{id}/exclude` | drive file or calendar event exclusion | `:273,290` |
| `GET /today` | `{week, meetings_by_day, prep_map, active_work, priorities (active, with latest update + blocker count), focus_days, counts}` | `cycle/service/dashboard.rs` (`filter_active_work`, `build_meeting_days`, `compute_focus_days`). Do not load teams/collaborators facets here (the dashboard did and decrypted the whole phase). |
| `GET/PUT /meetings/{entry_id}/prep`, `POST .../prep/draft` | notes, doc_urls, meeting_goal, priority; draft returns `{content}` | `cycle/routes/meeting_prep.rs:38,95,182` |

Ownership: every handler that takes an id must verify it belongs to the user through week -> phase -> user_id (`brag_entries` has no `user_id`). Add a helper `BragEntry::find_owned(id, user_id)` and use it everywhere. Test that user B gets 404 for user A's entry, meeting prep and week.

`Week::find_or_create_for_date` is allowed on `GET /today` but must be idempotent (`INSERT ... ON CONFLICT DO NOTHING` then select).

### M2c. Priorities and goals

| Method + path | Reuse |
|---|---|
| `GET /priorities?cycle_id` returns `{goals: [{goal, priorities: [...with latest update, entry_count]}], unassigned: [...], counts}` | `objectives::service::sort_priorities`, `group_by_department_goal` (`objectives/service.rs`) |
| `POST/PUT/DELETE /priorities`, `/priorities/{id}` | `objectives/routes.rs:226,240,251` |
| `POST/PUT/DELETE /goals`, `/goals/{id}` (completing a goal completes its priorities) | `:176,198,213` |
| `GET/POST /priorities/{id}/updates` | `PriorityUpdate` repo, `post_priority_update` (`:261`) |
| `POST /priorities/import` (multipart `csv_file`) returns `{created, updated, skipped, errors[]}` as data, not HTML | `objectives/import.rs`, `import_lattice_csv` (`:295`) |

### M2d. Review

Review documents are stored as BlockNote JSON in the existing encrypted `summaries.content` column, with `content_format = 'blocknote'`. Old rows have `'markdown'`; the client converts on first open (`tryParseMarkdownToBlocks`) and saves back as blocknote.

| Method + path | Behavior | Reuse |
|---|---|---|
| `GET /cycles/{id}/review` | `{sections: [{slug,title,question,form_*,focus_priorities,content,content_format,generated_at}], entry_count}`. All sections from `review_sections.toml`, not only the first (`primary_section_slug` is removed). | `review/routes/summaries.rs:100,350-395`, `Summary::list_for_phase` |
| `PUT /cycles/{id}/review/{slug}` `{content, content_format}` | | `save_section` (`:281`) |
| `POST /cycles/{id}/review/{slug}/draft` `{goal_ids?, entry_ids?}` returns `{markdown}` | `ai_draft_section` (`:234`), `build_self_reflection_prompt` (`ai/prompts.rs:15`) |
| `GET /cycles/{id}/archive` | read-only legacy: quarterly check-ins, contribution examples (+linked entry titles, one query), status updates, last-week reports. | existing models; add one batched `linked_entry_ids_for_phase` query instead of N+1 |

Jira key linkification (`summaries.rs:397-570`) becomes a client concern: render `PROJ-123` as links using the Atlassian base URL from `/config`. Remove the server-side per-save entry reload.

### M2d-2. Reports (from Logbook filters, saved)

A report is the current Logbook filter set turned into a document. There is no separate Reports page.

| Method + path | Behavior | Reuse |
|---|---|---|
| `POST /reports` `{cycle_id, title?, filters: {...same keys as GET /entries (from,to,priority_id(s),goal_id,source,type,q,team,person,...)}, with_ai: bool}` | Runs the same filtered query as `GET /entries` (one decrypt pass, no paging), computes the stats snapshot with the Trends aggregation (`cycle/service/trends.rs`, taking the same `EntryFilter`), picks key entries (top 10 by reach then complexity), optionally builds an AI narrative, saves, returns the full report. Default title: `<priority name or "All projects"> · <range label>`. | `list_for_phase_filtered`, `apply_in_memory_filters`, `ai::prompts` (adapt `build_last_week_summary_prompt` into `build_report_prompt(filters, stats, entries)`), `ai::helpers::get_ai_client` |
| `GET /reports?cycle_id` | list `{id,title,filters,entry_count,generated_at}`, newest first | new `Report` repo |
| `GET /reports/{id}` | `{...meta, stats, narrative: {content, content_format}, key_entries: Entry[]}` (the client renders PDF/DOCX/MD from this) | |
| `DELETE /reports/{id}` | | |

`stats`: `{entries, people, repos, teams, per_week: [{week_start, count}], per_project: [{priority_id,name,count}], people_top: [{name,count}], repos_top, teams_top}`. A refusal rule: if the filtered set is empty, return 422 `empty_selection` (the dialog shows it inline). Ownership is checked on every id. If AI is not configured or fails, the report is still created with an empty narrative (never fail because of AI); the response has `ai_error` so the dialog can say so. The AI call can take up to the existing 120 s timeout, so the dialog shows a progress state. No edit endpoint: reports are snapshots, regenerate by creating a new one.

### M2e. Trends, integrations, sync

| Method + path | Behavior | Reuse |
|---|---|---|
| `GET /trends?cycle_id&range=4w|quarter|cycle` (same aggregation function as reports, no filter) | typed struct: `kpis`, `weekly: [{week_start, by_type: {code, review, meeting, other}}]`, `priority_share`, `people[top 8]`, `repos`, `teams`, `weekday_hour` (optional, see note) | move logic from `cycle/routes/trends.rs:150-482` into `cycle/service/trends.rs` returning typed data instead of `serde_json::Value`; drop the hard-coded colours (`category_color`, line 13), colour is a client concern |
| `GET /integrations` | per service: status, last sync, config fields, excluded items, entry counts | `integrations/integrations_routes.rs:19-138`, `SERVICES` (`integrations/model.rs:26`) |
| `PUT /integrations/{service}`, `POST /integrations/{service}/test`, `DELETE /integrations/{service}?delete_entries=`, `DELETE /integrations/{service}/excluded/{id}` | JSON bodies; test returns `{ok, message}` | `:254,381,473,499,521` |
| `POST /sync` and `POST /sync/{service}` (optional `hard=true`) | **202 Accepted** and run in a background task (`tokio::spawn`). Never await a sync in a request (`sync_routes.rs:112,206` do today). | `integrations/orchestrator` |
| `GET /sync/status` | `{running, services: {...}, recent_logs[10]}` from `SyncStatusMap` + `SyncLog::recent_for_user` | `sync_routes.rs:25-95` |

Notes:
- `weekday_hour`: entries only have a date (`occurred_at`). Hour exists for meetings (`start_time`) and `created_at`. If this is not accurate enough, ship weekday only and hide the hour axis. Label the card "logged at".
- Wrap sync upserts in a transaction per service (`orchestrator.rs` ~243-330 does find_or_create + find + create per item without one).

Acceptance for each M2 part: contract tests (JSON shape + ownership + one unhappy path per write endpoint), `moon run api:lint api:test`, `types:check`. Legacy HTML routes still pass their old tests.

---

## M3. Web foundation

Goal: empty but complete shell in `apps/web`: auth gate, layout, routing, API client, tokens, tests, lint boundaries.

1. Scaffold: `npm create vite@latest apps/web -- --template react-ts`, then align to workledger's versions where they overlap (`/Users/bastian/CodingIsFun/_workspaces/workledger/package.json`): React 19.2, Vite 7, TypeScript 5.9, Tailwind 4.1 + `@tailwindcss/vite`, vitest 4, eslint 9 flat config with `typescript-eslint`, `eslint-plugin-react-hooks`, `eslint-plugin-react-refresh`. Add `react-router` 7, `@tanstack/react-query` 5, `zod`, `react-hook-form`, `@hookform/resolvers`, `@radix-ui/react-{dialog,popover,dropdown-menu,tabs,tooltip,select}`, `date-fns`, `clsx`, `@testing-library/react`, `@testing-library/user-event`, `jsdom`, `msw` 2, `eslint-plugin-import`. Editor and export packages are added in M4 (reports/review), not now.
2. `apps/web/moon.yml`: tasks `dev` (`vite`), `build` (`tsc -b && vite build`, outputs `dist`), `lint` (`eslint .`), `typecheck`, `test` (`vitest run`). `api:dev` and `web:dev` run together via a root task `dev` (`deps` + `local`).
3. `vite.config.ts`: dev proxy `{ '/api': 'http://localhost:8080', '/auth': 'http://localhost:8080', '/custom': ..., '/healthz': ... }` so cookies and OAuth stay same-origin. `build.rollupOptions.output.manualChunks` for `react`, `router`, `query`. Editor and export are dynamic imports (M4).
4. Tokens: `src/app/app.css` with Tailwind v4 `@import "tailwindcss"` and `@theme` mapping the **existing** values from `static/css/tokens.css` (same names without the `--color-` prefix removal: keep `--color-primary: #FF453F`, `--color-bg: #F0EBE3`, `--color-bg-secondary: #FAF0E6`, `--color-surface: #fff`, `--color-text: #000`, `--color-text-secondary: #5c5c5c`, `--color-border: #d4cec6`, the semantic and tint colours, radii 6/10/16, shadows, `--font-sans: 'Mozilla Text'`, `--font-display: 'Mozilla Headline'`). Copy fonts to `public/fonts/` and declare `@font-face` with `url('/fonts/...')`, `font-display: swap`. `index.html` links `/custom/tokens.css` after the app CSS (the Mozilla overlay hook keeps working). Visual reference: `mocks/assets/mock.css`.
   Font guard: the mocks showed non-brand fonts in places, so make this a checked requirement. `body` uses `var(--font-sans)` (Mozilla Text), headings `var(--font-display)` (Mozilla Headline), and form controls must use `font: inherit` (browsers default them to the system font). No other `font-family` anywhere (no serif for the editor; style BlockNote's content with the same stack). Add a Vitest/RTL test that renders the app shell and asserts `getComputedStyle(document.body).fontFamily` starts with "Mozilla Text", and do a visual check of Review, Logbook and Priorities against the old app before M5.
   Visual rules (from review feedback, keep the old app's calm look): headings 600 (page title 700 only), body and nav 400, buttons and active states 500. Active nav and selected chips use a tinted coral background (`rgba(255,69,63,.14)`) with coral text, never a solid fill or black. Hairline borders `#e3ddd4`, no card shadows, few chips and badges per row (one accent line per entry, details live in the drawer). Drawers are closed by default.
5. `src/lib/api-client.ts`: thin `fetch` wrapper. Always sends `X-Requested-With: brag-frog`, `credentials: 'same-origin'`, `Accept: application/json`. Parses the `{error:{code,message}}` shape into `ApiError`. 401 clears the query cache and navigates to `/signin`. Supports `AbortSignal`. No retries on 4xx.
6. `src/lib/react-query.ts`: `QueryClient` with `staleTime: 30_000`, `gcTime: 5*60_000`, `refetchOnWindowFocus: false`, retry 1. Query-key factories live in each feature's `api/` folder (`['entries', cycleId, filters]`).
7. `src/app/`: `provider.tsx` (QueryClientProvider, error boundary, toast), `router.tsx` (data router, `lazy()` per route), `routes/` one file per route. Routes: `/signin`, `/today`, `/logbook`, `/priorities`, `/review/:cycleId?/:section?`, `/trends`, `/settings/:tab?`, `/privacy`, `/terms`. `/` redirects to `/today` (or `/signin` on 401). Unknown paths render a not-found page.
8. `features/auth`: `useMe()` (`GET /me`), `AuthGate` (loading, 401 redirect), sign-in page (link to `/auth/login`, instance name from `/public-config`).
9. `components/layout`: `AppShell` (sidebar from `mocks/assets/shell.js`: cycle switcher, ⌘K, nav, sync indicator, user), mobile top nav below 860px, read-only banner for non-active cycles. Reuse structure from workledger's `src/components/layout/AppShell.tsx` and `src/app/components/AppHeader.tsx`. `components/ui`: Button, Card, Badge, Chip, Tabs, Drawer (Radix Dialog), Dialog, Select, Tooltip, Toast, EmptyState, Spinner, `ErrorBoundary` (copy `workledger/src/components/ui/ErrorBoundary.tsx`, `StatusToast.tsx`, `ConfirmAction.tsx`).
10. Shortcuts: `hooks/use-keyboard-shortcuts.ts` adapted from `workledger/src/hooks/useKeyboardShortcuts.ts`: ⌘K command palette (navigate + "Sync now" + search entries), ⌘J focus composer, `g` then `t/l/p/r/v/d/s` to navigate. Skip when focus is in an input or editor.
11. Cycle context: `features/cycles` provides `useActiveCycle()` and a `selectedCycleId` in the URL search param (`?cycle=ID`) so past cycles are shareable and read-only (mutations hidden when `cycle.id !== active.id`).
12. Testing: `src/testing/setup.ts` (jsdom, `@testing-library/jest-dom`), `src/testing/mocks/{handlers.ts,server.ts}` (MSW, handlers typed with generated types), `test-utils.tsx` (`renderWithProviders`). Vitest config in `vite.config.ts` (`test: { environment: 'jsdom', setupFiles }`).
13. ESLint: `import/no-restricted-paths` zones exactly as in bulletproof-react `docs/project-structure.md` (one zone per feature forbidding imports from other features; `features` cannot import `app`; shared folders cannot import `features` or `app`). Add a deliberate-violation fixture test that proves the rule fires.

Acceptance: `moon run web:lint web:typecheck web:test web:build` green; with `api:dev` + `web:dev` running, visiting `localhost:5173` while logged out lands on `/signin`, clicking sign in completes Google OAuth (same-origin via proxy) and lands on an empty Today inside the shell; 401 handling verified by deleting the cookie.

---

## M4. Features (one PR each, order matters)

Common requirements for every feature: loading skeleton, empty state, error state with retry, mutation errors as toasts, keyboard accessible controls (Radix), works at 375px, read-only when viewing a past cycle, an MSW-based test per main flow (render, mutate, error). Visual target is the matching file in `mocks/`. Use TanStack Query: `useQuery` for reads; `useMutation` with `invalidateQueries` (optimistic update only for entry edits and priority status).

### M4.1 Logbook (`features/logbook`, `features/entries`)
- `features/entries/api`: `useEntries(filters)` as `useInfiniteQuery` (cursor), `useEntryFacets`, create/update/delete/bulk/classify/exclude mutations. `features/entries/components`: `EntryRow`, `EntryDrawer` (edit form: react-hook-form + zod; fields from the mock), `Composer`, `SourceIcon`, `EvidenceRail` (used by review, takes `onInsert` prop).
- `features/logbook`: keep the existing layout (mocks/logbook.html): search + dept goal + priority on one row, a "More filters" toggle for the rest (source, team, person, type, reach, complexity, role, show only), date groups with count pills, entry rows with source icon, subtitle, date and external-link icon; the drawer replaces inline edit forms; no always-on composer, a "Log entry" button opens the composer dialog. A **Report** button next to it opens a dialog (`features/logbook/components/ReportDialog`): tab New (shows the active filters as chips and the matching entry count, title input, "Include an AI summary" checkbox shown only when `has_ai`, format PDF/DOCX/Markdown, **Generate and download**) and tab Past reports (title, count, date, per-format download buttons, delete). The dialog reads the filters from the URL search params, so what you see is what is reported. Filters in the URL;, filters stored in the URL search params (`useSearchParams`) so views are linkable; search debounced 250 ms; groups by week with sticky headers; virtualise with `@tanstack/react-virtual` once a group exceeds 200 rows; select mode with bulk edit; quick views ("No priority", "No team", "Needs classify").
- Composer: Enter logs, Shift+Enter opens details, chips for type/priority/team/people (people and team autocomplete from facets). Optimistic insert at top.
- Tests: filter -> request params; infinite scroll; edit via drawer; delete confirm.

### M4.2 Today (`features/today`)
- `useToday()`; blocks: composer (reuse `features/entries` through the route composing it as a slot), week strip (meetings, "prep ready/no prep" badges, click opens a Drawer with the meeting prep form: notes, doc URLs, goal, priority, AI draft button), priorities (top active with status and blocker count), open work (filter tabs All/Code/Reviews/Tickets).
- Replaces the focus-time block with a single line "9 meetings, 11h focus" in the week card header.

### M4.3 Priorities (`features/priorities`)
- Goals as groups, priority rows (progress from `measure_*`, tracking badge, due), "Active/Done/All" segmented filter, drawer with edit form, "post update" form and update timeline, linked entries count linking to the logbook with `?priority_id=`. Create goal / create priority dialogs, Lattice CSV import dialog (shows the result list). No cycles UI here.

### M4.4 Settings, cycles, integrations (`features/settings`, `features/cycles`, `features/integrations`)
- Tabs from the URL (`/settings/:tab`; reached from a gear icon at the bottom of the sidebar, not a nav item): profile (zod form), cycles (create dialog, activate, delete with typed-name confirm, **Download** menu: Markdown, PDF, DOCX built in M4.7), people, integrations, AI key.
- Integrations: list from `/integrations`; configure drawer with per-service fields from `/config`; test button; reset with "also delete entries" checkbox; excluded items restore list; "Sync all" posts 202 then polls `/sync/status` every 2 s while `running`, every 60 s otherwise (stop when the tab is hidden). The sidebar sync indicator reads the same query (no extra polling).

### M4.5 Editor (`components/editor`)
- Add deps: `@blocknote/core`, `@blocknote/react`, `@blocknote/mantine`, `@mantine/core` (same versions as workledger, 0.55.x / 8.3.x). Mantine is only imported inside `components/editor`, wrapped in its own `MantineProvider`, and the whole module is `React.lazy`-loaded so other routes do not pay for it. Check that Tailwind preflight does not break BlockNote styles (scope by wrapping in a class and adding minimal overrides).
- Copy from workledger and adapt (read each file first):
  - `src/features/editor/components/EditorProvider.tsx` (schema; **drop** the Excalidraw block and code-block language list beyond ~8; keep default blocks)
  - `src/features/editor/components/EntryEditor.tsx` (editor creation, `replaceBlocks` on external updates, suggestion menus)
  - `src/features/entries/hooks/useAutoSave.ts` (500 ms debounce, flush on unmount/pagehide/hidden)
  - `src/features/editor/specs/EntryLinkSpec.tsx` becomes an `entryRef` inline content spec: props `entryId`, `label`; rendered as the blue chip from the mocks; clicking opens the entry drawer (via a callback prop, not a feature import).
  - `[[` suggestion menu searches entries through a `searchEntries(query)` prop supplied by the route.
- Tests: autosave debounce (fake timers) and entry reference insertion.

### M4.6 Review (`features/review`)
- Single column, full content width like the other views (mock: `mocks/review.html`). Section tabs as pills at the top (`?section=slug` in the URL, status shown as small muted text "empty"), a one-line hint with two text links: Guidance (expands `form_question`, `form_guidance`, `form_bullets`, `form_tip` inline) and Career level (opens a Drawer from `/config` CLG data, replaces the level guide page). One BlockNote editor in a single card, autosaved. Header has only two buttons: **Export** (menu) and **Draft with AI** (uses the goal focus selection, `focus_priorities`, shown in a small popover on that button, not as always-visible checkboxes). No Write/Preview toggle (the editor is WYSIWYG).
- Evidence: a quiet text action under the editor, "＋ Add evidence from your logbook", opens a right-hand Drawer with search, a project select and entry rows with an **Insert** button (inserts an `entryRef` at the cursor). The drawer is closed by default; there is no permanent evidence column. `app/routes/review.tsx` composes `features/entries/EvidencePicker` into the page as a slot. It reads `GET /entries` with the filters.
- A small link at the bottom opens the read-only archive (`/review/:cycleId/archive`, from `GET /cycles/{id}/archive`).

### M4.7 Export (`lib/export`)
- Shared, not a feature. Copy and adapt from workledger (read first, keep file layout):
  - `src/features/editor/export.ts` -> `src/lib/export/index.ts` (`downloadDocument(kind, doc)`; dynamic imports for pdf and docx)
  - `utils/entry-document.ts` -> `lib/export/document.ts` (flatten blocks to the `DocBlock` union)
  - `export/pdf.tsx`, `export/docx.ts`, `export/style.ts`, `export/page.ts`, `export/fonts.ts`, `export/font-text.ts`
  - `utils/entry-markdown.ts` -> `lib/export/markdown.ts` (turn `entryRef` into plain text with the entry title)
  - tests `export/__tests__/renderers.test.ts`
- Add deps `@react-pdf/renderer`, `docx`, `@ibm/plex-sans` (PDF font) or embed Mozilla Text if the licence allows (check `static/fonts` licence first; otherwise keep Plex).
- Three entry points, all client-side so the server stays JSON-only:
  1. Logbook report (from `GET /reports/{id}`): title, filter description, stats, summary, entries per week (drawn as a simple table or vector bars in @react-pdf), who you worked with, key entries to PDF/DOCX/MD. No image rendering needed.
  2. Review section(s): MD/PDF/DOCX.
  3. Cycle bundle (Settings -> Cycles -> Download): fetch `/cycles/{id}/bundle`, compose one document: title and period, written self-review (sections), goals and priorities with updates, then entries grouped by ISO week. Keep PDFs readable for 400+ entries (page breaks per week; no per-entry images).
- Add the markdown/json bundle download as a plain "Raw JSON" option for backups.

### M4.8 Trends (`features/trends`)
- Range switcher in the URL. KPI row, weekly stacked bars (small SVG, own component; follow `punktespiegel/frontend/src/features/standings/bump-chart.tsx` for hand-written SVG style), priority share bars, top lists (people, repos, teams, click filters the logbook with the right query param), optional heat strip. No chart library. Colours from tokens only (purple/blue/coral/orange/green). Each chart has a text summary for screen readers and a table fallback behind a "Show data" toggle.

Acceptance per feature: `moon run web:lint web:typecheck web:test`; manual smoke against the real API for that view (record steps in the PR); Lighthouse (desktop) performance >= 90 on the built app for that route; no import-boundary violations.

---

## M5. Cutover (delete SSR)

Do this only after M4.1 to M4.8 are merged and used for at least a few days.

1. Serve the SPA from Axum in `apps/api/src/app.rs`:
   - Router order: `/healthz`, `/auth/*`, `/api/v1/*` (with its own JSON 404 fallback), `/custom/tokens.css`, then `fallback_service(ServeDir::new(web_dist).precompressed_gzip().fallback(ServeFile::new(web_dist/index.html)))`. Unknown non-API paths therefore return `index.html` (client routing).
   - Headers: `Cache-Control: public, max-age=31536000, immutable` for `/assets/*` and `/fonts/*`; `no-cache` for `index.html`. Use `SetResponseHeaderLayer`.
   - `CompressionLayer` (tower-http feature `compression-gzip`, `compression-br`) on the API and static fallback.
   - CSP in `kernel/middleware.rs`: `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; font-src 'self'; img-src 'self' data: https://*.googleusercontent.com; connect-src 'self'; frame-ancestors 'none'`. `'unsafe-inline'` styles stay because Mantine/BlockNote inject styles. Remove Google Fonts allowances and `unsafe-eval`.
2. Delete in one PR (end to end): `templates/`, `static/` (move favicon/img assets the SPA needs into `apps/web/public/`: `favicon.svg`, `favicon.png`, source icons used by the entry rows), all HTML handlers and their routes (everything in the section 3 table of routes that return PAGE/PARTIAL/HX-REDIR), `tera`, `pulldown-cmark` and `ammonia` (only if nothing else uses them: grep first; the markdown filter and Jira linkify were SSR-only), `hx_redirect`/`html_escape` helpers in `kernel/render.rs`, the `HX-Request` CSRF branch, `Form` DTOs, `kernel/serde_helpers.rs` helpers that only served HTML forms. Remove the dropped features end to end (delete `build_status_update_prompt` and the status update/last-week routes and models' write paths; `build_last_week_summary_prompt` and `EntryGroup` are adapted into `build_report_prompt` in M2d-2, not deleted): quarterly check-in routes + `reflections/routes`, contribution example routes, status update and last-week routes, level/review guide routes, `checkin_sections.toml`, `assessment_templates.toml`, `rating_scale.toml`, their loaders (`app.rs:63-68`) and their `custom/` overrides. **Keep** the models and repo functions needed by `GET /cycles/{id}/archive` (read-only). Keep `clg_levels.toml` (prompts and config endpoint).
3. Replace SSR tests in `apps/api/tests/integration/http_tests.rs` with the JSON contract tests from M2 (delete the HTML-substring ones). Keep `db_tests` and `sync_tests`.
4. `Dockerfile` (multi-stage; adapt paths to your final layout):
   ```dockerfile
   FROM node:24-alpine AS web
   WORKDIR /repo
   COPY package.json package-lock.json ./
   COPY apps/web/package.json apps/web/
   RUN npm ci
   COPY apps/web apps/web
   RUN npm run build -w apps/web

   FROM rust:1.93-slim AS api
   WORKDIR /repo
   COPY Cargo.toml Cargo.lock ./
   COPY apps/api/Cargo.toml apps/api/
   RUN mkdir -p apps/api/src && echo 'fn main(){}' > apps/api/src/main.rs && echo '' > apps/api/src/lib.rs \
    && cargo build --release -p brag-frog && rm -rf apps/api/src
   COPY apps/api/migrations apps/api/migrations
   COPY apps/api/src apps/api/src
   RUN touch apps/api/src/main.rs apps/api/src/lib.rs && cargo build --release -p brag-frog

   FROM debian:trixie-slim
   RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
   COPY --from=api /repo/target/release/brag-frog /usr/local/bin/brag-frog
   COPY --from=web /repo/apps/web/dist /app/web
   COPY apps/api/config /app/config
   WORKDIR /app
   ENV PORT=8080 BRAGFROG_DATABASE_PATH=/data/bragfrog.db BRAGFROG_WEB_DIST=/app/web BRAGFROG_CONFIG_DIR=/app/config
   EXPOSE 8080
   CMD ["brag-frog"]
   ```
   Verify that migrations are compile-time (`include_str!`) so the runtime image does not need them (current Dockerfile copies them anyway; keep copying if anything reads them at runtime).
5. Update `docker-compose.yml` (mount `./custom:/app/custom`, set `BRAGFROG_CUSTOM_DIR=/app/custom`), `.dockerignore` (`apps/web/node_modules`, `apps/web/dist`), `.gitignore`.
6. Docs: update `CLAUDE.md` (structure, stack: React SPA, remove HTMX/Tera gotchas 1, 5, 7, 8, 12, 13 as obsolete or rewritten), `docs/architecture.md`, `docs/conventions.md`, `docs/getting-started.md` (moon commands), `docs/self-hosting.md` (new env vars, health check), `docs/customization.md` (TOML removed ones, `custom/tokens.css` still supported).
7. Re-measure the M0 baseline and add an "after" table to `docs/perf-baseline.md`. Targets: API p95 under 150 ms for entries (50 rows) and today on a 1000-entry dataset locally; first load JS for `/today` under 200 KB gzip; editor chunk loaded only on reports/review.

Acceptance: `docker build .` and `docker run` serve the SPA at `/` and the API under `/api/v1`; deep links (`/review/3/impact_examples`) work after reload; cookie login works; `grep -r "hx-" apps` and `grep -r "tera" apps/api/Cargo.toml` return nothing; all tests green; manual run-through of the smoke checklist (section 7).

---

## M6. Versioned image and CI release

1. `.github/workflows/release.yml`: on tag `v*`, `docker buildx build` with cache, push `ghcr.io/<owner>/brag-frog:<tag>` and `:latest` (replace `<owner>` with the GitHub org/user that owns the repo; confirm it before the first run). If the repository or package is private, Cloud Build in M7 needs a token with `read:packages`; prefer making the image public if the source is public (`LICENSE` exists, confirm).
2. Release as `v6.0.0` (breaking: SSR removed, env vars added). Offer: update `CHANGELOG.md`, commit, tag, push, cut a GitHub release.
3. Keep the previous `v5.x` image tag available for rollback.

Acceptance: tag push produces an image that starts with only env vars + a volume.

---

## M7. `brag-frog-mozilla` changes (Google Cloud)

Current state (verified): local git repo (no remote), shell deploy to **Cloud Run gen2** in project `brag-frog`, region `us-central1`, service `brag-frog`, image `us-central1-docker.pkg.dev/brag-frog/brag-frog/brag-frog` (untagged), secrets via Secret Manager (`bragfrog-google-client-id`, `bragfrog-google-client-secret`, `bragfrog-kms-wrapped-dek`), SQLite on a GCS FUSE volume (`brag-frog-data` at `/data`, journal mode `delete`), KMS entrypoint (`deploy/gcp-kms-entrypoint.sh`), `--allow-unauthenticated`, `max-instances=1`, `min-instances=0`, no health check. The overlay `custom/` is only TOML (no templates or static customisations), so the frontend change does not break overrides. What breaks is `deploy/Dockerfile.mozilla` (copies `templates/` and `static/` from a sibling checkout).

Work in `/Users/bastian/CodingIsFun/_workspaces/brag-frog-mozilla`:

1. **`deploy/Dockerfile.mozilla`** becomes a thin layer on the released image:
   ```dockerfile
   ARG BRAGFROG_IMAGE
   FROM ${BRAGFROG_IMAGE}
   USER root
   RUN apt-get update && apt-get install -y --no-install-recommends curl python3 && rm -rf /var/lib/apt/lists/*
   COPY custom/ /app/custom/
   COPY deploy/gcp-kms-entrypoint.sh /usr/local/bin/entrypoint.sh
   ENV BRAGFROG_CUSTOM_DIR=/app/custom
   ENTRYPOINT ["entrypoint.sh"]
   CMD ["brag-frog"]
   ```
   Build context is the `brag-frog-mozilla` repo only. No more copying files into the upstream checkout, no `.gcloudignore` hack, no sibling `../brag-frog` dependency.
2. **`deploy/deploy.sh`:** replace `REPO_ROOT` logic with a required `BRAGFROG_VERSION` (for example `v6.0.0`). Build: `gcloud builds submit "$MOZ_ROOT" --config -` with `docker build --build-arg BRAGFROG_IMAGE=ghcr.io/<owner>/brag-frog:${BRAGFROG_VERSION} -f deploy/Dockerfile.mozilla -t ${IMAGE}:${BRAGFROG_VERSION}-moz.${MOZ_GIT_SHA} .`. Deploy that exact tag (no `:latest`). Print the digest after the build and store it in the release notes. Fix the dead "set BASE_URL on first deploy" branch (its default is never empty).
3. **Cloud Run flags** (edit `gcloud run deploy`):
   - `--startup-probe=httpGet.path=/healthz,httpGet.port=8080,periodSeconds=2,timeoutSeconds=2,failureThreshold=15` and `--liveness-probe=httpGet.path=/healthz,httpGet.port=8080`.
   - `--cpu-boost`.
   - Background sync runs on an in-process hourly timer (`main.rs:25`). With `min-instances=0` and request-based CPU the timer is throttled or the instance is gone, so hourly sync is unreliable today. Decide: either `--min-instances=1 --no-cpu-throttling` (about one always-on 1 vCPU/512Mi instance, recommended) or accept sync-on-login only.
   - Keep `max-instances=1` (SQLite single writer).
   - Env additions: none required. `BRAGFROG_CUSTOM_DIR` is set in the Dockerfile. `BRAGFROG_WEB_DIST`/`BRAGFROG_CONFIG_DIR` come from the base image.
4. **`custom/` cleanup:** delete `checkin_sections.toml` and `assessment_templates.toml` (loaders are gone in v6). Keep `clg_levels.toml`, `review_sections.toml`, `services.toml`. Remove from `review_sections.toml` any keys that only the old SSR forms used only if the API ignores them (check the `/config` DTO; unknown keys are harmless, so leave them if unsure). Note: `prompt*` fields stay server-side only and are no longer web-readable (M0 security fix).
5. **Docs:** update `README.md` (it says deploy.sh clones the source; it does not) and `docs/DEPLOY.md` (still says "three TOML files" and CultureAmp, links a missing `DEPLOYMENT_GUIDE.md`). Add a section "Upgrading": bump `BRAGFROG_VERSION`, run `deploy.sh`, verify `/healthz` and `/api/v1/public-config`, rollback = redeploy the previous `-moz.<sha>` tag.
6. **OAuth:** redirect URI stays `${URL}/auth/callback`; nothing changes in Google Cloud Console. The callback now lands on `/today`.
7. Verify on a staging revision first: `gcloud run deploy ... --no-traffic --tag=staging`, test `staging---brag-frog-....run.app` (add its `/auth/callback` to the OAuth client temporarily or test with the existing session cookie flow on the main URL via `--tag`), then `gcloud run services update-traffic --to-latest`.

Out of scope (decided: stay on Cloud Run): moving to mozcloud GKE, Cloud SQL, IAP. If that changes later, the single-container, env-configured image from M5 is already compatible (Workload Identity works with the KMS entrypoint via the metadata server).

Acceptance: `./deploy/deploy.sh` with `BRAGFROG_VERSION=v6.0.0` produces a tagged revision; probe passes; login works; an existing user's entries, priorities, and past cycles are visible; Review shows the old H1 self-review text (converted on open); rollback to the previous tag works.

---

## M8. Optional performance work (only if M7 measurements still show slow responses)

SQLite on GCS FUSE is slow for random IO and forces `journal_mode=delete` with a pool of 1 connection (`db/mod.rs:17-21`), so a background sync blocks every request and the `tower_sessions` write on each request goes through FUSE.

Option: local-disk SQLite replicated with **Litestream** to the existing bucket. Entrypoint: restore if missing (`litestream restore -if-db-not-exists -if-replica-exists`), then `litestream replicate -exec "brag-frog"`. Set `SQLITE_JOURNAL_MODE=wal`, keep `max-instances=1`, `min-instances=1`, memory 1Gi. Risks: data written since the last replication (about 1 s) can be lost if the instance is killed; restore time grows with DB size. Do a restore drill before switching. This changes deployment only, not application code. Implement in `brag-frog-mozilla` (new `deploy/litestream.yml`, entrypoint change, Dockerfile adds the litestream binary).

---

## 4. Known performance causes and where each is fixed

| Cause (file:line before the move) | Fix |
|---|---|
| Full phase decrypted per request, logbook twice (`logbook.rs:60,95`) | M2b: paged list, projection, one pass |
| Dashboard decrypts teams/collaborators for all entries (`dashboard.rs:86-89`, `worklog/repo.rs:446-495`) | M1 facets cache; `/today` does not load facets |
| Every filter change re-renders the phase (`logbook.rs:204`) | M4.1: JSON paging, client-side rendering, virtualisation |
| 322-line entry card with nested loops per entry (`entry_card.html`) | M4.1: one shared drawer, light rows |
| N+1 for contribution example links (`contribution_examples.rs:43-50`) | M2d archive: one batched query |
| Review reloads all entries for Jira links (`summaries.rs:116,306,337`) | M2d: linkify on the client |
| Sync awaited in request (`sync_routes.rs:112,206`) | M2e: 202 + status polling |
| 16 CSS files, no cache headers, no compression (`base.html:15-34`, `app.rs:437`) | M5: one hashed bundle, immutable cache, gzip/br |
| Polling `/sync/status` every 30 s on all pages (`base.html:299-307`) | M4.4: one shared query, visibility-aware |
| `has_ai_for_user` query per page | M1: part of `/me` |
| SQLite on GCS FUSE | M0 measure, M8 if needed |
| Blocking CPU work (AES) inside async handlers | M1 `spawn_blocking` helper |

## 5. Risks and decisions to confirm during execution

- **BlockNote bundle size and Mantine CSS vs Tailwind.** Lazy-load; if Mantine styles leak, scope them under the editor container. Fallback: plain `<textarea>` markdown editing for one release (the API stores markdown either way, `content_format` marks the type).
- **Markdown to blocks conversion** (`tryParseMarkdownToBlocks`) is lossy for exotic markdown. Old content is simple (AI drafts and typed text); keep the original markdown in the DB until the user saves from the new editor, and show a "view original" link for converted docs.
- **PDF fonts.** @react-pdf needs embeddable fonts. Plex Sans (from workledger) is safe. Using Mozilla Text in PDFs requires checking the font licence.
- **Cookie behaviour.** Same origin means the existing cookie (`SameSite=Lax`, 12 h idle) keeps working. Do not add CORS. The Vite proxy preserves this in dev.
- **ts-rs and chrono.** Dates are strings in the DTOs (`String`), not `chrono` types, to keep generated types simple.
- **Moon v2 config keys.** The handbook confirms `.moon/toolchains.yml` and per-project `moon.yml` with `command:` tasks; confirm anything else with `moon init` output before committing config.
- **Upstream image visibility** (ghcr public vs private) decides how Cloud Build pulls it in M7.
- **Hourly sync and Cloud Run CPU** (M7 step 3) is a cost/behaviour trade-off to decide with the owner.

## 6. File-by-file reuse map

| Need | Take from | Adapt |
|---|---|---|
| App shell, header, sidebar structure | `workledger/src/components/layout/AppShell.tsx`, `src/app/components/AppHeader.tsx`, `src/features/sidebar/components/Sidebar.tsx` | Replace day list with the brag-frog nav; add cycle switcher |
| Editor | `workledger/src/features/editor/components/{EditorProvider,EntryEditor}.tsx`, `specs/EntryLinkSpec.tsx`, `slash menu` files | Drop Excalidraw, rename link spec to `entryRef` |
| Autosave | `workledger/src/features/entries/hooks/useAutoSave.ts` | PUT to API instead of IndexedDB |
| Export | `workledger/src/features/editor/export.ts`, `export/*`, `utils/entry-document.ts`, `utils/entry-markdown.ts` | Add cycle-bundle composer |
| Shortcuts, error boundary, toast, confirm | `workledger/src/hooks/useKeyboardShortcuts.ts`, `src/components/ui/*` | Add `g` chords |
| Week utilities | `workledger/src/utils/weeks.ts` | ISO week helpers |
| Architecture rules | `punktespiegel/frontend/testing/architecture.test.mjs` (idea), bulletproof-react ESLint zones | Use ESLint zones as the enforcement |
| Async resource handling ideas | `punktespiegel/frontend/src/hooks/use-resource.ts`, `components/resource-panel.tsx` | TanStack Query replaces the hook; keep the "keep stale content while loading" behavior via `placeholderData: keepPreviousData` |
| Hand-written SVG chart style | `punktespiegel/frontend/src/features/standings/bump-chart.tsx` | Stacked bars |
| Rust + npm workspace layout, nginx notes | `punktespiegel/Cargo.toml`, `package.json`, `Dockerfile.web` | Axum serves the SPA, nginx is not used |

## 7. Smoke checklist (run at M5, M7)

1. Logged out: `/` goes to `/signin`; Google login completes; lands on `/today`.
2. Log an entry from Today and from Logbook; it appears in both without reload.
3. Filter the Logbook by source, priority and text; reload keeps the filters (URL); infinite scroll loads more.
4. Edit an entry in the drawer, change priority, delete a manual entry.
5. Priorities: create goal and priority, post an update with a blocker, see the timeline; complete a goal.
6. Settings: create a cycle, activate it, switch back; a past cycle shows the read-only banner; download MD, PDF, DOCX for a past cycle and open each.
7. Integrations: save a token, test connection, run Sync all; status updates without freezing the UI.
8. Logbook report: set filters, click Report, generate PDF (downloads and appears in Past reports), download DOCX and MD from Past reports, delete one; an empty filter result shows an inline error; with no AI key the report still has stats.
9. Review: write two sections, preview, export DOCX; archive shows old check-ins.
10. Trends: switch ranges; click a person goes to the filtered Logbook.
11. Mobile (375px): navigation, Today, Logbook, drawer full width.
12. `curl -i /api/v1/me` without cookie returns 401 JSON; `curl /custom/review_sections.toml` returns 404.

## 8. Order of work for the executor

1. M0 (and its security fix PR first).
2. M1.
3. In parallel: M2a to M2e (backend) and M3 (web foundation).
4. M4.1 to M4.8, each after its API part has merged.
5. M5, M6, then M7. M8 only if needed.
6. Finish: offer to update `CHANGELOG.md`, then commit, tag, push, and optionally cut the release (user workflow).
