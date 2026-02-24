use axum::extract::State;
use axum::response::Html;
use std::collections::HashMap;

use crate::AppState;
use crate::entries::model::BragEntry;
use crate::identity::auth::middleware::AuthUser;
use crate::identity::model::User;
use crate::goals::model::{DepartmentGoal, Priority};
use crate::review::model::BragPhase;
use crate::shared::error::AppError;

fn category_color(cat: &str) -> &'static str {
    match cat {
        "Code" => "#4A90D9",
        "Reviews" => "#9B59B6",
        "Docs" => "#27AE60",
        "Meetings" => "#E67E22",
        "Collaboration" => "#16A085",
        "Decisions" => "#E74C3C",
        _ => "#95A5A6",
    }
}

fn build_kr_category_json(
    kr_categories: &HashMap<i64, HashMap<&str, usize>>,
    kr_id: i64,
) -> Vec<serde_json::Value> {
    let Some(cats) = kr_categories.get(&kr_id) else {
        return vec![];
    };
    let mut sorted: Vec<_> = cats.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    sorted
        .into_iter()
        .map(|(name, count)| {
            serde_json::json!({
                "name": name,
                "count": count,
                "color": category_color(name),
            })
        })
        .collect()
}

fn entry_category(entry_type: &str) -> &'static str {
    match entry_type {
        "pr_reviewed" | "revision_reviewed" | "code_review" => "Reviews",
        "pr_authored" | "pr_merged" | "pr_development" | "revision_authored" | "development"
        | "bug_fixed" | "bug_filed" | "jira_completed" | "jira_story" | "jira_task"
        | "jira_epic" => "Code",
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

    let entries = BragEntry::list_for_phase_in_range(
        &state.db,
        phase.id,
        &phase.start_date,
        &phase.end_date,
        &auth.crypto,
    )
    .await?;

    // Entries by Department Goal — group entries by dept goal (via priority_id → department_goal_id)
    let dept_goals = DepartmentGoal::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let all_priorities = Priority::list_for_phase(&state.db, phase.id, &auth.crypto).await?;

    // Count entries per priority and per-priority category breakdown
    let mut priority_counts: HashMap<i64, usize> = HashMap::new();
    let mut priority_categories: HashMap<i64, HashMap<&str, usize>> = HashMap::new();
    for entry in &entries {
        if let Some(pri_id) = entry.priority_id {
            *priority_counts.entry(pri_id).or_insert(0) += 1;
            let cat = entry_category(&entry.entry_type);
            *priority_categories
                .entry(pri_id)
                .or_default()
                .entry(cat)
                .or_insert(0) += 1;
        }
    }

    // Build goal cards: each dept goal with its priorities showing progress + entry count
    let mut goal_cards: Vec<serde_json::Value> = dept_goals
        .iter()
        .filter_map(|g| {
            let pris: Vec<serde_json::Value> = all_priorities
                .iter()
                .filter(|p| p.department_goal_id == Some(g.id) && p.status != "cancelled")
                .map(|p| {
                    let entry_count = priority_counts.get(&p.id).copied().unwrap_or(0);
                    let categories = build_kr_category_json(&priority_categories, p.id);
                    serde_json::json!({
                        "name": p.title,
                        "color": p.color.as_deref().unwrap_or("#888"),
                        "progress": p.progress,
                        "entry_count": entry_count,
                        "categories": categories,
                    })
                })
                .collect();
            // Filter out goals where all priorities have 0% progress AND 0 entries
            let has_activity = pris.iter().any(|p| {
                p["progress"].as_i64().unwrap_or(0) > 0
                    || p["entry_count"].as_u64().unwrap_or(0) > 0
            });
            if !has_activity {
                return None;
            }
            Some(serde_json::json!({
                "title": g.title,
                "krs": pris,
            }))
        })
        .collect();

    // Include standalone priorities (not linked to any dept goal) that have activity
    let standalone_priorities: Vec<serde_json::Value> = all_priorities
        .iter()
        .filter(|p| p.department_goal_id.is_none() && p.status != "cancelled")
        .filter_map(|p| {
            let entry_count = priority_counts.get(&p.id).copied().unwrap_or(0);
            if p.progress == 0 && entry_count == 0 {
                return None;
            }
            let categories = build_kr_category_json(&priority_categories, p.id);
            Some(serde_json::json!({
                "name": p.title,
                "color": p.color.as_deref().unwrap_or("#888"),
                "progress": p.progress,
                "entry_count": entry_count,
                "categories": categories,
            }))
        })
        .collect();
    if !standalone_priorities.is_empty() {
        goal_cards.push(serde_json::json!({
            "title": "Unassigned Priorities",
            "krs": standalone_priorities,
        }));
    }

    // Category distribution — no "Other" bucket, use specific categories
    let mut category_counts: HashMap<&str, usize> = HashMap::new();
    for entry in &entries {
        let cat = entry_category(&entry.entry_type);
        *category_counts.entry(cat).or_insert(0) += 1;
    }
    let max_category_count = category_counts.values().max().copied().unwrap_or(1) as f64;
    let mut category_bars: Vec<serde_json::Value> = category_counts
        .iter()
        .map(|(k, v)| {
            let pct = (*v as f64 / max_category_count * 100.0).round() as i64;
            serde_json::json!({"name": k, "count": v, "pct": pct})
        })
        .collect();
    category_bars.sort_by(|a, b| {
        b["count"]
            .as_u64()
            .unwrap_or(0)
            .cmp(&a["count"].as_u64().unwrap_or(0))
    });

    // Per-source breakdowns for code review and ticket activity
    let prs_opened = entries
        .iter()
        .filter(|e| e.entry_type == "pr_authored" || e.entry_type == "revision_authored")
        .count();
    let prs_reviewed = entries
        .iter()
        .filter(|e| e.entry_type == "pr_reviewed" || e.entry_type == "revision_reviewed")
        .count();
    let prs_merged = entries
        .iter()
        .filter(|e| e.entry_type == "pr_merged")
        .count();

    // Per-source breakdowns
    let gh_prs_opened = entries
        .iter()
        .filter(|e| e.source == "github" && e.entry_type == "pr_authored")
        .count();
    let phab_revisions_authored = entries
        .iter()
        .filter(|e| e.source == "phabricator" && e.entry_type == "revision_authored")
        .count();

    // Ticket activity
    let jira_completed = entries
        .iter()
        .filter(|e| e.entry_type == "jira_completed")
        .count();
    let jira_created = entries
        .iter()
        .filter(|e| {
            e.source == "jira"
                && matches!(
                    e.entry_type.as_str(),
                    "jira_story" | "jira_task" | "jira_epic"
                )
        })
        .count();
    let bugs_filed = entries
        .iter()
        .filter(|e| e.entry_type == "bug_filed")
        .count();
    let bugs_fixed = entries
        .iter()
        .filter(|e| e.entry_type == "bug_fixed")
        .count();

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("phases", &phases);
    ctx.insert("total_entries", &entries.len());
    ctx.insert("goal_cards", &goal_cards);
    ctx.insert("category_bars", &category_bars);
    ctx.insert("prs_opened", &prs_opened);
    ctx.insert("prs_reviewed", &prs_reviewed);
    ctx.insert("prs_merged", &prs_merged);
    ctx.insert("gh_prs_opened", &gh_prs_opened);
    ctx.insert("phab_revisions_authored", &phab_revisions_authored);
    ctx.insert("jira_completed", &jira_completed);
    ctx.insert("jira_created", &jira_created);
    ctx.insert("bugs_filed", &bugs_filed);
    ctx.insert("bugs_fixed", &bugs_fixed);
    ctx.insert("current_page", "trends");

    let html = state.templates.render("pages/trends.html", &ctx)?;
    Ok(Html(html))
}
