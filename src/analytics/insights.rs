use std::collections::HashMap;

use crate::entries::model::BragEntry;
use crate::okr::model::KeyResult;

/// Map category name to the set of entry type slugs it contains.
pub fn category_to_entry_types(category: &str) -> Vec<&'static str> {
    match category {
        "reviews" => vec!["pr_reviewed", "revision_reviewed", "code_review"],
        "code" => vec![
            "pr_authored",
            "pr_merged",
            "revision_authored",
            "development",
            "bug_fixed",
            "bug_filed",
            "jira_completed",
            "jira_story",
            "jira_task",
            "jira_epic",
        ],
        "docs" => vec![
            "design_doc",
            "document",
            "confluence_page",
            "drive_created",
            "drive_edited",
            "drive_commented",
        ],
        "meetings" => vec!["meeting"],
        "collaboration" => vec![
            "workshop",
            "mentoring",
            "presentation",
            "pairing",
            "cross_team",
            "interview",
        ],
        "learning" => vec!["learning", "onboarding"],
        _ => vec![],
    }
}

/// Determine which category an entry type belongs to, if any.
fn entry_type_category(entry_type: &str) -> Option<&'static str> {
    match entry_type {
        "pr_reviewed" | "revision_reviewed" | "code_review" => Some("reviews"),
        "pr_authored" | "pr_merged" | "revision_authored" | "development" | "bug_fixed"
        | "bug_filed" | "jira_completed" | "jira_story" | "jira_task" | "jira_epic" => Some("code"),
        "design_doc" | "document" | "confluence_page" | "drive_created" | "drive_edited"
        | "drive_commented" => Some("docs"),
        "meeting" => Some("meetings"),
        "workshop" | "mentoring" | "presentation" | "pairing" | "cross_team" | "interview" => {
            Some("collaboration")
        }
        "learning" | "onboarding" => Some("learning"),
        _ => None,
    }
}

/// Compute insight stats from a filtered set of entries.
pub fn compute_insights(entries: &[BragEntry], key_results: &[KeyResult]) -> serde_json::Value {
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

    // Build KR → goal_id lookup
    let kr_to_goal: HashMap<i64, Option<i64>> =
        key_results.iter().map(|kr| (kr.id, kr.goal_id)).collect();

    let mut goal_counts: HashMap<i64, i64> = HashMap::new();

    for entry in entries {
        match entry_type_category(&entry.entry_type) {
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
            *repo_counts.entry(repo.clone()).or_insert(0) += 1;
        }

        if let Some(ref collabs) = entry.collaborators {
            for c in collabs.split(',') {
                let c = c.trim();
                if !c.is_empty() {
                    *person_counts.entry(c.to_string()).or_insert(0) += 1;
                }
            }
        }

        if let Some(ref teams) = entry.teams {
            for t in teams.split(',') {
                let t = t.trim();
                if !t.is_empty() {
                    *team_counts.entry(t.to_string()).or_insert(0) += 1;
                }
            }
        }

        // Productive days (weekday from occurred_at)
        if let Ok(date) = chrono::NaiveDate::parse_from_str(&entry.occurred_at, "%Y-%m-%d") {
            let weekday = date.format("%A").to_string();
            *weekday_counts.entry(weekday).or_insert(0) += 1;
        }

        // KR and goal breakdowns
        if let Some(kr_id) = entry.key_result_id {
            *kr_counts.entry(kr_id).or_insert(0) += 1;
            if let Some(Some(goal_id)) = kr_to_goal.get(&kr_id) {
                *goal_counts.entry(*goal_id).or_insert(0) += 1;
            }
        } else {
            unlinked_count += 1;
        }
    }

    // Top 5 helpers
    fn top_n(map: &HashMap<String, i64>, n: usize) -> Vec<serde_json::Value> {
        let mut items: Vec<_> = map.iter().collect();
        items.sort_by(|a, b| b.1.cmp(a.1));
        items
            .into_iter()
            .take(n)
            .map(|(k, v)| serde_json::json!({"name": k, "count": v}))
            .collect()
    }

    let top_people = top_n(&person_counts, 5);
    let top_repos = top_n(&repo_counts, 5);
    let top_teams = top_n(&team_counts, 5);

    // Productive days sorted desc
    let mut day_items: Vec<_> = weekday_counts.iter().collect();
    day_items.sort_by(|a, b| b.1.cmp(a.1));
    let productive_days: Vec<serde_json::Value> = day_items
        .into_iter()
        .map(|(k, v)| serde_json::json!({"name": k, "count": v}))
        .collect();

    // Goal breakdown
    let goal_breakdown: Vec<serde_json::Value> = goal_counts
        .iter()
        .map(|(id, count)| serde_json::json!({"id": id, "count": count}))
        .collect();

    // KR breakdown
    let kr_breakdown: Vec<serde_json::Value> = kr_counts
        .iter()
        .map(|(id, count)| serde_json::json!({"id": id, "count": count}))
        .collect();

    serde_json::json!({
        "reviews": reviews,
        "code": code,
        "docs": docs,
        "meetings": meetings,
        "collaboration": collaboration,
        "learning": learning,
        "total": entries.len(),
        "top_people": top_people,
        "top_repos": top_repos,
        "top_teams": top_teams,
        "productive_days": productive_days,
        "goal_breakdown": goal_breakdown,
        "kr_breakdown": kr_breakdown,
        "unlinked_count": unlinked_count,
    })
}

/// A group of entries sharing the same occurred_at date, with a human-readable label.
#[derive(serde::Serialize)]
pub struct DateGroup {
    pub date: String,
    pub label: String,
    pub count: usize,
    pub entries: Vec<BragEntry>,
}

/// Group entries by occurred_at date with human-readable labels.
pub fn build_date_groups(entries: &[BragEntry]) -> Vec<DateGroup> {
    // Get today's date for relative labels
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let yesterday = (chrono::Local::now() - chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();

    // Group entries by date (entries are already sorted by occurred_at DESC)
    let mut groups: Vec<DateGroup> = Vec::new();
    let mut current_date = String::new();

    for entry in entries {
        if entry.occurred_at != current_date {
            current_date = entry.occurred_at.clone();
            let label = format_date_label(&current_date, &today, &yesterday);
            groups.push(DateGroup {
                date: current_date.clone(),
                label,
                count: 0,
                entries: Vec::new(),
            });
        }
        if let Some(group) = groups.last_mut() {
            group.count += 1;
            group.entries.push(entry.clone());
        }
    }

    groups
}

/// Format a date string (YYYY-MM-DD) as a human-readable label.
fn format_date_label(date: &str, today: &str, yesterday: &str) -> String {
    if date == today {
        return "Today".to_string();
    }
    if date == yesterday {
        return "Yesterday".to_string();
    }

    // Parse and format as "Monday, February 9"
    if let Ok(parsed) = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        parsed.format("%A, %B %-d").to_string()
    } else {
        date.to_string()
    }
}

/// Query parameters for analyze filters (string types for URL state round-tripping).
#[derive(serde::Deserialize)]
pub struct AnalyzePageQuery {
    pub goal_id: Option<String>,
    pub key_result_id: Option<String>,
    pub category: Option<String>,
    pub source: Option<String>,
    pub team: Option<String>,
    pub collaborator: Option<String>,
    pub search: Option<String>,
    pub no_key_result: Option<String>,
    pub no_team: Option<String>,
    pub no_collaborator: Option<String>,
}

/// Query parameters for the HTMX-driven filter endpoint (typed fields).
#[derive(serde::Deserialize)]
pub struct AnalyzeFilterQuery {
    pub goal_id: Option<i64>,
    pub key_result_id: Option<i64>,
    pub category: Option<String>,
    pub source: Option<String>,
    pub team: Option<String>,
    pub collaborator: Option<String>,
    pub search: Option<String>,
    pub no_key_result: Option<String>,
    pub no_team: Option<String>,
    pub no_collaborator: Option<String>,
}

/// Apply in-memory filters that can't be done via SQL (encrypted fields, text search, special filters).
pub fn apply_in_memory_filters(entries: &mut Vec<BragEntry>, query: &AnalyzePageQuery) {
    // Text search on title
    if let Some(ref search) = query.search {
        let search = search.trim().to_lowercase();
        if !search.is_empty() {
            entries.retain(|e| e.title.to_lowercase().contains(&search));
        }
    }

    // Filter by team
    let filter_teams: Vec<String> = query
        .team
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if !filter_teams.is_empty() {
        entries.retain(|e| {
            if let Some(ref teams) = e.teams {
                let entry_teams: Vec<&str> = teams.split(',').map(|s| s.trim()).collect();
                filter_teams
                    .iter()
                    .any(|ft| entry_teams.contains(&ft.as_str()))
            } else {
                false
            }
        });
    }

    // Filter by collaborator
    let filter_collabs: Vec<String> = query
        .collaborator
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if !filter_collabs.is_empty() {
        entries.retain(|e| {
            if let Some(ref collabs) = e.collaborators {
                let entry_collabs: Vec<&str> = collabs.split(',').map(|s| s.trim()).collect();
                filter_collabs
                    .iter()
                    .any(|fc| entry_collabs.contains(&fc.as_str()))
            } else {
                false
            }
        });
    }

    // Special: unlinked entries (no key result)
    if query
        .no_key_result
        .as_deref()
        .is_some_and(|s| s == "1" || s == "true")
    {
        entries.retain(|e| e.key_result_id.is_none());
    }

    // Special: missing team
    if query
        .no_team
        .as_deref()
        .is_some_and(|s| s == "1" || s == "true")
    {
        entries.retain(|e| e.teams.as_deref().is_none_or(|t| t.trim().is_empty()));
    }

    // Special: missing collaborator
    if query
        .no_collaborator
        .as_deref()
        .is_some_and(|s| s == "1" || s == "true")
    {
        entries.retain(|e| {
            e.collaborators
                .as_deref()
                .is_none_or(|c| c.trim().is_empty())
        });
    }
}

/// Collect unique comma-separated values from a field across all entries.
pub fn collect_unique_values<F>(entries: &[BragEntry], field: F) -> Vec<String>
where
    F: Fn(&BragEntry) -> Option<&str>,
{
    let mut set = std::collections::HashSet::new();
    for entry in entries {
        if let Some(val) = field(entry) {
            for item in val.split(',') {
                let item = item.trim();
                if !item.is_empty() {
                    set.insert(item.to_string());
                }
            }
        }
    }
    set.into_iter().collect()
}
