# Contributing to Brag Frog

Thanks for your interest in contributing! This guide will help you get started.

## Getting Started

```bash
# Clone the repo
git clone https://github.com/gruberb/brag-frog.git
cd brag-frog

# Set up environment
cp .env.example .env
# Fill in BRAGFROG_GOOGLE_CLIENT_ID, BRAGFROG_GOOGLE_CLIENT_SECRET, BRAGFROG_ENCRYPTION_KEY

npm ci                  # web dependencies
moon run api:dev        # API on http://localhost:8080
moon run web:dev        # web app on http://localhost:5173

# Lint, test and build everything (what CI runs)
moon run :lint :test :build
```

**Requirements:** [moon](https://moonrepo.dev), Node 24 (pinned in `.prototools`), a Rust toolchain with clippy, and a working C linker (for SQLite). API tests use in-memory SQLite and web tests use MSW, so no external services are needed.

## Project Structure

```
apps/
├── api/                 # Rust crate `brag-frog` (Axum JSON API + OAuth redirects)
│   ├── src/
│   │   ├── api/         # /api/v1 handlers and DTOs, one file per area
│   │   ├── identity/    # Google OAuth, users, career levels
│   │   ├── worklog/     # Entries and entry types
│   │   ├── objectives/  # Priorities, department goals
│   │   ├── cycle/       # Cycles, weeks, meeting prep, trends
│   │   ├── review/      # Summaries, reports, AI documents
│   │   ├── integrations/# Sync services (GitHub, Jira, etc.)
│   │   ├── ai/          # Anthropic client and prompts
│   │   ├── kernel/      # Config, crypto, errors, middleware
│   │   └── db/          # SQLite pool, migrations
│   ├── config/          # Default TOML config (levels, review sections, services)
│   ├── migrations/      # SQL migration files
│   └── tests/           # Integration tests
└── web/                 # React 19 + Vite SPA
    └── src/             # app/, features/, components/, hooks/, lib/, types/api (generated)
.moon/                   # moon workspace and toolchains
website/                 # Landing page (separate from the app)
```

## Key Conventions

**API handlers** return `ApiResult<T>` (JSON). Use the `ApiAuth` extractor, keep handlers thin, and register routes in `apps/api/src/api/mod.rs`. State-changing requests need `X-Requested-With: brag-frog`.

**Generated types.** Wire types marked `#[ts(export)]` are exported to `apps/web/src/types/api/`. After changing one, run `moon run api:types` and commit the result. Never edit those files by hand.

**Web app structure.** Code flows one way: shared (`components`, `hooks`, `lib`, `stores`, `types`, `utils`, `config`) then `features` then `app`. A feature never imports another feature; compose features in `src/app/routes`. ESLint enforces this. No barrel files.

**Runtime SQL.** We use `sqlx::query` / `query_as` (not `query!`), so SQL errors surface at runtime. Run the app and test your queries.

**Encrypted fields.** Titles, descriptions, and other sensitive fields are AES-256-GCM encrypted BLOBs. You can't use SQL `LIKE` on them: filter in application code after decryption.

## Making Changes

### Branch naming

```
fix/short-description     # Bug fixes
feat/short-description    # New features
chore/short-description   # Maintenance, deps, docs
```

### Commit messages

Use conventional-ish prefixes:

```
fix: description of what was fixed
feat: description of the new feature
chore: maintenance task
docs: documentation change
```

### Pull request process

1. Fork the repo and create a branch from `main`
2. Make your changes
3. Run `moon run :lint :test :build`
4. Open a PR against `main`: the template will guide you
5. Keep PRs focused: one feature or fix per PR

### Adding a migration

1. Create a new `.sql` file in `apps/api/migrations/` (next number after `025_reports.sql`)
2. Register it in the `INCREMENTAL_MIGRATIONS` array in `apps/api/src/db/mod.rs`
3. Never modify `001_initial.sql` or any released migration
4. Migrations run automatically at startup

### CSS changes

Two CSS surfaces exist and must stay in sync:
- `website/css/`: landing page
- `apps/web/src/app/app.css` and `components.css`: the web app, including the sign-in page

Design tokens live in `apps/web/src/app/app.css`. Brand colours and the Mozilla Headline and Mozilla Text fonts are fixed: use shades of the existing colours, not new hues or fonts. `apps/web/src/app/shell.test.tsx` fails on other `font-family` values.

## What to Work On

Issues labeled [`good first issue`](https://github.com/gruberb/brag-frog/labels/good%20first%20issue) are a great starting point.

| Area | What it covers |
|------|---------------|
| `area: integrations` | GitHub, Jira, Phabricator, Bugzilla, Confluence, Google Drive/Calendar sync |
| `area: okrs` | Goals, key results, initiatives, phases |
| `area: ai` | Meeting prep drafts, self-review generation |
| `area: ui` | React components, CSS |
| `area: database` | Schema, migrations, queries |
| `area: auth` | Google OAuth, sessions, encryption |

## Questions?

Open a thread in [Discussions](https://github.com/gruberb/brag-frog/discussions). For bugs and feature requests, use the issue templates.
