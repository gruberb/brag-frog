# Self-Hosting Guide

Brag Frog is one Rust binary (API) that also serves the built web app, with an embedded SQLite database. No external services required beyond Google OAuth for authentication.

## Prerequisites

1. **Google OAuth 2.0 credentials** (for user authentication)
2. **An encryption key** (for encrypting API tokens at rest)

## Setting up Google OAuth 2.0

### 1. Create a Google Cloud project

- Go to [console.cloud.google.com](https://console.cloud.google.com)
- Create a new project (or use an existing one)
- No billing is required: OAuth is free

### 2. Configure the OAuth consent screen

- Navigate to **APIs & Services > OAuth consent screen**
- Choose **External** (or **Internal** if restricting to a Google Workspace domain)
- Fill in:
  - **App name:** Brag Frog
  - **User support email:** your email
  - **Authorized domains:** your deployment domain
  - **Developer contact email:** your email
- Add scopes: `email`, `profile`, `openid`
- If using Google Drive or Calendar sync, also add:
  - `https://www.googleapis.com/auth/drive.activity.readonly`
  - `https://www.googleapis.com/auth/calendar.readonly`

### 3. Create OAuth 2.0 credentials

- Navigate to **APIs & Services > Credentials**
- Click **Create Credentials > OAuth client ID**
- Application type: **Web application**
- **Authorized redirect URIs:**
  - Local: `http://localhost:8080/auth/callback`
  - Production: `https://your-domain.com/auth/callback`
- Note the **Client ID** and **Client Secret**

### 4. Generate an encryption key

```bash
openssl rand -base64 32
```

Save this key securely. It encrypts all API tokens stored in the database. If you lose it, users will need to re-enter their integration tokens.

---

## Docker Compose (recommended)

```bash
git clone https://github.com/gruberb/brag-frog.git && cd brag-frog
cp .env.example .env    # edit with your credentials
mkdir -p custom/        # optional: add config overrides
docker compose up -d
```

The `docker-compose.yml` builds the image from the `Dockerfile`, mounts a persistent volume for the database at `/data` and maps `./custom/` to `/app/custom`.

## Docker

```bash
docker build -t brag-frog .
docker run -p 8080:8080 \
  -e BRAGFROG_GOOGLE_CLIENT_ID=your-client-id \
  -e BRAGFROG_GOOGLE_CLIENT_SECRET=your-client-secret \
  -e BRAGFROG_ENCRYPTION_KEY=your-base64-key \
  -e BRAGFROG_BASE_URL=https://your-domain.com \
  -v bragfrog-data:/data \
  -v ./custom:/app/custom \
  brag-frog
```

The Dockerfile has three stages: `node:24-alpine` builds the web app, `rust:1.93-slim` builds the API, and `debian:trixie-slim` runs it. The image sets `BRAGFROG_WEB_DIST=/app/web`, `BRAGFROG_CONFIG_DIR=/app/config`, `BRAGFROG_CUSTOM_DIR=/app/custom` and `BRAGFROG_DATABASE_PATH=/data/bragfrog.db`, and listens on port 8080.

### Prebuilt image

Pushing a `v*` tag runs `.github/workflows/release.yml`, which publishes the image to GitHub Container Registry:

```bash
docker pull ghcr.io/gruberb/brag-frog:latest   # or a version tag
```

Use it in place of `brag-frog` in the `docker run` command above.

## Bare metal

Requirements on the build machine: Rust toolchain, Node 24, npm.

```bash
git clone https://github.com/gruberb/brag-frog.git && cd brag-frog
npm ci
npm run build -w apps/web           # writes apps/web/dist
cargo build --release -p brag-frog  # writes target/release/brag-frog
```

Migrations are compiled into the binary. Copy to your server:
- `target/release/brag-frog` (binary)
- `apps/web/dist/` (built web app)
- `apps/api/config/` (default config files)

Set environment variables and run:

```bash
export BRAGFROG_GOOGLE_CLIENT_ID=...
export BRAGFROG_GOOGLE_CLIENT_SECRET=...
export BRAGFROG_ENCRYPTION_KEY=...
export BRAGFROG_BASE_URL=https://your-domain.com
export BRAGFROG_WEB_DIST=/opt/brag-frog/web          # the copied dist/
export BRAGFROG_CONFIG_DIR=/opt/brag-frog/config
export BRAGFROG_CUSTOM_DIR=/opt/brag-frog/custom     # optional overrides
./brag-frog
```

The three directory variables default to `config`, `custom` and `../web/dist` relative to the working directory, so set them explicitly unless you run from `apps/api`.

## Health check

`GET /healthz` returns `200 ok` without touching the database. Use it for load balancer, Kubernetes or Cloud Run liveness probes. The image has no `curl`, so run container-level checks from outside or add a probe in your orchestrator.

## Fly.io

```bash
cd brag-frog
fly launch --no-deploy
```

Edit the generated `fly.toml`:

```toml
app = "brag-frog"
primary_region = "iad"

[env]
  BRAGFROG_HOST = "0.0.0.0"
  BRAGFROG_PORT = "8080"
  BRAGFROG_DATABASE_PATH = "/data/bragfrog.db"
  BRAGFROG_BASE_URL = "https://brag-frog.fly.dev"

[http_service]
  internal_port = 8080
  force_https = true
  auto_stop_machines = "stop"
  auto_start_machines = true

[mounts]
  source = "bragfrog_data"
  destination = "/data"

[[http_service.checks]]
  method = "GET"
  path = "/healthz"
  interval = "30s"
  timeout = "5s"
```

```bash
fly volumes create bragfrog_data --region iad --size 1
fly secrets set \
  BRAGFROG_GOOGLE_CLIENT_ID=... \
  BRAGFROG_GOOGLE_CLIENT_SECRET=... \
  BRAGFROG_ENCRYPTION_KEY=...
fly deploy
```

Add `https://brag-frog.fly.dev/auth/callback` to your Google OAuth redirect URIs.

---

## Restricting sign-ups

To limit access to your organization's Google Workspace domain:

```env
BRAGFROG_ALLOWED_DOMAIN=your-company.com
```

Only users with `@your-company.com` email addresses can sign in.

## Reverse proxy

If running behind nginx or similar:

```nginx
server {
    listen 443 ssl;
    server_name bragfrog.example.com;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

Set `BRAGFROG_BASE_URL=https://bragfrog.example.com` so OAuth redirects use the correct URL. With an https base URL the session cookie is marked `Secure`. The web app and the API share one origin, so a single `location /` is enough; no separate static file serving or CORS is needed.

## Backups

The entire application state lives in a single SQLite file (`bragfrog.db`). To back up:

```bash
# While the app is running (SQLite WAL mode supports this)
sqlite3 /path/to/bragfrog.db ".backup /path/to/backup.db"

# Or simply copy the file when the app is stopped
cp /path/to/bragfrog.db /path/to/backup.db
```

The encryption key (`BRAGFROG_ENCRYPTION_KEY`) must be backed up separately: without it, encrypted data in the database is unrecoverable.

## Updates

```bash
git pull origin main          # custom/ is gitignored, untouched
docker compose build          # rebuild (web app + API)
docker compose up -d          # migrations auto-apply on startup
```

Or pull a newer `ghcr.io/gruberb/brag-frog` tag. Back up `bragfrog.db` before upgrading. The browser picks up a new release on the next load because `index.html` is served with `no-cache` and the hashed assets are immutable.

## Environment variables reference

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `BRAGFROG_GOOGLE_CLIENT_ID` | Yes | none | Google OAuth client ID |
| `BRAGFROG_GOOGLE_CLIENT_SECRET` | Yes | none | Google OAuth client secret |
| `BRAGFROG_ENCRYPTION_KEY` | Yes | none | Base64-encoded 32-byte AES key |
| `BRAGFROG_INSTANCE_NAME` | No | none | Instance name shown on the sign-in page |
| `BRAGFROG_ALLOWED_DOMAIN` | No | none | Restrict sign-ups to an email domain |
| `BRAGFROG_PUBLIC_ONLY` | No | `false` | Only sync public/non-confidential data |
| `BRAGFROG_AI_MODEL` | No | `claude-opus-4-7` | Anthropic model for AI summaries |
| `BRAGFROG_BASE_URL` | No | `http://localhost:{port}` | Public URL for OAuth redirects |
| `BRAGFROG_HOST` | No | `0.0.0.0` | Bind address |
| `BRAGFROG_PORT` | No | `8080` | Listen port (also reads `PORT`) |
| `BRAGFROG_DATABASE_PATH` | No | `bragfrog.db` | Path to SQLite database |
| `BRAGFROG_CONFIG_DIR` | No | `config` | Directory with the default TOML configs |
| `BRAGFROG_CUSTOM_DIR` | No | `custom` | Directory with operator overrides (TOML files; only `tokens.css` is served) |
| `BRAGFROG_WEB_DIST` | No | `../web/dist` | Directory with the built web app (`index.html`, `assets/`) |
| `BRAGFROG_GOOGLE_REDIRECT_URI` | No | `{base_url}/auth/callback` | OAuth redirect URI |
| `SQLITE_JOURNAL_MODE` | No | `wal` | SQLite journal mode; use `delete` for FUSE/NFS-backed volumes |
