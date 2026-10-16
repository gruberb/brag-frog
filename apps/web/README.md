# Brag Frog web

React 19 SPA for Brag Frog. It talks to the Axum JSON API under `/api/v1` with the same session cookie, so there is no CORS and no token handling in the browser.

Stack: Vite 7, React Router 7, TanStack Query 5, Tailwind v4 (`@theme` tokens), Radix, react-hook-form + zod, BlockNote (lazy, Review only), `@react-pdf/renderer` and `docx` (lazy, on download).

## Commands

Run from the repo root with moon, or from `apps/web` with npm.

| Task | moon | npm |
|---|---|---|
| Dev server (proxies `/api /auth /custom /healthz` to `localhost:8080`) | `moon run web:dev` | `npm run dev` |
| Dev server with in-memory MSW data, no backend | | `npm run dev:mock` |
| Lint (ESLint, includes import boundaries) | `moon run web:lint` | `npm run lint` |
| Typecheck | `moon run web:typecheck` | `npm run typecheck` |
| Tests (Vitest + Testing Library + MSW) | `moon run web:test` | `npm run test` |
| Production build to `dist/` | `moon run web:build` | `npm run build` |

`BRAGFROG_API_URL` overrides the proxy target (default `http://localhost:8080`).

`dev:mock` starts the MSW service worker (`public/mockServiceWorker.js`, removed from production builds) with the stateful handlers in `src/testing/mocks`. The same handlers back the tests, so mock data and test data cannot drift.

## Structure

```
src/
  app/          main, provider, router, routes/* (composition of features), command palette
  components/   ui/ (Button, Drawer, Dialog, ...), layout/AppShell, editor/ (BlockNote, lazy)
  config/       paths, env
  features/     auth cycles entries integrations logbook priorities review settings today trends
  hooks/ lib/ stores/ testing/ types/ utils/
```

Rules (enforced by `import/no-restricted-paths`, proven by `src/testing/architecture.test.ts`):

- Flow is one way: shared (`components hooks lib stores types utils config`) then `features` then `app`.
- A feature never imports another feature. Routes in `src/app/routes` compose features and pass slots as props or components (for example the evidence picker and entry drawer into the Review page, the quick composer into Today).
- No barrel files. Query keys live next to each feature's `api/` code. `src/types/api` is for generated types.

## Look and feel

Tokens in `src/app/app.css` are the values of `apps/api/static/css/tokens.css` (`@theme static`, so `/custom/tokens.css` can still override them; that link sits after `#root` in `index.html` to win the cascade). Mozilla Headline and Mozilla Text are copied to `public/fonts`. Body uses `--font-stack`, headings `--font-stack-heading`, form controls use `font: inherit`. `src/app/shell.test.tsx` fails if any other `font-family` appears in UI code (PDF and DOCX output uses IBM Plex Sans, since react-pdf cannot embed WOFF2 variable fonts).

## API contract assumptions

Types are hand-written in `src/types/api/models.ts` from `implementation.md` until the ts-rs files land in `src/types/api/`. Generated types win on conflict; where this file had to guess, the guess is listed here so the backend can confirm or the type can be adjusted.

- Errors are `{ "error": { "code", "message" } }`. 401 clears the query cache and navigates to `/signin`. Every non-GET request sends `X-Requested-With: brag-frog`.
- `GET /me`: `user` also carries `display_name`, `avatar_url` and `role` (career level id, matches `clg_levels[].id`; stored as `users.role`). `active_cycle` has `entry_count` and `priority_count`.
- `GET /cycles` returns `Cycle[]` (`id, name, start_date, end_date, is_active, entry_count, priority_count`). `DELETE /cycles/{id}` takes `{ confirm_name }`.
- Cursor paging: the client sends `cursor` (opaque) and `limit=50` to `GET /entries` and reads `{ items, next_cursor, total_matching }`. Filters use the plan's key names plus a boolean `needs_classify` (for the "Needs classify" quick view). Booleans are sent as `true`.
- `GET /entries/facets`: `{ teams, people, sources, counts: { total, no_priority, no_team, needs_classify } }`.
- `GET /today`: `week { id, week_start, week_end, iso_week, year }`, `meetings_by_day [{ date, label, is_today, meetings [{ entry_id, title, start_time, end_time, recurring_group }] }]`, `prep_map` keyed by entry id, `active_work` as `EntrySummary[]`, `priorities` as priorities with `entry_count`, `latest_update`, `blocker_count`, `focus_days`, `counts { meetings, focus_hours }`.
- Meeting prep: `GET/PUT /meetings/{entry_id}/prep` with `{ notes, doc_urls[], meeting_goal, priority_id }`, `POST .../prep/draft` returns `{ content }`.
- Priorities: statuses `not_started | active | on_hold | completed | cancelled`, tracking `on_track | progressing | off_track | complete | incomplete | no_update`. `GET /priorities` returns `{ goals [{ goal, priorities }], unassigned, counts { active, done, all } }`. `POST /priorities/{id}/updates` takes `{ tracking_status, measure_value, comment, is_blocker }`. Import sends multipart `csv_file` and `cycle_id`. Goal completion is `PUT /goals/{id} { status: "completed" }`.
- Review: `GET /cycles/{id}/review` returns all sections with `content` and `content_format`. The client saves `content_format: "blocknote"` (JSON string of blocks) and converts old markdown on open. `POST .../draft` takes `{ goal_ids }` and returns `{ markdown }`. Jira keys are linkified on the client with `atlassian_base_url` from `/config`.
- `GET /config`: `{ instance_name, review_sections[], services[], clg_levels[], atlassian_base_url }`. `services[]` entries are `{ service, label, token_url, note, base_url_placeholder, email_placeholder, org_placeholder, default_orgs, default_base_url }`.
- Settings: `GET/PUT /settings/profile` uses `Profile` (`role` is the career level), `GET/POST/DELETE /settings/people`.
- Integrations: `GET /integrations` returns one entry per service with `status` (`connected | not_connected | error`), `last_sync_at`, `last_error`, `entry_count`, `config` (`base_url`, `email`, `orgs`), `has_token` and `excluded [{ id, label }]`. The Anthropic key is the `claude` service in the same API (`PUT { token }`, `POST .../test`, `DELETE`); the Integrations tab hides it and the AI tab manages it. `POST /sync` and `/sync/{service}` return 202; `GET /sync/status` is `{ running, services, recent_logs }` and is polled every 2 s while running, every 60 s otherwise, never in a hidden tab.
- Reports: `POST /reports` returns the full report (`stats`, `narrative`, `key_entries`, `ai_error`). A 422 with code `empty_selection` is shown inline. Downloads are rendered client-side from `GET /reports/{id}`.
- Trends: `GET /trends?cycle_id&range=4w|quarter|cycle` returns `kpis`, `weekly`, `priority_share`, `people`, `repos`, `teams`, `weekday_hour` (`null` hides the heat strip).
- Cycle bundle (`GET /cycles/{id}/bundle`): `{ cycle, review[], goals[], priorities[] with updates, weeks [{ week_start, iso_week, entries[] }] }`.

## Notes for later milestones

- First load for `/today` pulls zod through the entry form. If the 200 KB gzip budget in M5 is tight, the entry form can move behind a dynamic import.
- Editor chunk (`RichEditor`) loads only on `/review`; PDF and DOCX chunks load only when a download starts.
