# Changelog

All notable changes to Brag Frog will be documented in this file.

## [5.2.9] - 2026-05-27

### Changed
- **Review page enforces one Lattice answer.** The Review page now renders only the primary configured review section and uses selectable department-goal cards instead of a native multi-select box.
- **Lattice self-review prompt matches past review quality better.** Drafts now avoid generated-looking headings, use ticket IDs only as support, emphasize stakeholder impact, and weave CLG evidence into the narrative instead of producing rubric sections.
- **Review drafts render Markdown with Jira links.** The self-review answer now previews rendered Markdown and links known Jira issue keys from synced work items.
- **AI drafts default to Claude Opus 4.7.** `BRAGFROG_AI_MODEL` now falls back to `claude-opus-4-7` when no environment override is set.

## [5.2.8] - 2026-05-27

### Changed
- **Review page now mirrors the single Lattice answer flow.** Removed the Q1 check-in branch from the Review page and replaced section cards with one department-goal selector followed by one editable self-review textbox.

## [5.2.7] - 2026-05-27

### Changed
- **Review page simplified around Lattice.** The page now uses a Lattice-style menu, self-review workspace, and overview column. Review config defaults to the single contribution-and-impact self-review question.
- **Quarterly check-in AI drafts now use work items directly.** Drafts pull from logged work in the selected quarter instead of weekly reflection rollups.
- **Reports count completed Jira work by terminal status.** The Trends report now treats Jira entries with terminal statuses such as Done, Closed, Resolved, and Verified as closed work even when their entry type is not `jira_completed`.

### Removed
- **10x Protocol and legacy reflections.** Removed the `/protocol`, `/checkins`, `/checkin/{week}`, and monthly check-in surfaces, their Rust modules/templates, and their persistence tables via migration 023.

## [5.2.5] - 2026-05-14

### Fixed
- **Department goal focus now scopes AI evidence.** When a Self Review section has selected department goals, AI drafts now receive only those goals, their linked priorities, linked logbook entries, and contribution examples tied to those entries. This prevents unrelated high-signal work from other goals from leaking into generated drafts.

## [5.2.4] - 2026-05-14

### Changed
- **Department-goal-focused AI drafts.** Self Review focus controls now select department goals instead of individual priorities. Generated drafts receive the selected department goals plus their linked priorities and evidence as the primary scope.
- **Goal Outcomes grouped by department goal.** The default Lattice prompt and Mozilla overlay now ask AI to produce goal-outcome drafts around department goals, using linked priorities, contribution examples, logbook evidence, and impact signals as supporting detail.

## [5.2.3] - 2026-05-12

### Added
- **Lattice-style self-review answer surface.** Review sections can now render platform prompt metadata, including question number, required marker, guidance bullets, tip text, and answer placeholder, so Brag Frog can mirror the actual Lattice prompt while still generating from saved work.
- **Priority-focused AI drafts.** Review sections can opt into priority selection. The Lattice contribution examples and goal outcomes sections now let users choose which priorities to emphasize before generating a per-section draft.

## [5.2.2] - 2026-05-12

### Added
- **Lattice contribution and impact prompt.** Self Review now includes a dedicated "Contribution & Impact Examples" section for Lattice's 1-2 example question, with prompt guidance for outcomes, CLG behaviors, learnings, and next development steps.
- **Contribution example context in AI drafts.** Self Review AI generation now includes saved contribution examples, impact metadata, learnings, and linked logbook evidence so generated drafts can prioritize the strongest examples instead of relying only on raw entries and priorities.

### Changed
- Review pages now auto-select the first review quarter when the cycle includes both check-in and review quarters, so users land on the self-review sections instead of Q1 conversation prep.

## [5.2.0] - 2026-05-06

### Added
- **AI Draft for quarterly check-ins.** The "AI Draft" button on each section of the quarterly conversation prep was a stub (`disabled`, "AI Draft coming soon"). Now wired end-to-end: a per-section synthesis pulls the matching slice of every weekly reflection in the quarter plus brag entries inside the quarter's calendar window, runs them through the section's `ai_prompt` instruction from `checkin_sections.toml`, and drops the plain-text result into the textarea for the user to edit and save. Same no-persistence pattern as the Self Review's `ai_draft_section` — the draft only sticks if the user clicks Save.
- **`POST /quarterly-checkin/{quarter}/{year}/ai-draft/{section}`.** New route handled by `reflections::routes::checkins::ai_draft_quarterly_section`. Path params drive section/quarter selection; CSRF-gated by the standard `HX-Request` header check.
- **`build_quarterly_checkin_prompt` in `ai::prompts`.** Assembles the per-section prompt: question, weekly reflections (filtered to the field this section synthesises), and a capped list of brag entries from the quarter date range for concrete anchoring.

### Changed
- `templates/panels/quarterly_checkin_form.html` and `templates/pages/quarterly_checkin.html` enable the AI Draft button when the user has Claude configured. A small `fetchQuarterlyAiDraft` helper handles the fetch, button-disable-during-request, and inline status text on error.

## [5.1.2] - 2026-05-06

### Fixed
- **OAuth state expiring during Google consent screen.** The 10-minute freshness window introduced in v5.1.1 was too tight for real users — anyone slow on the account picker, prompted for 2FA, reading the scopes carefully, or hitting a network stall got "OAuth state token expired". Window raised to 30 minutes. The CSRF posture is unchanged: Google's authorization `code` is itself single-use and expires within ~10 minutes, so a leaked state token without a matching fresh code is inert.

## [5.1.1] - 2026-05-05

### Fixed
- **"Invalid OAuth state" after Google sign-in.** The CSRF state token used during the OAuth round-trip was kept in a single session-keyed slot. Any second render of the landing/login page — a duplicate tab, a refresh, the back button after starting OAuth, a link prefetcher hitting `/` — overwrote the in-flight token, so the callback's comparison failed even though the user did nothing wrong. The token is now a stateless HMAC-signed value (`<flow>:<nonce>:<ts>:<sig>`, 10-minute validity) verified locally against an HKDF-derived key from `BRAGFROG_ENCRYPTION_KEY`. No session writes on the initiator path; concurrent tabs and reloads can no longer invalidate each other.

### Added
- **`identity::oauth_state` module.** `mint(crypto, flow)` / `verify(crypto, token) → OAuthFlow`. Domain-separated HMAC-SHA256 key (HKDF info `brag-frog:oauth-state-hmac`), constant-time signature check via `hmac::Mac::verify_slice`, freshness window enforced on verify. Eight unit tests cover roundtrip, flow tampering, signature tampering, wrong-key rejection, malformed input, and expiry.
- **`hmac` dependency** (0.12) for state-token signing.

### Changed
- `landing_page`, `login_page`, `connect_google_drive`, `connect_google_calendar` mint signed tokens instead of writing to the session. The `Session` extractor was removed from the three initiator handlers (it had no other purpose).
- `callback` derives the OAuth flow (login/drive/calendar) from the verified token's flow tag rather than a `starts_with` check, so the routing decision is always integrity-protected.

## [5.1.0] - 2026-04-23

### Added
- **Persisted Last Week reports.** The AI-generated Last Week summary on `/reports` is now saved per current-week and survives navigation — no more regenerating every time you switch tabs or pages. Content, the date window it narrates, and an `updated_at` timestamp are stored together in the new `last_week_reports` table. Regenerate overwrites in place; revisiting a week still shows the report you generated earlier with the dates it actually refers to.
- **Migration 022 — `last_week_reports`.** New encrypted table keyed on `(week_id, user_id)` with `content` (AES-256-GCM), `window_start`, `window_end`. Phase-delete cascade updated to include it.

### Changed
- **Reports page subtitle shows a "Saved …" timestamp** alongside the window range when a stored report is displayed, so it's obvious at a glance when the text was last generated.

## [5.0.0] - 2026-04-22

### Added
- **Reports page.** New top-level page (`/reports`) with two tabs sharing a consistent shell:
  - *Last Week* — rolling AI-generated summary of logbook entries from Monday of the previous ISO week through today. Grouped by linked priority (and its parent department goal), with an `Unassigned` bucket for un-linked work. Read-only, regenerated on demand, never persisted; the logbook remains the source of truth.
  - *Latest Updates* — stakeholder-facing status narrative, editable and persisted per week. Markdown preview with an Edit toggle.
- **Migration 021 — `remove_focus_and_todos`.** Drops `weekly_focus`, `weekly_focus_entries`, and `todos` tables. Phase-delete cascade updated to match.

### Changed
- **Last Week summary prompt restructured.** Now takes pre-grouped `EntryGroup` slices keyed by priority/department goal and emits one `## [priority title]` section per bucket, rather than the previous type-grouped "What Shipped / What Progressed / Key Meetings / Help Given" layout. Reflects how stakeholders actually read progress.
- **Status Update moved from slide-over panel to inline section.** `status_update_panel` handler removed; `generate_status_update` and `save_status_update` now render the shared `components/status_update_section.html` fragment so HTMX can swap it in-place on the Reports page.
- **Meeting prep prompt no longer injects focus items.** Focus section removed from `build_meeting_prep_prompt` inputs and output.
- **Sidebar navigation.** Todos link replaced with Reports (document icon). Reflections keeps its current position.
- **Priorities page sinks completed department goals.** On `/priorities`, department goals with `status = "completed"` are now stable-sorted to the bottom of the list and render pre-collapsed, so the top of the page focuses on active work. Users can still expand a completed group manually and the choice is remembered via localStorage (explicit overrides always win over the status-based default).

### Removed
- **Weekly Focus.** Dashboard focus card, carryover suggestions, focus-entry linking, completion toggles, and Last Week button on the dashboard are gone. Handlers (`create_focus`, `update_focus`, `delete_focus`, `toggle_focus_complete`, `last_week_summary`), model (`src/cycle/model/focus.rs`), and repo (`src/cycle/repo/focus.rs`) deleted. The Last Week summary now lives on the Reports page and reads directly from the logbook.
- **Todos feature.** Standalone `/todos` page, encrypted todo titles, toggle/delete handlers, and the `src/todos/` module are removed. The lightweight planning use-case is better served by the logbook plus the Reports "Latest Updates" tab.
- Dashboard template simplified: removed focus card, carryover section, focus entry picker autocomplete, and the `has_ai` / `focus_items` / `picker_entries` / `carryover_items` context keys that fed them. Associated CSS blocks in `static/css/components/dashboard.css` and JS in `static/js/dashboard.js` removed.

### Migration Notes
Existing Weekly Focus and Todos data is dropped by migration 021. Export anything worth keeping before upgrading — there is no in-app export path for these tables.

## [4.0.0] - 2026-04-02

### Added
- **10x Engineer habits framework.** Weekly reflection reworded around ownership, blockers-as-tradeoffs, relationships, giving, and weekly planning. Habit tracker added (boundary protection, curiosity conversations, blocker communication, end-to-end ownership).
- **Enhanced weekly focus.** Focus items now support planning notes (task breakdown), completion toggles with strikethrough, and carryover suggestions from the previous week's incomplete items.
- **Tradeoff-framed blocker log.** Priority updates can be flagged as blockers with a structured tradeoff template ("We can [A] if [sacrifice], or [B] but [consequence]"). Unresolved blocker count shown on dashboard.
- **"What did I do last week?" AI summary.** Button on dashboard generates a structured summary (What Shipped, What Progressed, Key Meetings, Help Given) from last week's entries, focus items, and priorities.
- **Stakeholder status update composer.** Slide-over panel with AI-generated or manual status updates (Progress, Blockers as Tradeoffs, Next Week). Drafts saved per week, copy-to-clipboard for Slack/email.
- **Relationship health tracking.** People aliases now track relationship type (peer, cross-team, PM, designer, stakeholder) and last interaction dates. Stale relationships (30+ days) surfaced in monthly check-ins.
- **Monthly growth check-in.** New monthly reflection with four sections: Learning or Coasting, Reconnect List (auto-populated from stale relationships), Energy & Sustainability (trend from recent weeks), What Can You Let Go.
- **10x Protocol page.** Standalone page with article summary and 14-item weekly checklist grouped by Monday/Daily/Mid-week/Friday/Monthly. Persisted per week with clear-all reset.
- **Todos page.** Personal task list with encrypted titles, completion toggle, and delete. Completed items shown in collapsible section.
- **Reflections page tabs.** Weekly and Monthly reflections in one tabbed view with full CRUD for both.

### Changed
- **Dashboard simplified.** Removed weekly reflection card and monthly check-in prompt. Focus section is now standalone with Status Update and Last Week buttons in header.
- **Review page quarter cards.** All quarter cards now behave consistently — clicking loads content inline (conversation prep or review sections). First quarter auto-selected on page load.
- **Quarterly prep section titles.** New `quarterly_title` field in config prevents weekly titles ("Next Week") from appearing in quarterly context. Quarterly titles: Impact & Outcomes, Lessons & Adjustments, Growth & Relationships, Contributions & Collaboration, Looking Ahead.
- **Sidebar navigation.** Added 10x Protocol (clipboard-check icon) and Todos (checkmark icon) between Priorities and Reflections. Reflections icon changed to book.

### Fixed
- **Select dropdown arrows missing.** Added explicit `appearance: auto` to `.form-select`.
- **Cancel buttons on reflection forms.** Now use direct navigation to bypass HTMX form interception.
- **Nudge bar dismiss.** Removed nudges from dashboard entirely; guidance moved to 10x Protocol checklist.

## [3.0.1] - 2026-03-18

### Fixed
- **Completed priorities missing from dropdowns.** Logbook filter, dashboard quick-add, and entry edit dropdowns now include all priorities regardless of status, not just active ones.
- **Entry edit priority dropdown not grouped by goals.** The priority select when editing a logbook entry now uses `<optgroup>` headers to group priorities under their department goals, matching the logbook filter dropdown.

## [3.0.0] - 2026-03-13

### Changed
- **DDD decomposition of cycle/ module.** Split into three focused bounded contexts:
  - `cycle/` — Phase lifecycle, weeks, focus items, dashboard, logbook, meeting prep, trends
  - `reflections/` — Weekly and quarterly check-ins with config
  - `review/` — Self-review summaries, contribution examples, AI documents, export, assessment/rating config
- **AI helper functions moved to ai/ module.** `get_ai_client()` and `has_ai_for_user()` shared by all modules.
- **Config initialization decentralized.** Each module loads its own config; `initialize_config()` removed.
- Saving a weekly reflection now redirects to /checkins instead of /dashboard.
- Conversation prep opens in slide-over panel on review page instead of navigating away.

### Removed
- Dead code: `get_checkin_section()`, `BragPhase::is_active()` method
- Dead template: `department_goal_item.html`
- Dead CSS: sidebar panel, sidebar list, page mascot, bulk merge toggle classes
- Empty directories: `tools/`, `static/img/product/`

## [2.4.1] - 2026-03-09

### Fixed
- **Google Docs comment-only interactions missing from sync.** The Drive Activity API does not reliably surface comment activities on shared/team documents. Added a supplementary fetch using the Drive Files API and Comments API to catch comments that the Activity API misses. Files recently viewed by the user are checked for authored comments within the sync date range, creating `drive_commented` entries for any found.
- **Google Drive OAuth scope expanded.** Added `drive.readonly` scope alongside the existing `drive.activity.readonly` to support the supplementary comments fetch. Users need to re-connect Google Drive in integrations settings to grant the new scope. Existing tokens without the new scope degrade gracefully — the Activity API results are still returned.

## [2.4.0] - 2026-03-05

### Added
- **Live sync status on integrations page.** Sync All spawns a background sync and the activity area polls every 3s, showing a progress banner with the current service. Sync All button stays in spinner state until sync completes.
- **Sidebar sync indicator.** Green dot when synced, red dot on error, spinning arrows during active sync. Links to integrations page. Updates immediately via OOB swaps when sync starts/stops.
- **Lattice import via sidebar panel.** Import from Lattice now opens in the slide-over panel with inline success/error feedback instead of a full-page form.
- **Auto-create department goals from Lattice parent references.** Individual-only Lattice exports include Parent ID and Parent goal columns — the importer now auto-creates department goals from these and nests priorities under them.
- **Import upsert with external_id.** Re-importing the same Lattice CSV updates existing records instead of creating duplicates. New `external_id` column on priorities and department_goals tables.
- **Priorities page status summary bar.** Colored dot counts for Active, Not Started, On Hold, Completed, Cancelled. Click a status to filter.
- **Priorities search and filter toolbar.** Text search filters by title, dropdown filters by status.
- **Click-to-edit priorities and department goals.** Clicking a priority or department goal row opens the edit panel. No more inline edit/delete/info buttons.
- **Delete button in edit panels.** Both priority and department goal edit panels include a delete action at the bottom.
- **Background sync infrastructure.** `SyncStatusMap` (DashMap) tracks per-user sync state in memory, updated by background sync tasks.

### Changed
- Sync All button always visible on integrations page (not just when no entries exist).
- Synced Entries card no longer has its own Sync All button (single source in Services header).
- Lattice CSV parser accepts flexible column names (`ID`/`Goal ID`, `Goal name`/`Name`, etc.) and strips BOM.
- Department goal rows: chevron toggles collapse, clicking the row opens edit panel.

### Fixed
- Sync All button no longer returns to clickable state before sync finishes (was caused by background spawn returning immediately).
- Sidebar sync indicator was invisible due to `hx-preserve` conflicts with hx-boost page transitions and competing `margin-left: auto` in flex layout.

## [2.2.0] - 2026-02-26

### Added
- Impact narrative field on priority create form. Previously only available when editing an existing priority.
- Info toggle button on department goals and priorities to reveal description/narrative text inline.
- Description textarea in department goal inline edit form on priorities page.
### Fixed
- Department goal edit form on priorities page was missing the description field — only title and status could be edited.
- Department goal description now displays below the title row with a "Description" label, matching priority narrative layout.
- Priority impact narrative aligned flush left with an "Impact Narrative" label instead of indented italic text.
- Integration cards no longer auto-expand when token is missing — all cards start collapsed.
- Bulk edit toolbar on analyze page moved to overlay block with two-row layout (controls + fields) and body padding to keep footer reachable.

## [2.1.1] - 2026-02-26

### Fixed
- People alias table now uses a single `<table>` with fixed column widths (`table-layout: fixed` + `<colgroup>`). Add form inputs, data rows, and edit-mode inputs all share the same columns so everything stays aligned.
- Editing an alias no longer shifts column widths. Edit inputs use the same `alias-input` class as the add row.
- HTMX swap target changed to `innerHTML` on the `<tbody>` so the partial returns bare `<tr>` rows instead of a wrapping `<div>`.

## [2.1.0] - 2026-02-26

### Added
- **People aliases with team mapping.** Settings > People now supports mapping emails, GitHub usernames, and Jira accounts to display names and an optional team. Collaborator emails are stored raw and aliased only at display time, so alias changes take effect immediately without re-syncing.
- **Auto team enrichment from aliases.** When a collaborator matches an alias with a team, that team is automatically added to the entry — both at sync time (persisted) and at display time (for existing entries without re-sync).
- **Inline editing for people aliases.** Each alias row has an Edit button that turns it into editable input fields with Save/Cancel.
- **Collaborator extraction for GitHub, Jira, and Bugzilla.** GitHub sync now extracts PR reviewers (for authored/merged PRs) and PR authors (for reviewed PRs). Jira extracts assignee/creator. Bugzilla extracts assigned_to/creator. All stored as collaborators on entries.
- **Bulk edit for logbook entries.** Select multiple entries and apply priority, reach, complexity, role, teams, or collaborators in one action. Supports append or replace merge modes.
- **Bulk edit toolbar** with select all/deselect, field dropdowns, and append/replace toggle.

### Changed
- **Trends page rebuilt with native charts.** Replaced the insights module with inline CSS-only bar charts for categories, priorities, collaborators, repos, teams, weekdays, and impact signals. Removed the separate analytics/insights service.
- **Logbook report sidebar replaced by bulk edit.** The "Report" button is now "Bulk Edit" with a sticky bottom toolbar.
- **Entry cards show bulk-edit checkboxes** when bulk edit mode is active.
- **Entry meta row wraps on narrow screens** (flex-wrap added to `.entry-meta-left`).

### Removed
- `compute_insights()` function and `insights` service module (replaced by inline Trends page logic).
- `analyze_report.html` template (report sidebar replaced by bulk edit).

## [2.0.1] - 2026-02-26

### Fixed
- Calendar sync no longer stores event bodies. Invite descriptions (Zoom boilerplate, HTML fragments, attendee lists) were being written into `brag_entries.description`, cluttering the logbook. Sync now sets `description: None` for calendar entries, keeping descriptions exclusively for user-authored content.

### Changed
- Calendar attendees stored as collaborators. Attendee names and emails extracted during Google Calendar sync are now written to the `collaborators` field instead of being embedded in the description. They render as collaborator chips on entry cards.
- Sync upsert preserves user-written descriptions. The `ON CONFLICT` clause for `description` changed from unconditional overwrite to `COALESCE(existing, incoming)`, so descriptions set by the user (e.g., meeting prep notes) survive re-syncs.
- `SyncedEntry` carries a `collaborators` field. All non-calendar sync sources pass `None`; only Google Calendar populates it from attendee data.

### Added
- Meeting prep notes flow into logbook entries. Saving prep notes via the dashboard meeting panel encrypts and writes them into `brag_entries.description`, so prep content appears as the meeting body in the logbook and feeds into AI draft generation.

## [2.0.0] - 2026-02-24

### Changed
- **Priorities replace OKRs.** Goals, Key Results, and Initiatives removed in favour of a flat Priorities model with color coding and progress tracking
- **Dashboard layout restructured.** Weekly Focus + Weekly Check-in share a top row at equal height; sidebar reordered to Active Work → Priorities → Focus Time This Week
- **Trends page simplified.** Category Distribution bars now include per-type tooltip breakdowns (hover the ⓘ icon); separate Code Activity and Ticket Activity sections removed to eliminate conflicting counts
- **Category Distribution split.** Bug/Jira entry types moved from "Code" to a new "Tickets" category so counts align with what each category actually represents
- **Priority Activity bars use stacked layout.** Label row on top, bar below — long priority names no longer misalign bars
- **Focus Time day labels shortened** to Mo, Tu, We, Th, Fr
- **Summary page rebuilt** with config-driven sections from `review_sections.toml`
- **Annual Alignment page removed** (replaced by Review Guide)

### Added
- Priorities management page with color picker, drag-to-reorder, and progress tracking
- Review Guide page for self-review preparation
- Summary section components driven by `review_sections.toml` config
- Migration `003_drop_legacy_tables.sql` to clean up legacy OKR/initiative/impact-story tables
- Public routes module for unauthenticated pages

### Fixed
- Trends entry count now matches logbook by filtering out future synced entries
- Calendar sync includes all non-declined events from the user's primary calendar (previously required explicit accept/tentative, missing auto-added org and team meetings)
- Calendar sync handles large org-wide events where the API omits the full attendee list

### Removed
- Goals, Key Results, and Initiatives (model, routes, templates, CSS)
- Annual Alignment page and route
- Analytics module (insights moved to logbook routes)
- Sync button component
- Dead `just-added-list` div and CSS from dashboard quick capture form

## [1.1.4] - 2026-02-24

### Fixed
- SQLite journal mode now configurable via `SQLITE_JOURNAL_MODE` env var (default: WAL)

## [1.1.3] - 2026-02-24

### Fixed
- Dashboard meetings widget now shows all synced calendar meetings, including ones deleted from the logbook
- Manual meeting entries no longer appear in the dashboard meetings widget

## [1.1.2] - 2026-02-24

### Fixed
- Google Calendar sync now includes events where the user is the organizer (responseStatus may not be "accepted")
- Also accept tentatively-accepted events in calendar sync

## [1.1.1] - 2026-02-24

### Fixed
- Logbook entry count now excludes future synced entries, so total matches visible entries with no filters
- "Unlinked" filter now correctly treats entries linked to an initiative as linked

## [1.1.0] - 2026-02-24

### Added
- Link entries to initiatives via dropdown in both the inline edit form and dashboard quick-add
- Initiative filter on the logbook page (first toolbar row, next to Key Results)
- Inline edit form for initiatives on the Goals page, matching the existing goal/KR pattern
- Collapsible goals in the dashboard OKR snapshot with localStorage-persisted state
- "Show only" dropdown in logbook filters (Unlinked, Missing team, Missing collaborator)

### Changed
- Dashboard OKR snapshot hides fully-completed goals (all KRs at 100%) to reduce clutter
- Logbook filter toolbar reorganized: types and show-only moved to second row
- Initiative detail panel is now read-only; editing happens inline on the Goals page

### Fixed
- Google Calendar sync now includes large events with hidden guest lists (attendeesOmitted support)
- 1Password autofill popup no longer triggers on integration token fields (data-1p-ignore)
