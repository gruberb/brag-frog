# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in Brag Frog, please report it responsibly.

**Email:** [bastian@gruber.dev](mailto:bastian@gruber.dev)

Please include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if you have one)

**Do not** open a public issue for security vulnerabilities.

## Response Timeline

- **Acknowledgment:** within 48 hours
- **Initial assessment:** within 1 week
- **Fix or mitigation:** as soon as practical, depending on severity

## Scope

The following areas are particularly sensitive and in scope:

- **Encryption** — AES-256-GCM encryption of stored data (titles, descriptions, tokens, AI content)
- **Authentication** — Google OAuth flow, session management
- **SSRF** — User-supplied service URLs (Phabricator, Bugzilla, Atlassian base URLs)
- **CSRF** — State-changing requests and the `HX-Request` header check
- **Token storage** — Integration tokens (GitHub PAT, Atlassian, Bugzilla API keys)
- **Data leakage** — Unintended exposure of encrypted fields or cross-user data access

## Out of Scope

- Vulnerabilities in dependencies (report those upstream, but let us know so we can update)
- Denial of service against self-hosted instances you control
- Social engineering

## Supported Versions

| Version | Supported |
|---------|-----------|
| 1.0.x   | Yes       |
