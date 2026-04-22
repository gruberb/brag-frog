//! Reports page — AI-generated narrative views of recent work.
//!
//! Two tabs share a single visual shell so they feel identical:
//!
//! - **Last Week**: read-only rolling summary of logbook entries from last
//!   Monday through today, grouped by priority. Regenerated on demand, never
//!   persisted — the logbook is the source of truth.
//! - **Latest Updates**: stakeholder-facing status narrative. Editable and
//!   persisted per week (backed by the `status_updates` table and the
//!   `status_update::*` handlers). Rendered as markdown by default with an
//!   Edit toggle that reveals the underlying textarea.

use axum::extract::State;
use axum::response::Html;
use chrono::Local;

use crate::AppState;
use crate::ai::prompts::EntryGroup;
use crate::cycle::model::{BragPhase, StatusUpdate, Week};
use crate::identity::auth::middleware::AuthUser;
use crate::identity::model::User;
use crate::kernel::error::AppError;
use crate::objectives::model::{DepartmentGoal, Priority};
use crate::worklog::model::BragEntry;

/// GET /reports — render the two-tab reports page. Both tabs start in their
/// "empty" state; AI generation is triggered by the user via HTMX POSTs that
/// swap rendered sections into the tab panels.
pub async fn reports_page(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let phase = match BragPhase::get_active(&state.db, auth.user_id).await? {
        Some(p) => p,
        None => {
            let mut ctx = tera::Context::new();
            ctx.insert("user", &user);
            ctx.insert("current_page", "reports");
            let html = state.templates.render("pages/no_phase.html", &ctx)?;
            return Ok(Html(html));
        }
    };

    let now = Local::now().naive_local().date();
    let current_week = Week::find_or_create_for_date(&state.db, phase.id, now).await?;

    let has_ai = crate::ai::helpers::has_ai_for_user(&state, auth.user_id).await;

    // Prefill the Latest Updates tab with the saved draft for the current
    // week, if any. Users generally re-open this tab to tweak the last
    // version rather than start fresh.
    let status_update =
        StatusUpdate::find_for_week(&state.db, current_week.id, auth.user_id, &auth.crypto).await?;

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("current_week", &current_week);
    ctx.insert("week_id", &current_week.id);
    ctx.insert("has_ai", &has_ai);
    ctx.insert("status_update", &status_update);
    ctx.insert("current_page", "reports");

    let html = state.templates.render("pages/reports.html", &ctx)?;
    Ok(Html(html))
}

/// POST /reports/last-week/generate — AI-generate a grouped summary of work
/// since last Monday. Range is `[Monday of previous ISO week, today]`, so the
/// window is ~7–14 days depending on when the button is clicked. Entries are
/// grouped by their linked priority (and the priority's parent department
/// goal) so the model can narrate progress per objective.
pub async fn last_week_generate(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::BadRequest("No active phase".into()))?;

    let now = Local::now().naive_local().date();
    let current_week = Week::find_or_create_for_date(&state.db, phase.id, now).await?;

    // Window: Monday of the previous ISO week → today. Falls back to today's
    // `start_date` on parse error (impossible in practice — we just wrote it).
    let window_start = chrono::NaiveDate::parse_from_str(&current_week.start_date, "%Y-%m-%d")
        .map(|d| d - chrono::Duration::days(7))
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|_| current_week.start_date.clone());
    let window_end = now.format("%Y-%m-%d").to_string();

    let entries = BragEntry::list_for_phase_in_range(
        &state.db,
        phase.id,
        &window_start,
        &window_end,
        &auth.crypto,
    )
    .await?;

    let priorities = Priority::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let dept_goals = DepartmentGoal::list_for_phase(&state.db, phase.id, &auth.crypto).await?;

    // Bucket entries by priority_id. Priorities with no entries are dropped;
    // entries with no linked priority fall into the `None` bucket
    // ("Unassigned"). Preserves the DB-returned priority order.
    let mut groups: Vec<EntryGroup<'_>> = priorities
        .iter()
        .map(|p| {
            let dept_goal = p
                .department_goal_id
                .and_then(|id| dept_goals.iter().find(|dg| dg.id == id));
            EntryGroup {
                priority: Some(p),
                dept_goal,
                entries: entries
                    .iter()
                    .filter(|e| e.priority_id == Some(p.id))
                    .collect(),
            }
        })
        .filter(|g| !g.entries.is_empty())
        .collect();

    let unassigned: Vec<&BragEntry> = entries
        .iter()
        .filter(|e| e.priority_id.is_none())
        .collect();
    if !unassigned.is_empty() {
        groups.push(EntryGroup {
            priority: None,
            dept_goal: None,
            entries: unassigned,
        });
    }

    let prompt =
        crate::ai::prompts::build_last_week_summary_prompt(&groups, &window_start, &window_end);

    let ai = crate::ai::helpers::get_ai_client(&state, auth.user_id).await?;
    let generated = ai.generate(&prompt).await?;

    // `EntryGroup` holds borrowed references — serialise into a plain owned
    // struct before handing it to Tera for the evidence `<details>` block.
    #[derive(serde::Serialize)]
    struct RenderedGroup {
        heading: String,
        entries: Vec<RenderedEntry>,
    }
    #[derive(serde::Serialize)]
    struct RenderedEntry {
        id: i64,
        title: String,
        entry_type: String,
        occurred_at: String,
        status: Option<String>,
    }
    let rendered_groups: Vec<RenderedGroup> = groups
        .iter()
        .map(|g| RenderedGroup {
            heading: match (g.dept_goal, g.priority) {
                (Some(dg), Some(p)) => format!("{} — {}", dg.title, p.title),
                (None, Some(p)) => p.title.clone(),
                _ => "Unassigned".to_string(),
            },
            entries: g
                .entries
                .iter()
                .map(|e| RenderedEntry {
                    id: e.id,
                    title: e.title.clone(),
                    entry_type: e.entry_type.clone(),
                    occurred_at: e.occurred_at.clone(),
                    status: e.status.clone(),
                })
                .collect(),
        })
        .collect();

    let mut ctx = tera::Context::new();
    ctx.insert("summary", &generated);
    ctx.insert("window_start", &window_start);
    ctx.insert("window_end", &window_end);
    ctx.insert("groups", &rendered_groups);
    ctx.insert("has_ai", &true);

    let html = state
        .templates
        .render("components/last_week_section.html", &ctx)?;
    Ok(Html(html))
}
