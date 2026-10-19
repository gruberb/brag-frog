//! The Logbook filter set, shared by the entries list, reports and anything else that
//! selects "the entries the user is looking at".

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::worklog::model::{BragEntry, EntryType};

/// Which entries to look at. Every field is optional; comma-separated fields hold several
/// values. The same keys are used as query parameters and in saved reports.
#[derive(Debug, Default, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct LogbookFilter {
    /// Inclusive `YYYY-MM-DD`. Defaults to the cycle start.
    #[serde(default)]
    #[ts(optional)]
    pub from: Option<String>,
    /// Inclusive `YYYY-MM-DD`. Defaults to the cycle end.
    #[serde(default)]
    #[ts(optional)]
    pub to: Option<String>,
    /// Comma-separated sources (`github,phabricator`).
    #[serde(default)]
    #[ts(optional)]
    pub source: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub priority_id: Option<i64>,
    #[serde(default)]
    #[ts(optional)]
    pub goal_id: Option<i64>,
    /// Entry type category: a slug from `worklog::model::CATEGORIES`.
    #[serde(default)]
    #[ts(optional)]
    pub category: Option<String>,
    /// Case-insensitive title search.
    #[serde(default)]
    #[ts(optional)]
    pub q: Option<String>,
    /// Comma-separated team names.
    #[serde(default)]
    #[ts(optional)]
    pub team: Option<String>,
    /// Comma-separated collaborator names (after alias resolution).
    #[serde(default)]
    #[ts(optional)]
    pub person: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub reach: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub complexity: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub role: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub no_priority: Option<bool>,
    #[serde(default)]
    #[ts(optional)]
    pub no_team: Option<bool>,
    #[serde(default)]
    #[ts(optional)]
    pub no_person: Option<bool>,
}

fn split_csv(v: &Option<String>) -> Vec<String> {
    v.as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn trimmed(v: &Option<String>) -> Option<&str> {
    v.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

fn has_any(csv: &Option<String>, wanted: &[String]) -> bool {
    let have: Vec<&str> = csv.as_deref().unwrap_or("").split(',').map(str::trim).collect();
    wanted.iter().any(|w| have.contains(&w.as_str()))
}

fn is_blank(v: &Option<String>) -> bool {
    v.as_deref().is_none_or(|s| s.trim().is_empty())
}

impl LogbookFilter {
    /// Source names the SQL query filters on.
    pub fn sources(&self) -> Vec<String> {
        split_csv(&self.source)
    }

    /// Applies what SQL cannot do (title, teams and collaborators are encrypted): text search,
    /// teams, people, reach, complexity, role, entry category and the "missing X" views.
    pub fn apply_in_memory(&self, entries: &mut Vec<BragEntry>) {
        if let Some(search) = trimmed(&self.q) {
            let search = search.to_lowercase();
            entries.retain(|e| e.title.to_lowercase().contains(&search));
        }
        let teams = split_csv(&self.team);
        if !teams.is_empty() {
            entries.retain(|e| has_any(&e.teams, &teams));
        }
        let people = split_csv(&self.person);
        if !people.is_empty() {
            entries.retain(|e| has_any(&e.collaborators, &people));
        }
        if let Some(reach) = trimmed(&self.reach) {
            entries.retain(|e| e.reach.as_deref() == Some(reach));
        }
        if let Some(complexity) = trimmed(&self.complexity) {
            entries.retain(|e| e.complexity.as_deref() == Some(complexity));
        }
        if let Some(role) = trimmed(&self.role) {
            entries.retain(|e| e.role.as_deref() == Some(role));
        }
        if let Some(category) = trimmed(&self.category) {
            let allowed = EntryType::types_for_category(category);
            entries.retain(|e| allowed.contains(&e.entry_type.as_str()));
        }
        if self.no_priority == Some(true) {
            entries.retain(|e| e.priority_id.is_none());
        }
        if self.no_team == Some(true) {
            entries.retain(|e| is_blank(&e.teams));
        }
        if self.no_person == Some(true) {
            entries.retain(|e| is_blank(&e.collaborators));
        }
    }
}
