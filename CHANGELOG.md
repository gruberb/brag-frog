# Changelog

All notable changes to Brag Frog will be documented in this file.

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
