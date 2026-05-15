use axum::extract::State;
use axum::response::Html;
use std::collections::HashMap;

use crate::AppState;
use crate::cycle::model::BragPhase;
use crate::identity::auth::middleware::AuthUser;
use crate::identity::model::{PeopleAlias, User};
use crate::kernel::error::AppError;
use crate::objectives::model::{DepartmentGoal, Priority};
use crate::worklog::model::BragEntry;

fn category_color(cat: &str) -> &'static str {
    match cat {
        "Code" => "#4A90D9",
        "Reviews" => "#9B59B6",
        "Docs" => "#27AE60",
        "Meetings" => "#E67E22",
        "Collaboration" => "#16A085",
        "Decisions" => "#E74C3C",
        "Tickets" => "#F39C12",
        _ => "#95A5A6",
    }
}

fn entry_category(entry_type: &str) -> &'static str {
    match entry_type {
        "pr_reviewed" | "revision_reviewed" | "code_review" => "Reviews",
        "pr_authored" | "pr_merged" | "pr_development" | "revision_authored" | "development" => {
            "Code"
        }
        "bug_fixed" | "bug_filed" | "jira_completed" | "jira_story" | "jira_task" | "jira_epic" => {
            "Tickets"
        }
        "design_doc" | "document" | "confluence_page" | "drive_created" | "drive_edited"
        | "drive_commented" => "Docs",
        "meeting" => "Meetings",
        "workshop" | "mentoring" | "presentation" | "pairing" | "cross_team" | "interview" => {
            "Collaboration"
        }
        "learning" | "onboarding" => "Learning",
        "decision" => "Decisions",
        "process_improvement" => "Process",
        "unblocking" => "Unblocking",
        "incident_response" => "Incidents",
        "investigation" => "Investigation",
        "other" => "Uncategorized",
        _ => "Uncategorized",
    }
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

fn is_docs_work_type(entry_type: &str) -> bool {
    matches!(
        entry_type,
        "design_doc"
            | "document"
            | "confluence_page"
            | "drive_created"
            | "drive_edited"
            | "drive_commented"
    )
}

fn humanize_label(val: &str) -> String {
    match val {
        "cross_team" => "Cross-team".to_string(),
        other => {
            let mut c = other.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().to_string() + c.as_str(),
            }
        }
    }
}

/// Build sorted horizontal bar data from a counts map, normalized to max value.
fn build_bars(counts: &HashMap<String, usize>, limit: Option<usize>) -> Vec<serde_json::Value> {
    let mut items: Vec<_> = counts.iter().collect();
    items.sort_by(|a, b| b.1.cmp(a.1));
    if let Some(n) = limit {
        items.truncate(n);
    }
    let max = items.first().map(|(_, v)| **v).unwrap_or(1) as f64;
    items
        .into_iter()
        .map(|(k, v)| {
            let pct = (*v as f64 / max * 100.0).round() as i64;
            serde_json::json!({"name": humanize_label(k), "count": v, "pct": pct})
        })
        .collect()
}

/// Trends page — cross-phase analytics with CSS-only visualizations.
pub async fn trends_page(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let phases = BragPhase::list_for_user(&state.db, auth.user_id).await?;
    let phase = BragPhase::get_active(&state.db, auth.user_id).await?;

    if phase.is_none() {
        let mut ctx = tera::Context::new();
        ctx.insert("user", &user);
        ctx.insert("current_page", "trends");
        let html = state.templates.render("pages/no_phase.html", &ctx)?;
        return Ok(Html(html));
    }
    let phase = phase.unwrap();

    let mut entries = BragEntry::list_for_phase_in_range(
        &state.db,
        phase.id,
        &phase.start_date,
        &phase.end_date,
        &auth.crypto,
    )
    .await?;

    // Match logbook: hide future synced entries (e.g. upcoming calendar meetings)
    let today_str = chrono::Local::now().format("%Y-%m-%d").to_string();
    entries.retain(|e| e.source == "manual" || e.occurred_at.as_str() <= today_str.as_str());

    let all_priorities = Priority::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let department_goals =
        DepartmentGoal::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let alias_map = PeopleAlias::alias_map(&state.db, auth.user_id).await?;

    // Priority activity counts keyed by priority, plus an "Unlinked" bucket.
    let mut priority_counts: HashMap<i64, usize> = HashMap::new();
    let mut unlinked_count: usize = 0;

    // Impact signal breakdowns
    let mut reach_counts: HashMap<String, usize> = HashMap::new();
    let mut complexity_counts: HashMap<String, usize> = HashMap::new();
    let mut role_counts: HashMap<String, usize> = HashMap::new();
    let mut person_counts: HashMap<String, usize> = HashMap::new();

    // New metrics: repos, teams, weekday productivity, requested overview counters
    let mut repo_counts: HashMap<String, usize> = HashMap::new();
    let mut team_counts: HashMap<String, usize> = HashMap::new();
    let mut weekday_counts: HashMap<String, usize> = HashMap::new();
    let mut overview_prs_merged: usize = 0;
    let mut overview_prs_reviewed: usize = 0;
    let mut overview_jira_tickets_closed: usize = 0;
    let mut overview_docs_worked_on: usize = 0;
    let mut overview_meetings: usize = 0;

    // Category distribution + per-category type breakdown for tooltips
    let mut category_counts: HashMap<&str, usize> = HashMap::new();
    let mut category_type_counts: HashMap<&str, HashMap<&str, usize>> = HashMap::new();

    for entry in &entries {
        // Priority tracking
        if let Some(pri_id) = entry.priority_id {
            *priority_counts.entry(pri_id).or_insert(0) += 1;
        } else {
            unlinked_count += 1;
        }

        // Impact signals
        if let Some(ref reach) = entry.reach
            && !reach.is_empty()
        {
            *reach_counts.entry(reach.clone()).or_insert(0) += 1;
        }
        if let Some(ref complexity) = entry.complexity
            && !complexity.is_empty()
        {
            *complexity_counts.entry(complexity.clone()).or_insert(0) += 1;
        }
        if let Some(ref role) = entry.role
            && !role.is_empty()
        {
            *role_counts.entry(role.clone()).or_insert(0) += 1;
        }

        // Collaborators (apply alias map for display names)
        if let Some(ref collabs) = entry.collaborators {
            for c in collabs.split(',') {
                let c = c.trim();
                if !c.is_empty() {
                    let display = alias_map
                        .get(&c.to_lowercase())
                        .cloned()
                        .unwrap_or_else(|| c.to_string());
                    *person_counts.entry(display).or_insert(0) += 1;
                }
            }
        }

        // Repos
        if let Some(ref repo) = entry.repository
            && !repo.is_empty()
            && !repo.starts_with("http://")
            && !repo.starts_with("https://")
        {
            *repo_counts.entry(repo.clone()).or_insert(0) += 1;
        }

        // Teams
        if let Some(ref teams) = entry.teams {
            for t in teams.split(',') {
                let t = t.trim();
                if !t.is_empty() {
                    *team_counts.entry(t.to_string()).or_insert(0) += 1;
                }
            }
        }

        // Weekday productivity
        if let Ok(date) = chrono::NaiveDate::parse_from_str(&entry.occurred_at, "%Y-%m-%d") {
            let weekday = date.format("%A").to_string();
            *weekday_counts.entry(weekday).or_insert(0) += 1;
        }

        // Requested overview counters
        match entry.entry_type.as_str() {
            "pr_merged" => overview_prs_merged += 1,
            "pr_reviewed" => overview_prs_reviewed += 1,
            "jira_completed" => overview_jira_tickets_closed += 1,
            "meeting" => overview_meetings += 1,
            _ => {}
        }
        if is_docs_work_type(&entry.entry_type) {
            overview_docs_worked_on += 1;
        }

        // Category + type breakdown
        let cat = entry_category(&entry.entry_type);
        *category_counts.entry(cat).or_insert(0) += 1;
        let type_label = entry_type_label(&entry.entry_type);
        *category_type_counts
            .entry(cat)
            .or_default()
            .entry(type_label)
            .or_insert(0) += 1;
    }

    // Build priority activity grouped by department goal.
    let priority_lookup: HashMap<i64, &Priority> =
        all_priorities.iter().map(|p| (p.id, p)).collect();
    let department_goal_lookup: HashMap<i64, &DepartmentGoal> = department_goals
        .iter()
        .map(|goal| (goal.id, goal))
        .collect();
    let mut grouped_priority_counts: HashMap<Option<i64>, Vec<serde_json::Value>> = HashMap::new();

    for (pri_id, count) in &priority_counts {
        let Some(priority) = priority_lookup.get(pri_id) else {
            continue;
        };
        if priority.status == "cancelled" {
            continue;
        }
        let department_goal_id = priority
            .department_goal_id
            .filter(|id| department_goal_lookup.contains_key(id));

        grouped_priority_counts
            .entry(department_goal_id)
            .or_default()
            .push(serde_json::json!({
                "name": priority.title.as_str(),
                "color": priority.color.as_deref().unwrap_or("#888"),
                "count": count,
            }));
    }

    let mut priority_activity_groups: Vec<serde_json::Value> = Vec::new();
    for (department_goal_id, mut priorities) in grouped_priority_counts {
        priorities.sort_by(|a, b| {
            b["count"]
                .as_u64()
                .unwrap_or(0)
                .cmp(&a["count"].as_u64().unwrap_or(0))
                .then_with(|| {
                    a["name"]
                        .as_str()
                        .unwrap_or("")
                        .cmp(b["name"].as_str().unwrap_or(""))
                })
        });

        let group_count: u64 = priorities
            .iter()
            .map(|item| item["count"].as_u64().unwrap_or(0))
            .sum();
        let child_max = priorities
            .first()
            .and_then(|item| item["count"].as_u64())
            .unwrap_or(1) as f64;

        for priority in &mut priorities {
            let count = priority["count"].as_u64().unwrap_or(0) as f64;
            priority.as_object_mut().unwrap().insert(
                "pct".to_string(),
                serde_json::json!((count / child_max * 100.0).round() as i64),
            );
        }

        let (name, status) = department_goal_id
            .and_then(|id| department_goal_lookup.get(&id).copied())
            .map(|goal| (goal.title.as_str(), Some(goal.status.as_str())))
            .unwrap_or(("No Department Goal", None));

        priority_activity_groups.push(serde_json::json!({
            "name": name,
            "status": status,
            "color": "#FF453F",
            "count": group_count,
            "priorities": priorities,
        }));
    }

    if unlinked_count > 0 {
        priority_activity_groups.push(serde_json::json!({
            "name": "Unlinked Entries",
            "status": null,
            "color": "#95A5A6",
            "count": unlinked_count,
            "priorities": [{
                "name": "No priority assigned",
                "color": "#95A5A6",
                "count": unlinked_count,
                "pct": 100,
            }],
        }));
    }
    priority_activity_groups.sort_by(|a, b| {
        b["count"]
            .as_u64()
            .unwrap_or(0)
            .cmp(&a["count"].as_u64().unwrap_or(0))
            .then_with(|| {
                a["name"]
                    .as_str()
                    .unwrap_or("")
                    .cmp(b["name"].as_str().unwrap_or(""))
            })
    });

    let max_department_goal_count = priority_activity_groups
        .first()
        .and_then(|group| group["count"].as_u64())
        .unwrap_or(1) as f64;
    for group in &mut priority_activity_groups {
        let count = group["count"].as_u64().unwrap_or(0) as f64;
        group.as_object_mut().unwrap().insert(
            "pct".to_string(),
            serde_json::json!((count / max_department_goal_count * 100.0).round() as i64),
        );
    }

    // Impact signal bars
    let reach_bars = build_bars(&reach_counts, None);
    let complexity_bars = build_bars(&complexity_counts, None);
    let role_bars = build_bars(&role_counts, None);
    let collaborator_bars = build_bars(&person_counts, Some(8));

    // Category distribution bars with per-type tooltip
    let max_category_count = category_counts.values().max().copied().unwrap_or(1) as f64;
    let mut category_bars: Vec<serde_json::Value> = category_counts
        .iter()
        .map(|(k, v)| {
            let pct = (*v as f64 / max_category_count * 100.0).round() as i64;
            // Build tooltip showing per-type breakdown (sorted by count desc)
            let tooltip = if let Some(types) = category_type_counts.get(k) {
                let mut items: Vec<_> = types.iter().collect();
                items.sort_by(|a, b| b.1.cmp(a.1));
                items
                    .iter()
                    .map(|(label, count)| format!("{}: {}", label, count))
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                String::new()
            };
            serde_json::json!({"name": k, "count": v, "pct": pct, "color": category_color(k), "tooltip": tooltip})
        })
        .collect();
    category_bars.sort_by(|a, b| {
        b["count"]
            .as_u64()
            .unwrap_or(0)
            .cmp(&a["count"].as_u64().unwrap_or(0))
    });

    // New bar charts
    let repo_bars = build_bars(&repo_counts, Some(8));
    let team_bars = build_bars(&team_counts, Some(8));
    let weekday_bars = build_bars(&weekday_counts, None);

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("phases", &phases);
    ctx.insert("total_entries", &entries.len());
    ctx.insert("priority_activity_groups", &priority_activity_groups);
    ctx.insert("category_bars", &category_bars);
    ctx.insert("reach_bars", &reach_bars);
    ctx.insert("complexity_bars", &complexity_bars);
    ctx.insert("role_bars", &role_bars);
    ctx.insert("collaborator_bars", &collaborator_bars);
    ctx.insert("repo_bars", &repo_bars);
    ctx.insert("team_bars", &team_bars);
    ctx.insert("weekday_bars", &weekday_bars);
    ctx.insert("overview_total", &entries.len());
    ctx.insert("overview_prs_merged", &overview_prs_merged);
    ctx.insert("overview_prs_reviewed", &overview_prs_reviewed);
    ctx.insert(
        "overview_jira_tickets_closed",
        &overview_jira_tickets_closed,
    );
    ctx.insert("overview_docs_worked_on", &overview_docs_worked_on);
    ctx.insert("overview_meetings", &overview_meetings);
    ctx.insert("current_page", "trends");

    let html = state.templates.render("pages/trends.html", &ctx)?;
    Ok(Html(html))
}
