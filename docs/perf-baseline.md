# Performance baseline

Fill this in before the SSR removal (see implementation.md, M0 step 5), then again after M5 and M7.

Method: log in, copy the `id` session cookie, then for each route run 20 requests and record p50 and p95:

```bash
for i in $(seq 20); do curl -s -o /dev/null -w '%{time_total}\n' -H "Cookie: id=$SESSION" "$BASE/dashboard"; done | sort -n
```

Routes: `/dashboard`, `/logbook`, `/logbook/entries?search=x`, `/trends`, `/review/{phase_id}`.

| Route | Local p50 | Local p95 | Cloud Run p50 | Cloud Run p95 |
|---|---|---|---|---|
| /dashboard | | | | |
| /logbook | | | | |
| /logbook/entries?search=x | | | | |
| /trends | | | | |
| /review/{phase_id} | | | | |

Entries in the phase when measured: _
