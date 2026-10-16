# Customization Guide

Brag Frog is designed to be customized for any organization without modifying source code. All customization happens through config files and environment variables.

## The `custom/` overlay

The API reads two directories, set by `BRAGFROG_CUSTOM_DIR` (default `custom`) and `BRAGFROG_CONFIG_DIR` (default `config`). Both are relative to the API's working directory: `apps/api/` under `moon run api:dev`, `/app` in the Docker image (where they are preset to `/app/custom` and `/app/config`). On startup the API checks the custom directory first, then falls back to the config directory (`apps/api/config/` in the repo). The custom directory is gitignored, so your overrides survive `git pull`.

```
custom/
  clg_levels.toml          # Career ladder levels
  review_sections.toml     # Performance review sections & AI prompts
  services.toml            # Service URLs, default orgs, sync filters
  tokens.css               # Brand colors, fonts, CSS variables
  fonts/                   # Custom web fonts (.woff2)
```

Only include files you want to override. Missing files fall through to the defaults in `config/`. Config changes need an API restart.

The overlay supports exactly three TOML files plus `tokens.css`. Other files in `custom/` are ignored, and only `tokens.css` is served to the browser (as `/custom/tokens.css`); the TOML files may contain AI prompts and are never exposed over HTTP. `custom/fonts/` is not served either, see "Custom fonts" below.

## Career levels (`clg_levels.toml`)

Define your organization's IC career ladder. The levels feed the career level drawer in the Review view, the role choices in Settings (profile), and the level context in AI self-review prompts.

```toml
[[levels]]
title = "Software Engineer I"
code = "IC1"
description = "Develops features with guidance..."
competencies = [
    { name = "Technical Skills", description = "Writes clean code..." },
    { name = "Collaboration", description = "Works effectively in a team..." },
]

[[levels]]
title = "Software Engineer II"
code = "IC2"
# ...
```

See `apps/api/config/clg_levels.toml` for the full format.

## Review sections (`review_sections.toml`)

Configure which sections appear in the Review view (one document editor per section) and the prompts used for AI drafts. This maps to your review platform (Lattice, 15Five, etc.).

```toml
[[sections]]
slug = "impact"
title = "Impact & Results"
question = "What were your most significant contributions this cycle?"
ai_prompt = "Based on the engineer's logged work, summarize their key contributions and measurable impact..."

[[sections]]
slug = "growth"
title = "Growth & Development"
question = "How have you grown professionally?"
ai_prompt = "Identify areas of professional growth..."
```

See `apps/api/config/review_sections.toml` for the full format.

## Service defaults (`services.toml`)

Configure default URLs, placeholders, and sync filters for each integration.

```toml
[github]
default_orgs = "my-org"
org_placeholder = "my-org, my-org-infra"
token_url = "https://github.com/settings/tokens/new"
allowed_orgs = ["my-org", "my-org-infra"]  # Only sync from these orgs

[phabricator]
default_base_url = "https://phabricator.mycompany.com"
base_url_placeholder = "https://phabricator.mycompany.com"
allowed_projects = ["PROJ", "INFRA"]  # Only sync these project codes

[bugzilla]
default_base_url = "https://bugzilla.mycompany.com"
base_url_placeholder = "https://bugzilla.mycompany.com"
email_placeholder = "you@mycompany.com"
allowed_products = ["MyProduct", "Core"]  # Only sync these products

[atlassian]
default_base_url = "https://mycompany.atlassian.net"
base_url_placeholder = "https://mycompany.atlassian.net"
email_placeholder = "you@mycompany.com"
token_url = "https://id.atlassian.com/manage-profile/security/api-tokens"
allowed_jira_projects = ["ENG", "INFRA"]    # Only sync these Jira projects
allowed_confluence_spaces = ["ENG", "TEAM"] # Only sync these Confluence spaces

[claude]
token_url = "https://console.anthropic.com/settings/keys"
```

### Sync filters

The `allowed_*` fields restrict what gets synced org-wide. Empty arrays (the default) mean no restriction: users can sync everything they have access to.

| Field | Service | Effect |
|-------|---------|--------|
| `allowed_orgs` | GitHub | Only sync PRs from these GitHub organizations |
| `allowed_projects` | Phabricator | Post-filter revisions by project tag |
| `allowed_products` | Bugzilla | Only query these Bugzilla products |
| `allowed_jira_projects` | Jira | Add `project in (...)` to JQL |
| `allowed_confluence_spaces` | Confluence | Add `space in (...)` to CQL |

## Custom branding (`tokens.css`)

Override the design tokens to match your brand. Create `custom/tokens.css`:

```css
:root {
    --color-primary: #0066cc;
    --color-primary-hover: #0052a3;
    --font-stack: 'Your Font', system-ui, sans-serif;
    --font-stack-heading: 'Your Font', system-ui, sans-serif;
}
```

The web app links `/custom/tokens.css` after its own stylesheet, so any variable you define overrides the default. The defaults are in `apps/web/src/app/app.css` (Tailwind `@theme static`, so every token is emitted as a CSS variable); check that file for the variable names. A missing `tokens.css` is fine, the API returns an empty stylesheet.

This is a per-deployment override. In the repository itself, brand colours and the Mozilla fonts are fixed.

## Custom fonts

The custom directory serves only `tokens.css`, so font files cannot be loaded from `custom/fonts/`. Either:

- reference fonts hosted elsewhere in `custom/tokens.css`, or
- rebuild the web app with your `.woff2` files in `apps/web/public/fonts/` (served at `/fonts/`).

Content-Security-Policy is `font-src 'self'`, so external font hosts are blocked unless you change `security_headers` in `apps/api/src/kernel/middleware.rs`.

```css
@font-face {
    font-family: 'Your Font';
    src: url('/fonts/YourFont-Variable.woff2') format('woff2');
    font-weight: 100 900;
    font-display: swap;
}

:root {
    --font-stack: 'Your Font', system-ui, sans-serif;
    --font-stack-heading: 'Your Font', system-ui, sans-serif;
}
```

## Instance branding

Set `BRAGFROG_INSTANCE_NAME` to show your company name on the sign-in page:

```env
BRAGFROG_INSTANCE_NAME=Acme Corp
```

This replaces "Brag Frog" in the footer line of the sign-in page with "Acme Corp".

## Public-only mode

Set `BRAGFROG_PUBLIC_ONLY=true` to restrict all sync services to public/non-confidential data:

- **GitHub:** Only syncs from public repos
- **Bugzilla:** Strips API token, only public bugs returned
- **Phabricator:** Filters out revisions with restricted view policies
- **Jira:** Adds `AND "Security Level" is EMPTY` to JQL
- **Confluence:** Relies on token permissions (no additional filtering)

## Getting updates

```bash
git pull origin main          # custom/ is gitignored, untouched
docker compose build          # rebuild (web app + API)
docker compose up -d          # migrations auto-apply on startup
```

New config keys added to `apps/api/config/*.toml` are handled gracefully. Your `custom/` overlay takes precedence for files it defines (a custom file replaces the default file as a whole), so check the defaults after an update when you override one.
