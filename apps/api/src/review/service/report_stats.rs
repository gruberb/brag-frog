//! Pure aggregation for saved reports. No I/O, so it is covered by unit tests and can be
//! swapped for the Trends service later without touching the handlers.

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::cycle::service::logbook_insights::LogbookFilter;
use crate::worklog::model::BragEntry;

/// How many names the `*_top` lists keep.
pub const TOP_N: usize = 10;
/// How many key entries a report highlights.
pub const KEY_ENTRY_LIMIT: usize = 10;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NameCount {
    pub name: String,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WeekCount {
    /// Monday of the ISO week, `YYYY-MM-DD`.
    pub week_start: String,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectCount {
    /// `null` groups entries that are not linked to a priority.
    pub priority_id: Option<i64>,
    pub name: String,
    pub count: usize,
}

/// Numbers describing the filtered entry set. `entries` is the entry count, `people`, `repos`
/// and `teams` are distinct counts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReportStats {
    pub entries: usize,
    pub people: usize,
    pub repos: usize,
    pub teams: usize,
    pub per_week: Vec<WeekCount>,
    pub per_project: Vec<ProjectCount>,
    pub people_top: Vec<NameCount>,
    pub repos_top: Vec<NameCount>,
    pub teams_top: Vec<NameCount>,
}

/// The slice of an entry a report keeps as evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReportKeyEntry {
    pub id: i64,
    pub title: String,
    pub entry_type: String,
    pub source: String,
    pub source_url: Option<String>,
    pub occurred_at: String,
    pub priority_id: Option<i64>,
    pub repository: Option<String>,
    pub reach: Option<String>,
    pub complexity: Option<String>,
}

impl From<&BragEntry> for ReportKeyEntry {
    fn from(e: &BragEntry) -> Self {
        Self {
            id: e.id,
            title: e.title.clone(),
            entry_type: e.entry_type.clone(),
            source: e.source.clone(),
            source_url: e.source_url.clone(),
            occurred_at: e.occurred_at.clone(),
            priority_id: e.priority_id,
            repository: e.repository.clone(),
            reach: e.reach.clone(),
            complexity: e.complexity.clone(),
        }
    }
}

fn split_list(value: Option<&str>) -> impl Iterator<Item = &str> {
    value
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

/// Counts distinct values per entry (an entry naming the same person twice counts once), then
/// ranks by count descending and name ascending so output is deterministic.
fn ranked<'a>(values: impl Iterator<Item = HashSet<&'a str>>) -> Vec<NameCount> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for set in values {
        for v in set {
            *counts.entry(v).or_default() += 1;
        }
    }
    let mut out: Vec<NameCount> = counts
        .into_iter()
        .map(|(name, count)| NameCount {
            name: name.to_string(),
            count,
        })
        .collect();
    out.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
    out
}

fn week_start(date: &str) -> Option<String> {
    let d = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    let monday = d - Duration::days(i64::from(d.weekday().num_days_from_monday()));
    Some(monday.format("%Y-%m-%d").to_string())
}

/// Aggregates the entries of a report. `priority_names` resolves `per_project` names, an id
/// missing from it (deleted priority) is labelled "Unknown priority".
pub fn compute_report_stats(
    entries: &[BragEntry],
    priority_names: &HashMap<i64, String>,
) -> ReportStats {
    let people = ranked(
        entries
            .iter()
            .map(|e| split_list(e.collaborators.as_deref()).collect()),
    );
    let repos = ranked(entries.iter().map(|e| {
        e.repository
            .as_deref()
            .map(str::trim)
            .filter(|r| !r.is_empty())
            .into_iter()
            .collect()
    }));
    let teams = ranked(
        entries
            .iter()
            .map(|e| split_list(e.teams.as_deref()).collect()),
    );

    let mut weeks: BTreeMap<String, usize> = BTreeMap::new();
    for ws in entries.iter().filter_map(|e| week_start(&e.occurred_at)) {
        *weeks.entry(ws).or_default() += 1;
    }

    let mut projects: HashMap<Option<i64>, usize> = HashMap::new();
    for e in entries {
        *projects.entry(e.priority_id).or_default() += 1;
    }
    let mut per_project: Vec<ProjectCount> = projects
        .into_iter()
        .map(|(priority_id, count)| ProjectCount {
            name: match priority_id {
                None => "Unassigned".to_string(),
                Some(id) => priority_names
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| "Unknown priority".to_string()),
            },
            priority_id,
            count,
        })
        .collect();
    per_project.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));

    let top = |v: &[NameCount]| v.iter().take(TOP_N).cloned().collect::<Vec<_>>();
    ReportStats {
        entries: entries.len(),
        people: people.len(),
        repos: repos.len(),
        teams: teams.len(),
        per_week: weeks
            .into_iter()
            .map(|(week_start, count)| WeekCount { week_start, count })
            .collect(),
        per_project,
        people_top: top(&people),
        repos_top: top(&repos),
        teams_top: top(&teams),
    }
}

fn reach_rank(reach: Option<&str>) -> u8 {
    match reach {
        Some("company") => 4,
        Some("org") => 3,
        Some("cross_team") => 2,
        Some("team") => 1,
        _ => 0,
    }
}

fn complexity_rank(complexity: Option<&str>) -> u8 {
    match complexity {
        Some("novel") => 4,
        Some("complex") => 3,
        Some("moderate") => 2,
        Some("routine") => 1,
        _ => 0,
    }
}

/// The `limit` entries with the widest reach, then the highest complexity, then the most
/// recent. Entries without either rating still fill the list when there are few rated ones.
pub fn pick_key_entries(entries: &[BragEntry], limit: usize) -> Vec<ReportKeyEntry> {
    let mut ranked: Vec<&BragEntry> = entries.iter().collect();
    ranked.sort_by(|a, b| {
        reach_rank(b.reach.as_deref())
            .cmp(&reach_rank(a.reach.as_deref()))
            .then_with(|| {
                complexity_rank(b.complexity.as_deref())
                    .cmp(&complexity_rank(a.complexity.as_deref()))
            })
            .then_with(|| b.occurred_at.cmp(&a.occurred_at))
            .then_with(|| b.id.cmp(&a.id))
    });
    ranked
        .into_iter()
        .take(limit)
        .map(ReportKeyEntry::from)
        .collect()
}

/// `Jan 1 – Jun 30, 2025`, or with both years when the range crosses a year boundary. Falls
/// back to the raw strings when a date does not parse.
pub fn range_label(from: &str, to: &str) -> String {
    let parse = |s: &str| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok();
    match (parse(from), parse(to)) {
        (Some(f), Some(t)) if f.year() == t.year() => {
            format!("{} – {}", f.format("%b %-d"), t.format("%b %-d, %Y"))
        }
        (Some(f), Some(t)) => format!("{} – {}", f.format("%b %-d, %Y"), t.format("%b %-d, %Y")),
        _ => format!("{from} – {to}"),
    }
}

/// `<priority name or "All projects"> · <range label>`
pub fn default_title(priority_name: Option<&str>, from: &str, to: &str) -> String {
    format!(
        "{} · {}",
        priority_name.unwrap_or("All projects"),
        range_label(from, to)
    )
}

/// One line describing the active filters, for the AI prompt. Dates and the priority are
/// described separately by the caller.
pub fn describe_filters(f: &LogbookFilter) -> String {
    let parts: Vec<String> = [
        ("source", &f.source),
        ("category", &f.category),
        ("title contains", &f.q),
        ("team", &f.team),
        ("person", &f.person),
        ("reach", &f.reach),
        ("complexity", &f.complexity),
        ("role", &f.role),
    ]
    .into_iter()
    .filter_map(|(label, v)| {
        v.as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(|v| format!("{label}: {v}"))
    })
    .collect();
    if parts.is_empty() {
        "none".to_string()
    } else {
        parts.join("; ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: i64, date: &str) -> BragEntry {
        BragEntry {
            id,
            week_id: 1,
            priority_id: None,
            source: "manual".into(),
            source_id: None,
            source_url: None,
            title: format!("Entry {id}"),
            description: None,
            entry_type: "development".into(),
            status: None,
            repository: None,
            occurred_at: date.into(),
            teams: None,
            collaborators: None,
            outcome_statement: None,
            evidence_urls: None,
            role: None,
            impact_tags: None,
            reach: None,
            complexity: None,
            decision_alternatives: None,
            decision_reasoning: None,
            decision_outcome: None,
            meeting_role: None,
            recurring_group: None,
            start_time: None,
            end_time: None,
            created_at: String::new(),
            updated_at: String::new(),
            deleted_at: None,
        }
    }

    #[test]
    fn empty_input_gives_zeroed_stats() {
        let s = compute_report_stats(&[], &HashMap::new());
        assert_eq!(s.entries, 0);
        assert!(s.per_week.is_empty() && s.per_project.is_empty() && s.people_top.is_empty());
    }

    #[test]
    fn counts_distinct_people_repos_and_teams() {
        let mut a = entry(1, "2025-01-14");
        a.collaborators = Some("Ann, Bob, Ann".into());
        a.repository = Some("acme/foo".into());
        a.teams = Some("Search".into());
        let mut b = entry(2, "2025-01-15");
        b.collaborators = Some("Ann".into());
        b.repository = Some("acme/foo".into());
        b.teams = Some("Search, Ads".into());
        let mut c = entry(3, "2025-01-16");
        c.repository = Some("  ".into());

        let s = compute_report_stats(&[a, b, c], &HashMap::new());
        assert_eq!((s.entries, s.people, s.repos, s.teams), (3, 2, 1, 2));
        assert_eq!(
            s.people_top[0],
            NameCount {
                name: "Ann".into(),
                count: 2
            },
            "duplicate in one entry counts once"
        );
        assert_eq!(
            s.people_top[1],
            NameCount {
                name: "Bob".into(),
                count: 1
            }
        );
        assert_eq!(
            s.repos_top,
            vec![NameCount {
                name: "acme/foo".into(),
                count: 2
            }]
        );
        assert_eq!(s.teams_top[0].name, "Search");
    }

    #[test]
    fn ties_rank_alphabetically_and_lists_are_capped() {
        let entries: Vec<BragEntry> = (0..15)
            .map(|i| {
                let mut e = entry(i, "2025-01-14");
                e.collaborators = Some(format!("P{i:02}"));
                e
            })
            .collect();
        let s = compute_report_stats(&entries, &HashMap::new());
        assert_eq!(s.people, 15);
        assert_eq!(s.people_top.len(), TOP_N);
        assert_eq!(s.people_top[0].name, "P00");
    }

    #[test]
    fn groups_by_iso_week_starting_monday() {
        // 2025-01-12 is a Sunday, 2025-01-13 a Monday.
        let entries = [
            entry(1, "2025-01-12"),
            entry(2, "2025-01-13"),
            entry(3, "2025-01-19"),
            entry(4, "not-a-date"),
        ];
        let s = compute_report_stats(&entries, &HashMap::new());
        assert_eq!(
            s.per_week,
            vec![
                WeekCount {
                    week_start: "2025-01-06".into(),
                    count: 1
                },
                WeekCount {
                    week_start: "2025-01-13".into(),
                    count: 2
                },
            ]
        );
    }

    #[test]
    fn per_project_names_unassigned_and_deleted_priorities() {
        let mut a = entry(1, "2025-01-14");
        a.priority_id = Some(7);
        let mut b = entry(2, "2025-01-14");
        b.priority_id = Some(7);
        let mut c = entry(3, "2025-01-14");
        c.priority_id = Some(99);
        let d = entry(4, "2025-01-14");
        let names = HashMap::from([(7, "Search".to_string())]);

        let s = compute_report_stats(&[a, b, c, d], &names);
        assert_eq!(
            s.per_project[0],
            ProjectCount {
                priority_id: Some(7),
                name: "Search".into(),
                count: 2
            }
        );
        let labels: Vec<&str> = s.per_project.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(labels, ["Search", "Unassigned", "Unknown priority"]);
    }

    #[test]
    fn key_entries_rank_reach_then_complexity_then_recency() {
        let mut team_novel = entry(1, "2025-01-10");
        team_novel.reach = Some("team".into());
        team_novel.complexity = Some("novel".into());
        let mut org_routine = entry(2, "2025-01-01");
        org_routine.reach = Some("org".into());
        org_routine.complexity = Some("routine".into());
        let mut org_complex = entry(3, "2025-01-02");
        org_complex.reach = Some("org".into());
        org_complex.complexity = Some("complex".into());
        let unrated_new = entry(4, "2025-02-01");
        let unrated_old = entry(5, "2025-01-05");

        let all = [
            team_novel,
            org_routine,
            org_complex,
            unrated_new,
            unrated_old,
        ];
        let ids: Vec<i64> = pick_key_entries(&all, 4).iter().map(|e| e.id).collect();
        assert_eq!(ids, [3, 2, 1, 4]);
        assert_eq!(pick_key_entries(&all, 99).len(), 5);
    }

    #[test]
    fn titles_and_range_labels() {
        assert_eq!(
            range_label("2025-01-01", "2025-06-30"),
            "Jan 1 – Jun 30, 2025"
        );
        assert_eq!(
            range_label("2024-10-01", "2025-03-31"),
            "Oct 1, 2024 – Mar 31, 2025"
        );
        assert_eq!(range_label("x", "y"), "x – y");
        assert_eq!(
            default_title(None, "2025-01-01", "2025-06-30"),
            "All projects · Jan 1 – Jun 30, 2025"
        );
        assert_eq!(
            default_title(Some("Search"), "2025-01-01", "2025-06-30"),
            "Search · Jan 1 – Jun 30, 2025"
        );
    }

    #[test]
    fn describes_only_set_filters() {
        assert_eq!(describe_filters(&LogbookFilter::default()), "none");
        let f = LogbookFilter {
            source: Some("github".into()),
            team: Some(" ".into()),
            q: Some("search".into()),
            ..Default::default()
        };
        assert_eq!(
            describe_filters(&f),
            "source: github; title contains: search"
        );
    }
}
