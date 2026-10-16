use axum::extract::State;
use axum::response::Html;
use std::collections::HashMap;

use crate::AppState;
use crate::cycle::model::BragPhase;
use crate::cycle::service::trends::{self, EntryFilter, NameCount, PriorityShare};
use crate::identity::auth::middleware::AuthUser;
use crate::identity::model::User;
use crate::kernel::error::AppError;

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

/// Horizontal bar data from ranked counts, normalised to the largest value.
fn build_bars(items: &[NameCount]) -> Vec<serde_json::Value> {
    let max = items.first().map(|i| i.count).unwrap_or(1).max(1) as f64;
    items
        .iter()
        .map(|i| {
            let pct = (i.count as f64 / max * 100.0).round() as i64;
            serde_json::json!({"name": humanize_label(&i.name), "count": i.count, "pct": pct})
        })
        .collect()
}

struct Group<'a> {
    name: String,
    status: Option<String>,
    color: &'static str,
    count: u32,
    items: Vec<&'a PriorityShare>,
}

/// Groups per-priority activity under department goals for the legacy template.
fn priority_activity_groups(shares: &[PriorityShare]) -> Vec<serde_json::Value> {
    // Entries without a priority get their own group, separate from priorities without a goal.
    let mut grouped: HashMap<(Option<i64>, bool), Vec<&PriorityShare>> = HashMap::new();
    for share in shares {
        grouped
            .entry((share.department_goal_id, share.priority_id.is_none()))
            .or_default()
            .push(share);
    }

    let mut groups: Vec<Group> = grouped
        .into_iter()
        .map(|((goal_id, unlinked), items)| {
            let count = items.iter().map(|i| i.count).sum();
            let (name, status, color) = match (goal_id, items.first()) {
                (Some(_), Some(first)) => (
                    first.department_goal_title.clone().unwrap_or_default(),
                    first.department_goal_status.clone(),
                    "#FF453F",
                ),
                _ if unlinked => ("Unlinked Entries".to_string(), None, "#95A5A6"),
                _ => ("No Department Goal".to_string(), None, "#FF453F"),
            };
            Group { name, status, color, count, items }
        })
        .collect();
    groups.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));

    let max_group = groups.first().map(|g| g.count).unwrap_or(1).max(1) as f64;
    groups
        .into_iter()
        .map(|Group { name, status, color, count, items }| {
            // `items` arrive sorted by count desc from the service.
            let child_max = items.first().map(|i| i.count).unwrap_or(1).max(1) as f64;
            let priorities: Vec<serde_json::Value> = items
                .iter()
                .map(|i| {
                    let (title, color) = if i.priority_id.is_none() {
                        (i.title.as_str(), "#95A5A6")
                    } else {
                        (i.title.as_str(), i.color.as_deref().unwrap_or("#888"))
                    };
                    serde_json::json!({
                        "name": title,
                        "color": color,
                        "count": i.count,
                        "pct": (i.count as f64 / child_max * 100.0).round() as i64,
                    })
                })
                .collect();
            serde_json::json!({
                "name": name,
                "status": status,
                "color": color,
                "count": count,
                "pct": (count as f64 / max_group * 100.0).round() as i64,
                "priorities": priorities,
            })
        })
        .collect()
}

/// Trends page: cross-phase analytics with CSS-only visualizations.
pub async fn trends_page(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let phases = BragPhase::list_for_user(&state.db, auth.user_id).await?;
    let Some(phase) = BragPhase::get_active(&state.db, auth.user_id).await? else {
        let mut ctx = tera::Context::new();
        ctx.insert("user", &user);
        ctx.insert("current_page", "trends");
        let html = state.templates.render("pages/no_phase.html", &ctx)?;
        return Ok(Html(html));
    };

    let data = trends::compute(
        &state.db,
        &auth.crypto,
        auth.user_id,
        &phase,
        &EntryFilter::default(),
        chrono::Local::now().date_naive(),
    )
    .await?;

    let category_bars: Vec<serde_json::Value> = {
        let max = data.categories.first().map(|c| c.count).unwrap_or(1).max(1) as f64;
        data.categories
            .iter()
            .map(|c| {
                let tooltip = c
                    .types
                    .iter()
                    .map(|t| format!("{}: {}", t.name, t.count))
                    .collect::<Vec<_>>()
                    .join("\n");
                serde_json::json!({
                    "name": c.name,
                    "count": c.count,
                    "pct": (c.count as f64 / max * 100.0).round() as i64,
                    "color": category_color(&c.name),
                    "tooltip": tooltip,
                })
            })
            .collect()
    };

    // The template only showed weekdays that have activity, busiest first.
    let mut weekday_ranked: Vec<NameCount> = data
        .weekday
        .iter()
        .filter(|d| d.count > 0)
        .cloned()
        .collect();
    weekday_ranked.sort_by_key(|d| std::cmp::Reverse(d.count));

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("phases", &phases);
    ctx.insert("total_entries", &data.kpis.total);
    ctx.insert(
        "priority_activity_groups",
        &priority_activity_groups(&data.priority_share),
    );
    ctx.insert("category_bars", &category_bars);
    ctx.insert("reach_bars", &build_bars(&data.impact.reach));
    ctx.insert("complexity_bars", &build_bars(&data.impact.complexity));
    ctx.insert("role_bars", &build_bars(&data.impact.role));
    ctx.insert("collaborator_bars", &build_bars(&data.people));
    ctx.insert("repo_bars", &build_bars(&data.repos));
    ctx.insert("team_bars", &build_bars(&data.teams));
    ctx.insert("weekday_bars", &build_bars(&weekday_ranked));
    ctx.insert("overview_total", &data.kpis.total);
    ctx.insert("overview_prs_merged", &data.kpis.prs_merged);
    ctx.insert("overview_prs_reviewed", &data.kpis.prs_reviewed);
    ctx.insert("overview_jira_tickets_closed", &data.kpis.jira_closed);
    ctx.insert("overview_docs_worked_on", &data.kpis.docs_worked_on);
    ctx.insert("overview_meetings", &data.kpis.meetings);
    ctx.insert("current_page", "trends");

    let html = state.templates.render("pages/trends.html", &ctx)?;
    Ok(Html(html))
}
