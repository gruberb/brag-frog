use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Redirect, Response},
};
use std::collections::HashMap;
use tower_sessions::Session;

use crate::AppState;
use crate::analytics::insights::{
    AnalyzeFilterQuery, AnalyzePageQuery, apply_in_memory_filters, build_date_groups,
    category_to_entry_types, collect_unique_values, compute_insights,
};
use crate::entries::model::BragEntry;
use crate::identity::auth::middleware::AuthUser;
use crate::identity::model::User;
use crate::okr::model::{Goal, Initiative, KeyResult};
use crate::review::model::BragPhase;
use crate::shared::error::AppError;

/// Landing page: redirects authenticated users to `/dashboard`, shows login page otherwise.
pub async fn landing_page(
    State(state): State<AppState>,
    session: Session,
) -> Result<Response, AppError> {
    // Check if user is already logged in (non-error pattern)
    let user_id: Option<i64> = session.get("user_id").await.unwrap_or(None);

    if user_id.is_some() {
        return Ok(Redirect::to("/dashboard").into_response());
    }

    let mut ctx = tera::Context::new();

    let state_token = format!("login:{}", uuid::Uuid::new_v4());
    session
        .insert("oauth_state", &state_token)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to set OAuth state: {}", e)))?;

    let google_auth_url = crate::identity::auth::google_auth_url(&state.config, &state_token);
    ctx.insert("google_auth_url", &google_auth_url);
    ctx.insert("instance_name", &state.config.instance_name);

    let html = state.templates.render("pages/landing.html", &ctx)?;
    Ok(Html(html).into_response())
}

/// Renders the logbook page with filter toolbar, date-grouped entries, insights, and report sidebar.
pub async fn logbook(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(page_query): Query<AnalyzePageQuery>,
) -> Result<Html<String>, AppError> {
    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let phase = match BragPhase::get_active(&state.db, auth.user_id).await? {
        Some(p) => p,
        None => {
            let mut ctx = tera::Context::new();
            ctx.insert("user", &user);
            ctx.insert("current_page", "logbook");
            let html = state.templates.render("pages/no_phase.html", &ctx)?;
            return Ok(Html(html));
        }
    };

    let goals = Goal::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let key_results = KeyResult::list_active_for_user(&state.db, auth.user_id).await?;
    let initiatives = Initiative::list_for_phase(&state.db, phase.id, &auth.crypto).await?;

    let today_str = chrono::Local::now().format("%Y-%m-%d").to_string();

    // Build goal → key result IDs map (for JS cascading)
    let mut goal_key_results_map: HashMap<i64, Vec<i64>> = HashMap::new();
    for kr in &key_results {
        if let Some(gid) = kr.goal_id {
            goal_key_results_map.entry(gid).or_default().push(kr.id);
        }
    }
    let goal_key_results_json =
        serde_json::to_string(&goal_key_results_map).unwrap_or_else(|_| "{}".to_string());

    // Fetch entries WITHOUT category filter (so insights reflect the full base set)
    // Use full phase end date so manual entries with future dates are included.
    let sources: Vec<String> = page_query
        .source
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let goal_id: Option<i64> = page_query.goal_id.as_deref().and_then(|s| s.parse().ok());
    let key_result_id: Option<i64> = page_query
        .key_result_id
        .as_deref()
        .and_then(|s| s.parse().ok());

    let mut entries = BragEntry::list_for_phase_filtered(
        &state.db,
        phase.id,
        key_result_id,
        goal_id,
        &[],
        Some(phase.start_date.as_str()),
        Some(&phase.end_date),
        &sources,
        &auth.crypto,
    )
    .await?;

    // Hide future synced entries (e.g. upcoming calendar meetings) but keep manual entries
    entries.retain(|e| e.source == "manual" || e.occurred_at.as_str() <= today_str.as_str());

    // Post-fetch in-memory filters (search, team, collaborator, special flags)
    apply_in_memory_filters(&mut entries, &page_query);

    // Compute insights from the base set (before category filter)
    let insights = compute_insights(&entries, &key_results);

    // Now apply category filter in-memory
    if let Some(ref cat) = page_query.category
        && !cat.is_empty()
    {
        let allowed = category_to_entry_types(cat);
        entries.retain(|e| allowed.contains(&e.entry_type.as_str()));
    }

    // Collect unique teams and collaborators from ALL phase entries for dropdown options
    let all_entries = BragEntry::list_for_phase_in_range(
        &state.db,
        phase.id,
        &phase.start_date,
        &phase.end_date,
        &auth.crypto,
    )
    .await?;

    let mut all_teams: Vec<String> = collect_unique_values(&all_entries, |e| e.teams.as_deref());
    all_teams.sort();
    let mut all_collaborators: Vec<String> =
        collect_unique_values(&all_entries, |e| e.collaborators.as_deref());
    all_collaborators.sort();

    let date_groups = build_date_groups(&entries);

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("goals", &goals);
    ctx.insert("key_results", &key_results);
    ctx.insert("initiatives", &initiatives);
    ctx.insert("total_entries", &all_entries.len());
    ctx.insert("filtered_count", &entries.len());
    ctx.insert("current_page", "logbook");
    ctx.insert("goal_key_results_json", &goal_key_results_json);
    ctx.insert("all_teams", &all_teams);
    ctx.insert("all_collaborators", &all_collaborators);
    ctx.insert("insights", &insights);
    ctx.insert("date_groups", &date_groups);
    ctx.insert(
        "entry_types",
        &crate::entries::model::EntryType::as_json_options(),
    );
    ctx.insert(
        "manual_entry_types",
        &crate::entries::model::EntryType::as_manual_json_options(),
    );
    ctx.insert(
        "grouped_entry_types",
        &crate::entries::model::EntryType::as_grouped_json_options(),
    );
    ctx.insert(
        "manual_grouped_entry_types",
        &crate::entries::model::EntryType::as_manual_grouped_json_options(),
    );

    // Pass initial filter values for URL state restoration
    ctx.insert("init_goal_id", &page_query.goal_id.as_deref().unwrap_or(""));
    ctx.insert(
        "init_key_result_id",
        &page_query.key_result_id.as_deref().unwrap_or(""),
    );
    ctx.insert(
        "init_initiative_id",
        &page_query.initiative_id.as_deref().unwrap_or(""),
    );
    ctx.insert(
        "init_category",
        &page_query.category.as_deref().unwrap_or(""),
    );
    ctx.insert("init_source", &page_query.source.as_deref().unwrap_or(""));
    ctx.insert("init_team", &page_query.team.as_deref().unwrap_or(""));
    ctx.insert(
        "init_collaborator",
        &page_query.collaborator.as_deref().unwrap_or(""),
    );
    ctx.insert("init_search", &page_query.search.as_deref().unwrap_or(""));
    ctx.insert(
        "init_no_key_result",
        &page_query.no_key_result.as_deref().unwrap_or(""),
    );
    ctx.insert("init_no_team", &page_query.no_team.as_deref().unwrap_or(""));
    ctx.insert(
        "init_no_collaborator",
        &page_query.no_collaborator.as_deref().unwrap_or(""),
    );

    let html = state.templates.render("pages/analyze.html", &ctx)?;
    Ok(Html(html))
}

/// HTMX handler: applies filters and returns the entry list + report + count via OOB swaps.
pub async fn logbook_filtered_entries(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<AnalyzeFilterQuery>,
) -> Result<Html<String>, AppError> {
    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("No active phase".to_string()))?;

    let today_str = chrono::Local::now().format("%Y-%m-%d").to_string();

    let sources: Vec<String> = query
        .source
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Use full phase end date so manual entries with future dates are included.
    let mut entries = BragEntry::list_for_phase_filtered(
        &state.db,
        phase.id,
        query.key_result_id,
        query.goal_id,
        &[],
        Some(phase.start_date.as_str()),
        Some(&phase.end_date),
        &sources,
        &auth.crypto,
    )
    .await?;

    // Hide future synced entries but keep manual entries
    entries.retain(|e| e.source == "manual" || e.occurred_at.as_str() <= today_str.as_str());

    // Build a page_query-compatible struct for in-memory filters
    let page_query = AnalyzePageQuery {
        goal_id: query.goal_id.map(|id| id.to_string()),
        key_result_id: query.key_result_id.map(|id| id.to_string()),
        initiative_id: query.initiative_id.map(|id| id.to_string()),
        category: query.category.clone(),
        source: query.source,
        team: query.team,
        collaborator: query.collaborator,
        search: query.search,
        no_key_result: query.no_key_result,
        no_team: query.no_team,
        no_collaborator: query.no_collaborator,
    };
    apply_in_memory_filters(&mut entries, &page_query);

    let key_results = KeyResult::list_active_for_user(&state.db, auth.user_id).await?;
    let goals = Goal::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let initiatives = Initiative::list_for_phase(&state.db, phase.id, &auth.crypto).await?;

    // Compute insights from base set (before category filter)
    let insights = compute_insights(&entries, &key_results);

    // Now apply category filter in-memory
    if let Some(ref cat) = query.category
        && !cat.is_empty()
    {
        let allowed = category_to_entry_types(cat);
        entries.retain(|e| allowed.contains(&e.entry_type.as_str()));
    }

    let date_groups = build_date_groups(&entries);

    let mut ctx = tera::Context::new();
    ctx.insert("entries", &entries);
    ctx.insert("key_results", &key_results);
    ctx.insert("goals", &goals);
    ctx.insert("initiatives", &initiatives);
    ctx.insert("insights", &insights);
    ctx.insert("date_groups", &date_groups);
    ctx.insert("filtered_count", &entries.len());
    ctx.insert(
        "entry_types",
        &crate::entries::model::EntryType::as_json_options(),
    );
    ctx.insert(
        "manual_entry_types",
        &crate::entries::model::EntryType::as_manual_json_options(),
    );
    ctx.insert(
        "grouped_entry_types",
        &crate::entries::model::EntryType::as_grouped_json_options(),
    );
    ctx.insert(
        "manual_grouped_entry_types",
        &crate::entries::model::EntryType::as_manual_grouped_json_options(),
    );

    // Render report partial with OOB swap so sidebar + count update from one response
    let report_html = state
        .templates
        .render("components/analyze_report.html", &ctx)?;
    let entries_html = state
        .templates
        .render("components/analyze_results.html", &ctx)?;

    let html = format!(
        "{}<div id=\"report-content\" hx-swap-oob=\"innerHTML\">{}</div><span id=\"filtered-count\" hx-swap-oob=\"innerHTML\">{}</span>",
        entries_html,
        report_html,
        entries.len()
    );
    Ok(Html(html))
}
