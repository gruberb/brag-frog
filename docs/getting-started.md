# Getting Started

A developer guide for setting up, running, and contributing to Brag Frog.

## Prerequisites

- **Rust** (edition 2024) — install via [rustup.rs](https://rustup.rs/)
- **Google OAuth 2.0 credentials** — [Google Cloud Console](https://console.cloud.google.com/apis/credentials), create an OAuth 2.0 Client ID (Web application). Add `http://localhost:8080/auth/callback` as an authorized redirect URI.
- **openssl** (for generating the encryption key)

## Setup

```bash
git clone https://github.com/gruberb/brag-frog.git
cd brag-frog
cp .env.example .env
```

Edit `.env` with your credentials:

```env
BRAGFROG_GOOGLE_CLIENT_ID=your-client-id
BRAGFROG_GOOGLE_CLIENT_SECRET=your-client-secret
BRAGFROG_ENCRYPTION_KEY=$(openssl rand -base64 32)
```

Then run:

```bash
cargo run
# → http://localhost:8080
```

The SQLite database (`bragfrog.db`) is auto-created on first run. All migrations apply automatically.

## Running Tests

```bash
cargo test
```

Tests use in-memory SQLite with the full schema applied via `db::run_migrations`. No external services needed.

- **Unit tests** (`#[cfg(test)]` modules): `crypto.rs`, `models/mod.rs`, `models/entry.rs`, `clg.rs`
- **Integration tests** (`tests/db_tests.rs`): entry CRUD, soft-delete, phase cascade, week find-or-create, key result hierarchy

## First Steps in the App

1. **Sign in** with Google at `http://localhost:8080`
2. **Create a phase** (performance review cycle) — e.g., "H1 2026" with start/end dates
3. **Add manual entries** via the logbook, or set up integrations to sync from GitHub, Jira, etc.
4. **Set up integrations** on the Integrations page — each service needs its own API token
5. **Sync** to pull work items from configured services into the active phase
6. **Create goals and key results**, then link entries to them
7. **Generate AI self-review drafts** (requires an Anthropic API key in integrations)

## Project Structure Overview

```
src/           Rust source (lib.rs + main.rs dual-crate)
config/        Default TOML config files (career levels, review sections, services)
custom/        Org-specific config overrides (gitignored, checked first on startup)
templates/     Tera HTML templates (pages/ for full pages, components/ for HTMX partials)
static/        CSS, JS, images (no build step)
migrations/    Numbered SQL migration files
tests/         Integration tests
docs/          Documentation
```

For the full module map, see [architecture.md](architecture.md).

## Common Dev Tasks

### Adding a New Route

1. Create the handler function in the appropriate `src/routes/*.rs` file
2. Add the route to `src/routes/mod.rs` in `create_router()` — protected routes go in the `protected_routes` group
3. Use `AuthUser` as the first extractor parameter (before `State(state)`) for protected routes
4. Return `Result<Html<String>, AppError>` — render a template or return an HTML fragment
5. Create the template in `templates/pages/` (full page) or `templates/components/` (HTMX partial)

```rust
pub async fn my_page(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let mut ctx = tera::Context::new();
    let user = User::find_by_id(&state.db, auth.user_id).await?;
    ctx.insert("user", &user);
    ctx.insert("current_page", "my-page");
    let html = state.templates.render("pages/my_page.html", &ctx)?;
    Ok(Html(html))
}
```

### Adding a New Migration

1. Create `migrations/NNN_description.sql` with plain SQL statements separated by `;`
2. In `src/db.rs`, add an idempotent check + execution block:
   - Check: test for the specific change (e.g., column existence via `pragma_table_info`)
   - Execute: `include_str!` the migration file, split by `;`, execute each statement
3. **Never modify existing migration files** — especially `001_initial.sql`
4. Integration tests pick up new migrations automatically

### Adding a New Template

- **Full page:** Create `templates/pages/my_page.html`, extend `base.html`
- **HTMX partial:** Create `templates/components/my_component.html` (no base layout)
- Template path in `render()` must match the file path relative to `templates/`
- Templates are loaded once at startup — restart the server to pick up new files

### Adding a New Sync Service

1. Create `src/sync/my_service.rs` implementing the `SyncService` trait:
   - `sync()` — fetch work items within a date range, return `Vec<SyncedEntry>`
   - `test_connection()` — verify credentials, return `ConnectionStatus`
2. Add the module to `src/sync/mod.rs`
3. Add a match arm in `get_sync_service()` to map the service name to your implementation
4. Add integration UI in `templates/components/` and register routes in `routes/integrations.rs`
5. Optionally add service defaults to `config/services.toml`

### Customizing Config

Create a `custom/` directory and place your overridden TOML/CSS files there. The app checks `custom/` first, then falls back to `config/`. See [customization.md](customization.md) for details.

## Further Reading

- [Architecture](architecture.md) — module map, request lifecycle, database design, sync architecture
- [Self-Hosting](self-hosting.md) — Docker, bare metal, Fly.io deployment
- [Customization](customization.md) — config overlays, branding, sync filters
