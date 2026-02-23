# Contributing to Brag Frog

Thanks for your interest in contributing! This guide will help you get started.

## Getting Started

```bash
# Clone the repo
git clone https://github.com/gruberb/brag-frog.git
cd brag-frog

# Set up environment
cp .env.example .env
# Fill in GOOGLE_CLIENT_ID, GOOGLE_CLIENT_SECRET, ENCRYPTION_KEY

# Run the app
cargo run  # http://localhost:8080

# Run tests (uses in-memory SQLite, no external deps)
cargo test
```

**Requirements:** Rust 1.75+ and a working C linker (for SQLite).

## Project Structure

```
src/
├── main.rs              # Entry point
├── lib.rs               # Module declarations, AppState
├── routes/              # Axum route handlers
├── models/              # Data types, entry types, KR scoring
├── db/                  # SQLite queries, migrations
├── services/            # Sync services (GitHub, Jira, etc.)
├── templates.rs         # Tera template loading
├── crypto.rs            # AES-256-GCM encryption
└── serde_helpers.rs     # Form deserialization helpers
templates/               # Tera HTML templates (SSR)
├── pages/               # Full page templates
├── components/          # HTMX partial fragments
├── panels/              # Slide-over panel content
└── base.html            # Layout shell
static/                  # CSS, JS, images (no build step)
config/                  # TOML config files (levels, review sections, services)
migrations/              # SQL migration files
website/                 # Landing page (separate from the app)
```

## Key Conventions

**Route handlers** return `Result<Html<String>, AppError>`:
- Full pages render `pages/*.html`
- HTMX partials render `components/*.html`
- Panel content renders `panels/*.html`

**HTMX everywhere.** The `<body>` has `hx-boost="true"`, so all links and forms are intercepted. Entry CRUD returns HTML fragments for `hx-swap`.

**No build step.** Templates are loaded at startup. CSS and JS are served from `static/` as-is.

**Runtime SQL.** We use `sqlx::query` / `query_as` (not `query!`), so SQL errors surface at runtime. Run the app and test your queries.

**Encrypted fields.** Titles, descriptions, and other sensitive fields are AES-256-GCM encrypted BLOBs. You can't use SQL `LIKE` on them — filter in application code after decryption.

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
3. Run `cargo test` and `cargo clippy`
4. Open a PR against `main` — the template will guide you
5. Keep PRs focused: one feature or fix per PR

### Adding a migration

1. Create a new `.sql` file in `migrations/` (e.g., `005_add_foo.sql`)
2. Register it in the `INCREMENTAL_MIGRATIONS` array in `src/db/mod.rs`
3. Never modify `001_initial.sql` or any released migration
4. Migrations run automatically at startup

### CSS changes

Three CSS surfaces exist and must stay in sync:
- `website/css/` — Landing page
- `static/css/main.css` (`.login-page`) — Sign-in page
- `static/css/` — App pages

Design tokens live in `static/css/tokens.css`. Keep the existing color palette and fonts.

## What to Work On

Issues labeled [`good first issue`](https://github.com/gruberb/brag-frog/labels/good%20first%20issue) are a great starting point.

| Area | What it covers |
|------|---------------|
| `area: integrations` | GitHub, Jira, Phabricator, Bugzilla, Confluence, Google Drive/Calendar sync |
| `area: okrs` | Goals, key results, initiatives, phases |
| `area: ai` | Meeting prep drafts, self-review generation |
| `area: ui` | Templates, CSS, HTMX interactions |
| `area: database` | Schema, migrations, queries |
| `area: auth` | Google OAuth, sessions, encryption |

## Questions?

Open a thread in [Discussions](https://github.com/gruberb/brag-frog/discussions). For bugs and feature requests, use the issue templates.
