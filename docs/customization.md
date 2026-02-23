# Customization Guide

Brag Frog is designed to be customized for any organization without modifying source code. All customization happens through config files and environment variables.

## The `custom/` overlay

Create a `custom/` directory in the project root. On startup, the app checks `custom/` first, then falls back to `config/`. The `custom/` directory is gitignored, so your overrides survive `git pull`.

```
custom/
  clg_levels.toml          # Career ladder levels
  review_sections.toml     # Performance review sections & AI prompts
  services.toml            # Service URLs, default orgs, sync filters
  tokens.css               # Brand colors, fonts, CSS variables
  fonts/                   # Custom web fonts (.woff2)
```

Only include files you want to override — missing files fall through to `config/` defaults.

## Career levels (`clg_levels.toml`)

Define your organization's IC career ladder. This populates the Level Guide page.

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

See `config/clg_levels.toml` for the full format.

## Review sections (`review_sections.toml`)

Configure what sections appear in the AI-generated self-review and the prompts used to generate them. This maps to your review platform (CultureAmp, Lattice, 15Five, etc.).

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

See `config/review_sections.toml` for the full format.

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

The `allowed_*` fields restrict what gets synced org-wide. Empty arrays (the default) mean no restriction — users can sync everything they have access to.

| Field | Service | Effect |
|-------|---------|--------|
| `allowed_orgs` | GitHub | Only sync PRs from these GitHub organizations |
| `allowed_projects` | Phabricator | Post-filter revisions by project tag |
| `allowed_products` | Bugzilla | Only query these Bugzilla products |
| `allowed_jira_projects` | Jira | Add `project in (...)` to JQL |
| `allowed_confluence_spaces` | Confluence | Add `space in (...)` to CQL |

## Custom branding (`tokens.css`)

Override CSS design tokens to match your brand. Create `custom/tokens.css`:

```css
:root {
    --color-primary: #0066cc;
    --color-primary-hover: #0052a3;
    --font-stack: 'Your Font', system-ui, sans-serif;
    --font-stack-heading: 'Your Font', system-ui, sans-serif;
}
```

The custom CSS is loaded after the default `tokens.css`, so any variables you define will override the defaults.

## Custom fonts

1. Place `.woff2` files in `custom/fonts/`
2. Reference them in `custom/tokens.css`:

```css
@font-face {
    font-family: 'Your Font';
    src: url('/custom/fonts/YourFont-Variable.woff2') format('woff2');
    font-weight: 100 900;
    font-display: swap;
}

:root {
    --font-stack: 'Your Font', system-ui, sans-serif;
    --font-stack-heading: 'Your Font', system-ui, sans-serif;
}
```

## Instance branding

Set `BRAGFOX_INSTANCE_NAME` to show your company name on the login page:

```env
BRAGFOX_INSTANCE_NAME=Acme Corp
```

This displays "Brag Frog | Acme Corp" in the navigation bar.

## Public-only mode

Set `BRAGFOX_PUBLIC_ONLY=true` to restrict all sync services to public/non-confidential data:

- **GitHub:** Only syncs from public repos
- **Bugzilla:** Strips API token, only public bugs returned
- **Phabricator:** Filters out revisions with restricted view policies
- **Jira:** Adds `AND "Security Level" is EMPTY` to JQL
- **Confluence:** Relies on token permissions (no additional filtering)

## Getting updates

```bash
git pull origin main          # custom/ is gitignored, untouched
docker compose build          # rebuild
docker compose up -d          # migrations auto-apply on startup
```

New config keys added to `config/*.toml` are handled gracefully — your `custom/` overlay takes precedence for keys it defines; new keys fall through to defaults.
