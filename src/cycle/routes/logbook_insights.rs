use crate::worklog::model::{BragEntry, EntryType};

/// Delegates to `EntryType::types_for_category`. Public for callers that
/// use the logbook_insights module directly (e.g., trends).
pub fn category_to_entry_types(category: &str) -> Vec<&'static str> {
    EntryType::types_for_category(category)
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
    pub department_goal_id: Option<String>,
    pub priority_id: Option<String>,
    pub category: Option<String>,
    pub source: Option<String>,
    pub team: Option<String>,
    pub collaborator: Option<String>,
    pub search: Option<String>,
    pub no_priority: Option<String>,
    pub no_team: Option<String>,
    pub no_collaborator: Option<String>,
    pub reach: Option<String>,
    pub complexity: Option<String>,
    pub role: Option<String>,
}

/// Query parameters for the HTMX-driven filter endpoint (typed fields).
#[derive(serde::Deserialize)]
pub struct AnalyzeFilterQuery {
    pub department_goal_id: Option<i64>,
    pub priority_id: Option<i64>,
    pub category: Option<String>,
    pub source: Option<String>,
    pub team: Option<String>,
    pub collaborator: Option<String>,
    pub search: Option<String>,
    pub no_priority: Option<String>,
    pub no_team: Option<String>,
    pub no_collaborator: Option<String>,
    pub reach: Option<String>,
    pub complexity: Option<String>,
    pub role: Option<String>,
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

    // Filter by reach
    if let Some(ref reach) = query.reach {
        let reach = reach.trim();
        if !reach.is_empty() {
            entries.retain(|e| e.reach.as_deref() == Some(reach));
        }
    }

    // Filter by complexity
    if let Some(ref complexity) = query.complexity {
        let complexity = complexity.trim();
        if !complexity.is_empty() {
            entries.retain(|e| e.complexity.as_deref() == Some(complexity));
        }
    }

    // Filter by role
    if let Some(ref role) = query.role {
        let role = role.trim();
        if !role.is_empty() {
            entries.retain(|e| e.role.as_deref() == Some(role));
        }
    }

    // Special: unlinked entries (no priority)
    if query
        .no_priority
        .as_deref()
        .is_some_and(|s| s == "1" || s == "true")
    {
        entries.retain(|e| e.priority_id.is_none());
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
