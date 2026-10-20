//! Trend aggregation over a cycle's entries. Returns typed data only: presentation (colours,
//! percentages, humanised labels) is the caller's concern, so the same numbers can feed the
//! legacy HTML page, the JSON API and, later, saved reports.

use std::collections::HashMap;

use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use ts_rs::TS;

use crate::cycle::model::BragPhase;
use crate::identity::model::PeopleAlias;
use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;
use crate::objectives::model::{DepartmentGoal, Priority};
use crate::worklog::model::{BragEntry, EntryType, category_label};

/// How many rows the people, repository and team rankings keep.
pub const TOP_N: usize = 8;

/// Selects the entries a trends computation covers. Every field is optional so the same
/// type can describe a plain cycle view, a Logbook filter set or a saved report.
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EntryFilter {
    /// Inclusive `YYYY-MM-DD`; defaults to the cycle start.
    pub from: Option<String>,
    /// Inclusive `YYYY-MM-DD`; defaults to the cycle end.
    pub to: Option<String>,
    #[serde(default)]
    pub priority_ids: Vec<i64>,
    pub goal_id: Option<i64>,
    pub source: Option<String>,
    pub entry_type: Option<String>,
    pub reach: Option<String>,
    pub complexity: Option<String>,
    pub role: Option<String>,
    /// Case-insensitive substring of title or description.
    pub q: Option<String>,
    pub team: Option<String>,
    /// Matches a collaborator by raw value or by alias display name.
    pub person: Option<String>,
    #[serde(default)]
    pub no_priority: bool,
    #[serde(default)]
    pub no_team: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Kpis {
    pub docs_created: u32,
    pub prs_opened: u32,
    pub prs_reviewed: u32,
    pub prs_merged: u32,
    pub meetings: u32,
    pub tickets_opened: u32,
    pub tickets_closed: u32,
}

/// Entry counts per category. One field per entry of
/// [`CATEGORIES`](crate::worklog::model::CATEGORIES) (a test keeps them in
/// step), typed fields rather than a map so the web client gets every key without null checks.
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CategoryTotals {
    pub code: u32,
    pub reviews: u32,
    pub tickets: u32,
    pub docs: u32,
    pub meetings: u32,
    pub collaboration: u32,
    pub learning: u32,
    pub other: u32,
}

impl CategoryTotals {
    fn add(&mut self, category: &str) {
        let slot = match category {
            "code" => &mut self.code,
            "reviews" => &mut self.reviews,
            "tickets" => &mut self.tickets,
            "docs" => &mut self.docs,
            "meetings" => &mut self.meetings,
            "collaboration" => &mut self.collaboration,
            "learning" => &mut self.learning,
            _ => &mut self.other,
        };
        *slot += 1;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WeekBucket {
    /// Monday of the ISO week, `YYYY-MM-DD`.
    pub week_start: String,
    pub by_category: CategoryTotals,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NameCount {
    pub name: String,
    pub count: u32,
}

/// A category with its per-type breakdown (labels as shown in the UI).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CategoryCount {
    /// Category slug, as accepted by the logbook `category` filter.
    pub key: String,
    pub name: String,
    pub count: u32,
    pub types: Vec<NameCount>,
}

/// Activity attributed to one priority. `priority_id` is `None` for entries without a priority.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PriorityShare {
    pub priority_id: Option<i64>,
    pub title: String,
    /// The user's own priority colour (data, not a design token).
    pub color: Option<String>,
    pub status: Option<String>,
    pub department_goal_id: Option<i64>,
    pub department_goal_title: Option<String>,
    pub department_goal_status: Option<String>,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ImpactBreakdown {
    pub reach: Vec<NameCount>,
    pub complexity: Vec<NameCount>,
    pub role: Vec<NameCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Trends {
    pub kpis: Kpis,
    pub weekly: Vec<WeekBucket>,
    pub categories: Vec<CategoryCount>,
    pub priority_share: Vec<PriorityShare>,
    pub impact: ImpactBreakdown,
    pub people: Vec<NameCount>,
    pub repos: Vec<NameCount>,
    pub teams: Vec<NameCount>,
    /// Entries per weekday (Monday first, always seven rows). Entries only carry a date, so
    /// there is no hour axis; the UI labels this "logged at".
    pub weekday: Vec<NameCount>,
}

const WEEKDAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

/// Category slug of an entry type; unknown types (old rows) count as `other`.
fn entry_category(entry_type: &str) -> &'static str {
    EntryType::category_for_slug(entry_type).unwrap_or("other")
}

fn entry_type_label(entry_type: &str) -> &'static str {
    match entry_type {
        "pr_authored" => "PRs Opened",
        "pr_merged" => "PRs Merged",
        "pr_reviewed" => "PRs Reviewed",
        "pr_development" => "Development PRs",
        "revision_authored" => "Revisions Authored",
        "revision_reviewed" => "Revisions Reviewed",
        "code_review" => "Code Reviews",
        "development" => "Development",
        "bug_filed" => "Bugs Filed",
        "bug_fixed" => "Bugs Fixed",
        "jira_completed" => "Jira Completed",
        "jira_story" => "Jira Stories",
        "jira_task" => "Jira Tasks",
        "jira_epic" => "Jira Epics",
        "jira_progress" => "Jira Progress",
        "meeting" => "Meetings",
        "design_doc" => "Design Docs",
        "document" => "Documents",
        "confluence_page" => "Confluence Pages",
        "drive_created" => "Drive Created",
        "drive_edited" => "Drive Edited",
        "drive_commented" => "Drive Commented",
        _ => "Other",
    }
}

/// Edits and comments are excluded. Drive keeps one entry per file and day and lets an edit
/// win over a create, so a file created and edited the same day is not counted here.
fn is_doc_created(entry_type: &str) -> bool {
    matches!(
        entry_type,
        "drive_created" | "confluence_page" | "document" | "design_doc"
    )
}

/// Every ticket entry is dated by the issue's creation, except `jira_progress`, which marks a
/// day on which a status changed.
fn is_ticket_opened(entry_type: &str) -> bool {
    entry_category(entry_type) == "tickets" && entry_type != "jira_progress"
}

/// Jira by terminal status, Bugzilla by `bug_fixed` (which Jira bugs also use once done).
fn is_ticket_closed(entry: &BragEntry) -> bool {
    entry.entry_type == "bug_fixed" || is_closed_jira_entry(entry)
}

fn is_terminal_jira_status(status: Option<&str>) -> bool {
    let Some(status) = status else {
        return false;
    };
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "done"
            | "closed"
            | "resolved"
            | "verified"
            | "complete"
            | "completed"
            | "fixed"
            | "won't do"
            | "wont do"
            | "duplicate"
            | "cancelled"
            | "canceled"
            | "declined"
    )
}

fn is_closed_jira_entry(entry: &BragEntry) -> bool {
    entry.entry_type == "jira_completed"
        || (entry.source == "jira" && is_terminal_jira_status(entry.status.as_deref()))
}

fn split_list(value: Option<&str>) -> impl Iterator<Item = &str> {
    value
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn display_person(raw: &str, aliases: &HashMap<String, String>) -> String {
    aliases
        .get(&raw.to_lowercase())
        .cloned()
        .unwrap_or_else(|| raw.to_string())
}

/// Sorted by count descending, ties by name so output is stable across runs.
fn ranked(counts: HashMap<String, u32>, limit: Option<usize>) -> Vec<NameCount> {
    let mut items: Vec<NameCount> = counts
        .into_iter()
        .map(|(name, count)| NameCount { name, count })
        .collect();
    items.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
    if let Some(n) = limit {
        items.truncate(n);
    }
    items
}

fn eq_ci(a: Option<&str>, b: &str) -> bool {
    a.is_some_and(|v| v.eq_ignore_ascii_case(b))
}

impl EntryFilter {
    /// In-memory predicate for fields the database cannot filter (encrypted columns) and for
    /// the cheap ones too, so a single pass handles every key. Date bounds are applied by the
    /// SQL range query, not here.
    fn matches(
        &self,
        e: &BragEntry,
        priority_to_goal: &HashMap<i64, Option<i64>>,
        aliases: &HashMap<String, String>,
    ) -> bool {
        if self.no_priority && e.priority_id.is_some() {
            return false;
        }
        if !self.priority_ids.is_empty()
            && !e
                .priority_id
                .is_some_and(|p| self.priority_ids.contains(&p))
        {
            return false;
        }
        if let Some(goal) = self.goal_id
            && e.priority_id
                .and_then(|p| priority_to_goal.get(&p).copied().flatten())
                != Some(goal)
        {
            return false;
        }
        if self.source.as_deref().is_some_and(|s| s != e.source) {
            return false;
        }
        if self
            .entry_type
            .as_deref()
            .is_some_and(|t| t != e.entry_type)
        {
            return false;
        }
        if let Some(v) = &self.reach
            && !eq_ci(e.reach.as_deref(), v)
        {
            return false;
        }
        if let Some(v) = &self.complexity
            && !eq_ci(e.complexity.as_deref(), v)
        {
            return false;
        }
        if let Some(v) = &self.role
            && !eq_ci(e.role.as_deref(), v)
        {
            return false;
        }
        if let Some(q) = self.q.as_deref().filter(|q| !q.is_empty()) {
            let q = q.to_lowercase();
            let hit = e.title.to_lowercase().contains(&q)
                || e.description
                    .as_deref()
                    .is_some_and(|d| d.to_lowercase().contains(&q));
            if !hit {
                return false;
            }
        }
        if self.no_team && split_list(e.teams.as_deref()).next().is_some() {
            return false;
        }
        if let Some(team) = &self.team
            && !split_list(e.teams.as_deref()).any(|t| t.eq_ignore_ascii_case(team))
        {
            return false;
        }
        if let Some(person) = &self.person {
            let hit = split_list(e.collaborators.as_deref()).any(|c| {
                c.eq_ignore_ascii_case(person)
                    || display_person(c, aliases).eq_ignore_ascii_case(person)
            });
            if !hit {
                return false;
            }
        }
        true
    }
}

fn monday_of(date: NaiveDate) -> NaiveDate {
    date - Duration::days(i64::from(date.weekday().num_days_from_monday()))
}

/// Aggregates the cycle's entries selected by `filter`.
///
/// `today` hides future synced entries (for example upcoming calendar meetings) exactly like
/// the Logbook does; manual entries are kept regardless of date. Callers pass it in so the
/// result is deterministic under test.
pub async fn compute(
    pool: &SqlitePool,
    crypto: &UserCrypto,
    user_id: i64,
    phase: &BragPhase,
    filter: &EntryFilter,
    today: NaiveDate,
) -> Result<Trends, AppError> {
    let from = filter.from.as_deref().unwrap_or(&phase.start_date);
    let to = filter.to.as_deref().unwrap_or(&phase.end_date);

    let mut entries = BragEntry::list_for_phase_in_range(pool, phase.id, from, to, crypto).await?;
    let today_str = today.format("%Y-%m-%d").to_string();
    entries.retain(|e| e.source == "manual" || e.occurred_at.as_str() <= today_str.as_str());

    let priorities = Priority::list_for_phase(pool, phase.id, crypto).await?;
    let goals = DepartmentGoal::list_for_phase(pool, phase.id, crypto).await?;
    let aliases = PeopleAlias::alias_map(pool, user_id).await?;

    let priority_to_goal: HashMap<i64, Option<i64>> = priorities
        .iter()
        .map(|p| (p.id, p.department_goal_id))
        .collect();
    entries.retain(|e| filter.matches(e, &priority_to_goal, &aliases));

    let mut kpis = Kpis {
        docs_created: 0,
        prs_opened: 0,
        prs_reviewed: 0,
        prs_merged: 0,
        meetings: 0,
        tickets_opened: 0,
        tickets_closed: 0,
    };
    let mut by_priority: HashMap<Option<i64>, u32> = HashMap::new();
    let mut reach: HashMap<String, u32> = HashMap::new();
    let mut complexity: HashMap<String, u32> = HashMap::new();
    let mut role: HashMap<String, u32> = HashMap::new();
    let mut people: HashMap<String, u32> = HashMap::new();
    let mut repos: HashMap<String, u32> = HashMap::new();
    let mut teams: HashMap<String, u32> = HashMap::new();
    let mut weekday = [0u32; 7];
    let mut weekly: HashMap<NaiveDate, CategoryTotals> = HashMap::new();
    let mut categories: HashMap<&'static str, HashMap<&'static str, u32>> = HashMap::new();

    for e in &entries {
        *by_priority.entry(e.priority_id).or_default() += 1;

        for (value, counts) in [
            (&e.reach, &mut reach),
            (&e.complexity, &mut complexity),
            (&e.role, &mut role),
        ] {
            if let Some(v) = value.as_deref().filter(|v| !v.is_empty()) {
                *counts.entry(v.to_string()).or_default() += 1;
            }
        }
        for c in split_list(e.collaborators.as_deref()) {
            *people.entry(display_person(c, &aliases)).or_default() += 1;
        }
        // Repository can hold a URL for sources without a repo concept; those are not repos.
        if let Some(r) = e
            .repository
            .as_deref()
            .filter(|r| !r.is_empty() && !r.starts_with("http://") && !r.starts_with("https://"))
        {
            *repos.entry(r.to_string()).or_default() += 1;
        }
        for t in split_list(e.teams.as_deref()) {
            *teams.entry(t.to_string()).or_default() += 1;
        }

        let category = entry_category(&e.entry_type);
        *categories
            .entry(category)
            .or_default()
            .entry(entry_type_label(&e.entry_type))
            .or_default() += 1;

        if let Ok(date) = NaiveDate::parse_from_str(&e.occurred_at, "%Y-%m-%d") {
            weekday[date.weekday().num_days_from_monday() as usize] += 1;
            weekly.entry(monday_of(date)).or_default().add(category);
        }

        match e.entry_type.as_str() {
            "pr_authored" => kpis.prs_opened += 1,
            "pr_merged" => kpis.prs_merged += 1,
            "pr_reviewed" => kpis.prs_reviewed += 1,
            "meeting" => kpis.meetings += 1,
            _ => {}
        }
        if is_doc_created(&e.entry_type) {
            kpis.docs_created += 1;
        }
        if is_ticket_opened(&e.entry_type) {
            kpis.tickets_opened += 1;
        }
        if is_ticket_closed(e) {
            kpis.tickets_closed += 1;
        }
    }

    // Zero-filled so charts get a continuous x axis.
    let start = NaiveDate::parse_from_str(from, "%Y-%m-%d").ok();
    let end = NaiveDate::parse_from_str(to, "%Y-%m-%d")
        .ok()
        .map(|d| d.min(today));
    let mut weekly_out = Vec::new();
    if let (Some(start), Some(end)) = (start, end) {
        let mut week = monday_of(start);
        while week <= end {
            weekly_out.push(WeekBucket {
                week_start: week.format("%Y-%m-%d").to_string(),
                by_category: weekly.remove(&week).unwrap_or_default(),
            });
            week += Duration::days(7);
        }
    }
    // Entries dated outside the computed axis (manual future entries) still count.
    let mut leftover: Vec<_> = weekly.into_iter().collect();
    leftover.sort_by_key(|(d, _)| *d);
    weekly_out.extend(leftover.into_iter().map(|(d, by_category)| WeekBucket {
        week_start: d.format("%Y-%m-%d").to_string(),
        by_category,
    }));

    let priority_lookup: HashMap<i64, &Priority> = priorities.iter().map(|p| (p.id, p)).collect();
    let goal_lookup: HashMap<i64, &DepartmentGoal> = goals.iter().map(|g| (g.id, g)).collect();
    let mut priority_share: Vec<PriorityShare> = by_priority
        .into_iter()
        .filter_map(|(id, count)| match id {
            None => Some(PriorityShare {
                priority_id: None,
                title: "No priority assigned".to_string(),
                color: None,
                status: None,
                department_goal_id: None,
                department_goal_title: None,
                department_goal_status: None,
                count,
            }),
            Some(id) => {
                let p = priority_lookup.get(&id)?;
                if p.status == "cancelled" {
                    return None;
                }
                let goal = p.department_goal_id.and_then(|g| goal_lookup.get(&g));
                Some(PriorityShare {
                    priority_id: Some(id),
                    title: p.title.clone(),
                    color: p.color.clone(),
                    status: Some(p.status.clone()),
                    department_goal_id: goal.map(|g| g.id),
                    department_goal_title: goal.map(|g| g.title.clone()),
                    department_goal_status: goal.map(|g| g.status.clone()),
                    count,
                })
            }
        })
        .collect();
    priority_share.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.title.cmp(&b.title)));

    let mut categories: Vec<CategoryCount> = categories
        .into_iter()
        .map(|(key, types)| {
            let types = ranked(
                types.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
                None,
            );
            CategoryCount {
                key: key.to_string(),
                name: category_label(key).unwrap_or("Other").to_string(),
                count: types.iter().map(|t| t.count).sum(),
                types,
            }
        })
        .collect();
    categories.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));

    Ok(Trends {
        kpis,
        weekly: weekly_out,
        categories,
        priority_share,
        impact: ImpactBreakdown {
            reach: ranked(reach, None),
            complexity: ranked(complexity, None),
            role: ranked(role, None),
        },
        people: ranked(people, Some(TOP_N)),
        repos: ranked(repos, Some(TOP_N)),
        teams: ranked(teams, Some(TOP_N)),
        weekday: WEEKDAYS
            .iter()
            .zip(weekday)
            .map(|(name, count)| NameCount {
                name: (*name).to_string(),
                count,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::CategoryTotals;
    use crate::worklog::model::CATEGORIES;

    #[test]
    fn category_totals_have_one_field_per_category() {
        let mut totals = CategoryTotals::default();
        for (slug, _) in CATEGORIES {
            totals.add(slug);
        }
        let json = serde_json::to_value(&totals).unwrap();
        let fields = json.as_object().unwrap();
        assert_eq!(fields.len(), CATEGORIES.len());
        for (slug, _) in CATEGORIES {
            // Each category lands in its own field, not in `other`.
            assert_eq!(fields[*slug], 1, "{slug}");
        }
    }
}
