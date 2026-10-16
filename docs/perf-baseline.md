# Performance baseline

Compares the old server-rendered app with the current React + JSON API app. Numbers were not captured yet; the table is empty on purpose.

The server-rendered version (Rust + Tera + HTMX) is at git tag `v5.2.12`. Check it out in a separate worktree, build and run it on another port, and run the same loop against both:

```bash
git worktree add ../brag-frog-v5 v5.2.12
```

Method: log in, copy the `id` session cookie, then for each route run 20 requests and record p50 and p95. Measure the same database, the same cycle and the same machine for both versions.

```bash
for i in $(seq 20); do curl -s -o /dev/null -w '%{time_total}\n' -H "Cookie: id=$SESSION" "$BASE/path"; done | sort -n
```

For `v5.2.12`, request the HTML routes. For the current app, request the API endpoint that backs the view (the SPA shell itself is a static file). The current API needs no CSRF header for GET requests.

| View | Old route (`v5.2.12`) | Current endpoint |
|---|---|---|
| Dashboard / Today | `/dashboard` | `/api/v1/today` |
| Logbook | `/logbook` | `/api/v1/entries` |
| Logbook search | `/logbook/entries?q=x` | `/api/v1/entries?q=x` |
| Trends | `/trends` | `/api/v1/trends` |
| Review | `/review/{phase_id}` | `/api/v1/cycles/{id}/review` |

| View | Old local p50 | Old local p95 | Current local p50 | Current local p95 |
|---|---|---|---|---|
| Today | | | | |
| Logbook | | | | |
| Logbook search | | | | |
| Trends | | | | |
| Review | | | | |

Entries in the cycle when measured: _
