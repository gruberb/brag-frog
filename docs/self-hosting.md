# Self-Hosting Guide

Brag Frog is a single Rust binary with an embedded SQLite database. No external services required beyond Google OAuth for authentication.

## Prerequisites

1. **Google OAuth 2.0 credentials** (for user authentication)
2. **An encryption key** (for encrypting API tokens at rest)

## Setting up Google OAuth 2.0

### 1. Create a Google Cloud project

- Go to [console.cloud.google.com](https://console.cloud.google.com)
- Create a new project (or use an existing one)
- No billing is required — OAuth is free

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

The `docker-compose.yml` mounts a persistent volume for the database and maps `./custom/` into the container.

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

The SQLite database is stored at `/data/bragfrog.db` inside the container.

## Bare metal

```bash
git clone https://github.com/gruberb/brag-frog.git && cd brag-frog
cargo build --release
```

Copy to your server:
- `target/release/brag-frog` (binary)
- `templates/` (HTML templates)
- `static/` (CSS, JS, images)
- `config/` (default config files)
- `migrations/` (database migrations)

Set environment variables and run:

```bash
export BRAGFROG_GOOGLE_CLIENT_ID=...
export BRAGFROG_GOOGLE_CLIENT_SECRET=...
export BRAGFROG_ENCRYPTION_KEY=...
export BRAGFROG_BASE_URL=https://your-domain.com
./brag-frog
```

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

Set `BRAGFROG_BASE_URL=https://bragfrog.example.com` so OAuth redirects use the correct URL.

## Backups

The entire application state lives in a single SQLite file (`bragfrog.db`). To back up:

```bash
# While the app is running (SQLite WAL mode supports this)
sqlite3 /path/to/bragfrog.db ".backup /path/to/backup.db"

# Or simply copy the file when the app is stopped
cp /path/to/bragfrog.db /path/to/backup.db
```

The encryption key (`BRAGFROG_ENCRYPTION_KEY`) must be backed up separately — without it, encrypted data in the database is unrecoverable.

## Updates

```bash
git pull origin main          # custom/ is gitignored, untouched
docker compose build          # rebuild
docker compose up -d          # migrations auto-apply on startup
```

## Environment variables reference

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `BRAGFROG_GOOGLE_CLIENT_ID` | Yes | — | Google OAuth client ID |
| `BRAGFROG_GOOGLE_CLIENT_SECRET` | Yes | — | Google OAuth client secret |
| `BRAGFROG_ENCRYPTION_KEY` | Yes | — | Base64-encoded 32-byte AES key |
| `BRAGFROG_INSTANCE_NAME` | No | — | Shows "Brag Frog \| {name}" on login page |
| `BRAGFROG_ALLOWED_DOMAIN` | No | — | Restrict sign-ups to an email domain |
| `BRAGFROG_PUBLIC_ONLY` | No | `false` | Only sync public/non-confidential data |
| `BRAGFROG_AI_MODEL` | No | `claude-sonnet-4-5-20250929` | Anthropic model for AI summaries |
| `BRAGFROG_BASE_URL` | No | `http://localhost:{port}` | Public URL for OAuth redirects |
| `BRAGFROG_HOST` | No | `0.0.0.0` | Bind address |
| `BRAGFROG_PORT` | No | `8080` | Listen port (also reads `PORT`) |
| `BRAGFROG_DATABASE_PATH` | No | `bragfrog.db` | Path to SQLite database |
