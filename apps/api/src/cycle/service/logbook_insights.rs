use crate::worklog::model::{BragEntry, EntryType};

/// Delegates to `EntryType::types_for_category`. Public for callers that
/// use the logbook_insights module directly (e.g., trends).
pub fn category_to_entry_types(category: &str) -> Vec<&'static str> {
    EntryType::types_for_category(category)
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
