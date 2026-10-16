# Getting Started

A developer guide for setting up, running, and contributing to Brag Frog.

## Prerequisites

- **moon** ([moonrepo.dev](https://moonrepo.dev/docs/install)): task runner. `.prototools` pins moon and Node 24.
- **Node 24** and npm (moon's proto installs the pinned version, or install it yourself)
- **Rust** (edition 2024) with clippy: install via [rustup.rs](https://rustup.rs/)
- **Google OAuth 2.0 credentials**: [Google Cloud Console](https://console.cloud.google.com/apis/credentials), create an OAuth 2.0 Client ID (Web application). Add `http://localhost:8080/auth/callback` as an authorized redirect URI.
- **openssl** (for generating the encryption key)

## Setup

```bash
git clone https://github.com/gruberb/brag-frog.git
cd brag-frog
cp .env.example .env
npm ci
```

Edit `.env` with your credentials:

```env
BRAGFROG_GOOGLE_CLIENT_ID=your-client-id
BRAGFROG_GOOGLE_CLIENT_SECRET=your-client-secret
BRAGFROG_ENCRYPTION_KEY=<output of: openssl rand -base64 32>
```

Then start both apps (two terminals):

```bash
moon run api:dev    # API on http://localhost:8080
moon run web:dev    # web app on http://localhost:5173, proxies /api and /auth to :8080
```

Open `http://localhost:5173`. The sign-in redirect goes through the proxy, so cookies and OAuth stay same-origin. `BRAGFROG_API_URL` changes the proxy target.

moon runs the API from `apps/api`, so the SQLite database (`bragfrog.db`) is created there on first run and `config/` resolves to `apps/api/config`. All migrations apply automatically. `.env` at the repo root is found by walking up from the working directory.

To try the UI without a backend, run `npm run dev:mock` in `apps/web` (in-memory MSW data).

To serve the built web app from the API instead of Vite:

```bash
moon run web:build   # writes apps/web/dist
moon run api:dev     # default BRAGFROG_WEB_DIST=../web/dist, open http://localhost:8080
```

## Running Tests, Lint and Build

```bash
moon run :lint :test :build   # all projects, same as CI (`moon ci`)
moon run api:test             # cargo test, in-memory SQLite
moon run api:lint             # clippy -D warnings
moon run web:test             # Vitest + Testing Library + MSW
moon run web:lint             # ESLint, includes import boundaries
moon run web:typecheck
```

- **API unit tests** (`#[cfg(test)]` modules): crypto, entry types, config overlay, `/custom` handling
- **API integration tests** (`apps/api/tests/integration`): HTTP tests per area (`api_*_tests.rs`), `db_tests.rs`, `sync_tests.rs`. They run all migrations through `db::run_migrations`.
- **Web tests**: next to the code (`*.test.ts(x)`), route tests in `src/app/routes`

## First Steps in the App

1. **Sign in** with Google
2. **Create a cycle** (performance review period) in Settings, Cycles, e.g. "H1 2026" with start/end dates, and make it the active cycle
3. **Add manual entries** with the composer on Today or the Logbook, or set up integrations to sync from GitHub, Jira, etc.
4. **Set up integrations** in Settings, Integrations. Each token-based service needs its own API token; Google Drive and Calendar use the Google connect flow.
5. **Sync** to pull work items from configured services into the active cycle
6. **Create department goals and priorities**, then link entries to them
7. **Write the self-review** in Review, optionally with AI drafts (requires an Anthropic API key in Settings, AI)

## Project Structure Overview

```
apps/api/            Rust crate `brag-frog` (lib + bin)
  src/               api/ (JSON handlers), identity/, worklog/, objectives/, cycle/, review/, integrations/, ai/, kernel/, db/
  config/            Default TOML config (career levels, review sections, services)
  migrations/        Numbered SQL migration files
  tests/             Integration tests
apps/web/            React app (see apps/web/README.md)
  src/               app/, features/, components/, hooks/, lib/, stores/, utils/, types/api (generated)
custom/              Operator overrides (gitignored, checked first on startup)
.moon/               moon workspace config
docs/                Documentation
```

For the full module map, see [architecture.md](architecture.md).

## Common Dev Tasks

### Adding an API Endpoint

1. Create the handler in the matching file under `apps/api/src/api/` (or a new file declared in `api/mod.rs`)
2. Register the route in `router()` in `apps/api/src/api/mod.rs`. It is then behind the CSRF header check and the session check.
3. Use `ApiAuth` as the first extractor (before `State(state)`) and return `ApiResult<T>`
4. Define request and response types with `#[derive(Serialize, TS)] #[ts(export)]`, then run `moon run api:types` and commit the generated files in `apps/web/src/types/api/`
5. Add an integration test in `apps/api/tests/integration/`

```rust
pub async fn my_endpoint(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
) -> ApiResult<MyResponse> {
    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("User not found"))?;
    Ok(Json(MyResponse { name: user.name }))
}
```

### Adding a Feature in the Web App

1. Create `apps/web/src/features/<name>/` with `api/` (TanStack Query hooks, query keys next to them) and `components/`
2. Use the generated types from `@/types/api`. Do not edit them by hand.
3. Do not import from another feature. If two features must work together, compose them in a route under `apps/web/src/app/routes/` and pass components or callbacks as props.
4. Add the route in `apps/web/src/app/router.tsx` and its path in `src/config/paths.ts`
5. Add MSW handlers in `src/testing/mocks` and a test next to the route
6. Run `moon run web:lint web:typecheck web:test`

### Adding a New Migration

1. Create `apps/api/migrations/NNN_description.sql` (next number after `025_reports.sql`) with plain SQL statements
2. Register it in the `INCREMENTAL_MIGRATIONS` array in `apps/api/src/db/mod.rs` (`(id, include_str!(...))`)
3. **Never modify existing migration files**, especially `001_initial.sql`
4. Integration tests pick up new migrations automatically

### Adding a New Sync Service

1. Create `apps/api/src/integrations/my_service.rs` implementing the `SyncService` trait:
   - `sync()`: fetch work items within a date range, return `Vec<SyncedEntry>`
   - `test_connection()`: verify credentials, return `ConnectionStatus`
2. Declare the module in `apps/api/src/integrations/mod.rs`
3. Add a match arm in `get_sync_service()` (`factory.rs`) to map the service name to your implementation
4. Add the service's labels, settings fields and source names in `integrations/catalog.rs`. The web app renders the configure form from the `fields` the API returns, so no web change is needed for plain text and checkbox fields.
5. Optionally add service defaults to `apps/api/config/services.toml`

### Customizing Config

Create a `custom/` directory and place your overridden TOML/CSS files there. The API checks `custom/` first, then falls back to `config/`. See [customization.md](customization.md) for details.

## Further Reading

- [Architecture](architecture.md): module map, request lifecycle, database design, sync architecture, web app
- [Self-Hosting](self-hosting.md): Docker, bare metal, Fly.io deployment
- [Customization](customization.md): config overlays, branding, sync filters
