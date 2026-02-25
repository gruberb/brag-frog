use std::collections::HashMap;

use crate::objectives::model::Priority;
use crate::worklog::model::{BragEntry, EntryType};

/// Compute insight statistics from a filtered set of entries.
/// Returns a JSON value with category counts, repo/person/team breakdowns,
/// priority alignment, and other analytics.
pub fn compute_insights(entries: &[BragEntry], priorities: &[Priority]) -> serde_json::Value {
    let mut reviews = 0i64;
    let mut code = 0i64;
    let mut docs = 0i64;
    let mut meetings = 0i64;
    let mut collaboration = 0i64;
    let mut learning = 0i64;

    let mut repo_counts: HashMap<String, i64> = HashMap::new();
    let mut person_counts: HashMap<String, i64> = HashMap::new();
    let mut team_counts: HashMap<String, i64> = HashMap::new();
    let mut weekday_counts: HashMap<String, i64> = HashMap::new();
    let mut kr_counts: HashMap<i64, i64> = HashMap::new();
    let mut unlinked_count: i64 = 0;

    let priority_to_dept_goal: HashMap<i64, Option<i64>> =
        priorities.iter().map(|p| (p.id, p.department_goal_id)).collect();

    let mut goal_counts: HashMap<i64, i64> = HashMap::new();
    let mut reach_counts: HashMap<String, i64> = HashMap::new();
    let mut complexity_counts: HashMap<String, i64> = HashMap::new();
    let mut role_counts: HashMap<String, i64> = HashMap::new();

    for entry in entries {
        match EntryType::category_for_slug(&entry.entry_type) {
            Some("reviews") => reviews += 1,
            Some("code") => code += 1,
            Some("docs") => docs += 1,
            Some("meetings") => meetings += 1,
            Some("collaboration") => collaboration += 1,
            Some("learning") => learning += 1,
            _ => {}
        }

        if let Some(ref repo) = entry.repository
            && !repo.is_empty()
            && !repo.starts_with("http://")
            && !repo.starts_with("https://")
        {
            *repo_counts.entry(repo.clone()).or_default() += 1;
        }

        if let Some(ref collabs) = entry.collaborators {
            for name in collabs.split(',') {
                let t = name.trim();
                if !t.is_empty() {
                    *person_counts.entry(t.to_string()).or_default() += 1;
                }
            }
        }

        if let Some(ref teams) = entry.teams {
            for name in teams.split(',') {
                let t = name.trim();
                if !t.is_empty() {
                    *team_counts.entry(t.to_string()).or_default() += 1;
                }
            }
        }

        if let Ok(date) = chrono::NaiveDate::parse_from_str(&entry.occurred_at, "%Y-%m-%d") {
            let wd = date.format("%A").to_string();
            *weekday_counts.entry(wd).or_default() += 1;
        }

        if let Some(pid) = entry.priority_id {
            *kr_counts.entry(pid).or_default() += 1;
            if let Some(Some(gid)) = priority_to_dept_goal.get(&pid) {
                *goal_counts.entry(*gid).or_default() += 1;
            }
        } else {
            unlinked_count += 1;
        }

        if let Some(ref reach) = entry.reach
            && !reach.is_empty()
        {
            *reach_counts.entry(reach.clone()).or_default() += 1;
        }
        if let Some(ref complexity) = entry.complexity
            && !complexity.is_empty()
        {
            *complexity_counts.entry(complexity.clone()).or_default() += 1;
        }
        if let Some(ref role) = entry.role
            && !role.is_empty()
        {
            *role_counts.entry(role.clone()).or_default() += 1;
        }
    }

    let top_repos = top_n_sorted(&repo_counts, 5);
    let top_people = top_n_sorted(&person_counts, 5);
    let top_teams = top_n_sorted(&team_counts, 5);

    let total = entries.len() as i64;

    serde_json::json!({
        "total": total,
        "reviews": reviews,
        "code": code,
        "docs": docs,
        "meetings": meetings,
        "collaboration": collaboration,
        "learning": learning,
        "top_repos": top_repos,
        "top_people": top_people,
        "top_teams": top_teams,
        "weekday_counts": weekday_counts,
        "priority_counts": kr_counts,
        "goal_counts": goal_counts,
        "unlinked_count": unlinked_count,
        "reach_counts": reach_counts,
        "complexity_counts": complexity_counts,
        "role_counts": role_counts,
    })
}

/// Sort a map by value descending and take top N.
fn top_n_sorted(map: &HashMap<String, i64>, n: usize) -> Vec<(String, i64)> {
    let mut items: Vec<(String, i64)> = map.iter().map(|(k, v)| (k.clone(), *v)).collect();
    items.sort_by(|a, b| b.1.cmp(&a.1));
    items.truncate(n);
    items
}

/// Apply in-memory filters to entries: source, type, category, priority, teams, role.
pub fn apply_in_memory_filters(
    entries: Vec<BragEntry>,
    source: Option<&str>,
    entry_type: Option<&str>,
    category: Option<&str>,
    priority_id: Option<i64>,
    teams: Option<&str>,
    role: Option<&str>,
) -> Vec<BragEntry> {
    let mut result = entries;

    if let Some(src) = source {
        result.retain(|e| e.source == src);
    }

    if let Some(et) = entry_type {
        result.retain(|e| e.entry_type == et);
    }

    if let Some(cat) = category {
        let types = EntryType::types_for_category(cat);
        if !types.is_empty() {
            result.retain(|e| types.contains(&e.entry_type.as_str()));
        }
    }

    if let Some(pid) = priority_id {
        if pid == 0 {
            result.retain(|e| e.priority_id.is_none());
        } else {
            result.retain(|e| e.priority_id == Some(pid));
        }
    }

    if let Some(team) = teams {
        result.retain(|e| {
            e.teams
                .as_ref()
                .is_some_and(|t| t.split(',').any(|part| part.trim() == team))
        });
    }

    if let Some(role) = role {
        result.retain(|e| e.role.as_deref() == Some(role));
    }

    result
}
