use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;

use crate::AppState;
use crate::entries::model::BragEntry;
use crate::entries::model::EntryType;
use crate::identity::auth::middleware::AuthUser;
use crate::identity::model::User;
use crate::okr::model::{Goal, KeyResult};
use crate::review::model::{BragPhase, Week};
use crate::shared::error::AppError;

/// Renders the export page with phase selection and format options.
pub async fn export_page(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let phase = BragPhase::get_active(&state.db, auth.user_id).await?;
    let phases = BragPhase::list_for_user(&state.db, auth.user_id).await?;

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("phases", &phases);
    ctx.insert("current_page", "export");

    let html = state.templates.render("pages/export.html", &ctx)?;
    Ok(Html(html))
}

/// Query parameters for the download endpoint: phase, format (json/markdown), and goal inclusion.
#[derive(Debug, Deserialize)]
pub struct ExportParams {
    pub phase_id: i64,
    pub format: String,
    #[serde(default)]
    pub include_goals: Option<String>,
}

/// Generates and serves a downloadable brag document (Markdown or JSON) for a phase.
pub async fn export_download(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<ExportParams>,
) -> Result<Response, AppError> {
    let phase = BragPhase::find_by_id(&state.db, params.phase_id, auth.user_id)
        .await?
        .ok_or(AppError::NotFound("Phase not found".to_string()))?;

    let include_goals = params.include_goals.as_deref() == Some("true");

    // Load data
    let entries = BragEntry::list_for_phase_in_range(
        &state.db,
        phase.id,
        &phase.start_date,
        &phase.end_date,
        &auth.crypto,
    )
    .await?;

    let weeks = Week::list_for_phase(&state.db, phase.id).await?;
    let key_results = KeyResult::list_for_user(&state.db, auth.user_id).await?;
    let goals = Goal::list_for_phase(&state.db, phase.id, &auth.crypto).await?;

    // Build lookup maps
    let kr_map: HashMap<i64, &KeyResult> = key_results.iter().map(|kr| (kr.id, kr)).collect();
    let goal_map: HashMap<i64, &Goal> = goals.iter().map(|g| (g.id, g)).collect();
    let week_map: HashMap<i64, &Week> = weeks.iter().map(|w| (w.id, w)).collect();

    // Group entries by week_id
    let mut entries_by_week: HashMap<i64, Vec<&BragEntry>> = HashMap::new();
    for entry in &entries {
        entries_by_week
            .entry(entry.week_id)
            .or_default()
            .push(entry);
    }

    // Sort entries within each week by occurred_at
    for entries in entries_by_week.values_mut() {
        entries.sort_by(|a, b| a.occurred_at.cmp(&b.occurred_at));
    }

    // Sort weeks descending (most recent first)
    let mut sorted_weeks = weeks.clone();
    sorted_weeks.sort_by(|a, b| b.year.cmp(&a.year).then(b.iso_week.cmp(&a.iso_week)));

    // Sanitize phase name for filename
    let safe_name: String = phase
        .name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();

    match params.format.as_str() {
        "json" => {
            let json = build_json(
                &phase,
                &sorted_weeks,
                &entries_by_week,
                &goals,
                &key_results,
                &kr_map,
                &goal_map,
                include_goals,
            );
            let filename = format!("brag-frog-{}.json", safe_name);
            Ok((
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "application/json".to_string()),
                    (
                        header::CONTENT_DISPOSITION,
                        format!("attachment; filename=\"{}\"", filename),
                    ),
                ],
                json,
            )
                .into_response())
        }
        _ => {
            // Default to markdown
            let md = build_markdown(
                &phase,
                &sorted_weeks,
                &entries_by_week,
                &goals,
                &key_results,
                &kr_map,
                &goal_map,
                &week_map,
                include_goals,
            );
            let filename = format!("brag-frog-{}.md", safe_name);
            Ok((
                StatusCode::OK,
                [
                    (
                        header::CONTENT_TYPE,
                        "text/markdown; charset=utf-8".to_string(),
                    ),
                    (
                        header::CONTENT_DISPOSITION,
                        format!("attachment; filename=\"{}\"", filename),
                    ),
                ],
                md,
            )
                .into_response())
        }
    }
}

// Builds a Markdown brag document with optional goals/KRs section and weekly entry groups.
#[allow(clippy::too_many_arguments)]
fn build_markdown(
    phase: &BragPhase,
    sorted_weeks: &[Week],
    entries_by_week: &HashMap<i64, Vec<&BragEntry>>,
    goals: &[Goal],
    key_results: &[KeyResult],
    kr_map: &HashMap<i64, &KeyResult>,
    goal_map: &HashMap<i64, &Goal>,
    _week_map: &HashMap<i64, &Week>,
    include_goals: bool,
) -> String {
    let mut out = String::new();

    out.push_str(&format!("# Brag Document — {}\n\n", phase.name));
    out.push_str(&format!(
        "**Period:** {} to {}\n",
        phase.start_date, phase.end_date
    ));

    if include_goals && !goals.is_empty() {
        out.push_str("\n---\n\n## Goals & Key Results\n");

        for goal in goals {
            out.push_str(&format!("\n### Goal: {}\n", goal.title));

            let mut meta = Vec::new();
            if let Some(ref cat) = goal.category
                && !cat.is_empty()
            {
                meta.push(format!("Category: {}", cat));
            }
            meta.push(format!("Status: {}", format_status(&goal.status)));
            out.push_str(&format!("*{}*\n", meta.join(" | ")));

            if let Some(ref desc) = goal.description
                && !desc.is_empty()
            {
                out.push_str(&format!("\n{}\n", desc));
            }

            // Key results under this goal
            let goal_krs: Vec<&KeyResult> = key_results
                .iter()
                .filter(|kr| kr.goal_id == Some(goal.id))
                .collect();

            for kr in &goal_krs {
                out.push_str(&format!(
                    "\n- **Key Result:** {} — {}% complete\n",
                    kr.name, kr.progress
                ));
            }
        }

        // Unassigned key results
        let unassigned: Vec<&KeyResult> = key_results
            .iter()
            .filter(|kr| kr.goal_id.is_none() && !kr.is_archived)
            .collect();

        if !unassigned.is_empty() {
            out.push_str("\n### Unassigned Key Results\n");
            for kr in &unassigned {
                out.push_str(&format!(
                    "\n- **Key Result:** {} — {}% complete\n",
                    kr.name, kr.progress
                ));
            }
        }
    }

    out.push_str("\n---\n");

    for week in sorted_weeks {
        let week_entries = entries_by_week.get(&week.id);
        let entries = match week_entries {
            Some(e) if !e.is_empty() => e,
            _ => continue,
        };

        out.push_str(&format!(
            "\n## Week {} · {} to {}\n",
            week.iso_week, week.start_date, week.end_date
        ));

        for entry in entries {
            let type_name = EntryType::display_name(&entry.entry_type);
            out.push_str(&format!("\n### {}: {}\n", type_name, entry.title));
            out.push_str(&format!("- **Date:** {}\n", entry.occurred_at));

            if let Some(kr_id) = entry.key_result_id
                && let Some(kr) = kr_map.get(&kr_id)
            {
                out.push_str(&format!("- **Key Result:** {}\n", kr.name));
                if let Some(goal_id) = kr.goal_id
                    && let Some(goal) = goal_map.get(&goal_id)
                {
                    out.push_str(&format!("- **Goal:** {}\n", goal.title));
                }
            }

            if let Some(ref teams) = entry.teams
                && !teams.is_empty()
            {
                out.push_str(&format!("- **Teams:** {}\n", teams));
            }

            if let Some(ref collaborators) = entry.collaborators
                && !collaborators.is_empty()
            {
                out.push_str(&format!("- **People:** {}\n", collaborators));
            }

            if let Some(ref url) = entry.source_url
                && !url.is_empty()
            {
                out.push_str(&format!("- **Link:** {}\n", url));
            }

            if let Some(ref desc) = entry.description
                && !desc.is_empty()
            {
                out.push_str(&format!("\n> {}\n", desc.replace('\n', "\n> ")));
            }
        }
    }

    out
}

// Builds a JSON brag document with phase metadata, optional goals/KRs, and weekly entries.
#[allow(clippy::too_many_arguments)]
fn build_json(
    phase: &BragPhase,
    sorted_weeks: &[Week],
    entries_by_week: &HashMap<i64, Vec<&BragEntry>>,
    goals: &[Goal],
    key_results: &[KeyResult],
    kr_map: &HashMap<i64, &KeyResult>,
    _goal_map: &HashMap<i64, &Goal>,
    include_goals: bool,
) -> String {
    let mut root = serde_json::json!({
        "phase": {
            "name": phase.name,
            "start_date": phase.start_date,
            "end_date": phase.end_date,
        }
    });

    if include_goals {
        let goals_json: Vec<serde_json::Value> = goals
            .iter()
            .map(|g| {
                serde_json::json!({
                    "id": g.id,
                    "title": g.title,
                    "description": g.description,
                    "category": g.category,
                    "status": g.status,
                })
            })
            .collect();
        root["goals"] = serde_json::json!(goals_json);

        let krs_json: Vec<serde_json::Value> = key_results
            .iter()
            .filter(|kr| !kr.is_archived)
            .map(|kr| {
                serde_json::json!({
                    "id": kr.id,
                    "name": kr.name,
                    "goal_id": kr.goal_id,
                    "status": kr.status,
                    "progress": kr.progress,
                })
            })
            .collect();
        root["key_results"] = serde_json::json!(krs_json);
    }

    let weeks_json: Vec<serde_json::Value> = sorted_weeks
        .iter()
        .filter_map(|week| {
            let week_entries = entries_by_week.get(&week.id)?;
            if week_entries.is_empty() {
                return None;
            }

            let entries_json: Vec<serde_json::Value> = week_entries
                .iter()
                .map(|e| {
                    let mut ej = serde_json::json!({
                        "title": e.title,
                        "entry_type": e.entry_type,
                        "entry_type_label": EntryType::display_name(&e.entry_type),
                        "occurred_at": e.occurred_at,
                        "source": e.source,
                    });
                    if let Some(ref desc) = e.description {
                        ej["description"] = serde_json::json!(desc);
                    }
                    if let Some(ref url) = e.source_url {
                        ej["source_url"] = serde_json::json!(url);
                    }
                    if let Some(ref teams) = e.teams {
                        ej["teams"] = serde_json::json!(teams);
                    }
                    if let Some(ref collaborators) = e.collaborators {
                        ej["collaborators"] = serde_json::json!(collaborators);
                    }
                    if let Some(kr_id) = e.key_result_id {
                        ej["key_result_id"] = serde_json::json!(kr_id);
                        if let Some(kr) = kr_map.get(&kr_id) {
                            ej["key_result_name"] = serde_json::json!(kr.name);
                        }
                    }
                    ej
                })
                .collect();

            Some(serde_json::json!({
                "week_number": week.week_number,
                "iso_week": week.iso_week,
                "year": week.year,
                "start_date": week.start_date,
                "end_date": week.end_date,
                "entries": entries_json,
            }))
        })
        .collect();

    root["weeks"] = serde_json::json!(weeks_json);

    serde_json::to_string_pretty(&root).unwrap_or_default()
}

// Converts a status slug ("in_progress") to its display label ("In Progress").
fn format_status(status: &str) -> &str {
    match status {
        "in_progress" => "In Progress",
        "completed" => "Completed",
        "not_started" => "Not Started",
        "on_hold" => "On Hold",
        _ => status,
    }
}
