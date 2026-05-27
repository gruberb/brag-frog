use std::collections::{HashMap, HashSet};

use axum::{
    Form, Json,
    extract::{Path, Query, State},
    response::Html,
};

use crate::AppState;
use crate::ai::prompts::build_self_reflection_prompt;
use crate::ai::{get_ai_client, has_ai_for_user};
use crate::cycle::model::BragPhase;
use crate::identity::auth::middleware::AuthUser;
use crate::identity::clg::ClgLevel;
use crate::identity::model::User;
use crate::kernel::error::AppError;
use crate::kernel::render::render_markdown;
use crate::objectives::model::{DepartmentGoal, Priority};
use crate::review::model::{
    ContributionExample, Summary, assessment_config, get_section, rating_scale_config,
    section_question, section_title,
};
use crate::worklog::model::BragEntry;

// Aggregated data needed to build AI prompts for summary generation.
struct SummaryData {
    entries: Vec<BragEntry>,
    dept_goals: Vec<DepartmentGoal>,
    priorities: Vec<Priority>,
    contribution_examples: Vec<ContributionExample>,
    example_entry_ids: HashMap<i64, Vec<i64>>,
    clg_level: Option<&'static ClgLevel>,
    wants_promotion: bool,
}

#[derive(serde::Deserialize)]
pub struct AiDraftQuery {
    pub dept_goal_ids: Option<String>,
}

#[derive(serde::Serialize)]
pub struct DraftResponse {
    pub content: String,
    pub rendered_html: String,
}

// Loads entries, department goals, priorities, and CLG level for building AI prompts.
async fn load_summary_data(
    state: &AppState,
    phase_id: i64,
    user_id: i64,
) -> Result<SummaryData, AppError> {
    let user_crypto = state.crypto.for_user(user_id)?;
    let entries = BragEntry::list_for_phase(&state.db, phase_id, &user_crypto).await?;
    let dept_goals = DepartmentGoal::list_for_phase(&state.db, phase_id, &user_crypto).await?;
    let priorities = Priority::list_for_phase(&state.db, phase_id, &user_crypto).await?;
    let contribution_examples =
        ContributionExample::list_for_phase(&state.db, phase_id, &user_crypto).await?;
    let mut example_entry_ids = HashMap::new();
    for example in &contribution_examples {
        let linked_ids = ContributionExample::linked_entry_ids(&state.db, example.id).await?;
        example_entry_ids.insert(example.id, linked_ids);
    }

    let user = User::find_by_id(&state.db, user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;
    let clg_level = user
        .role
        .as_deref()
        .and_then(|r| crate::identity::clg::get_level(r));

    Ok(SummaryData {
        entries,
        dept_goals,
        priorities,
        contribution_examples,
        example_entry_ids,
        clg_level,
        wants_promotion: user.wants_promotion,
    })
}

fn review_marker_for_phase(phase: &BragPhase) -> (String, String) {
    let year = phase.end_date.get(0..4).unwrap_or("Review").to_string();
    let end_month = phase
        .end_date
        .get(5..7)
        .and_then(|m| m.parse::<u32>().ok())
        .unwrap_or(12);

    if end_month <= 6 {
        (format!("Q2 {}", year), "Mid-year Review".to_string())
    } else {
        (format!("Q4 {}", year), "Year-end Review".to_string())
    }
}

/// Renders the self-review page as a single Lattice-style answer surface.
pub async fn summary_page(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(phase_id): Path<i64>,
) -> Result<Html<String>, AppError> {
    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let phase = BragPhase::find_by_id(&state.db, phase_id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Phase not found".to_string()))?;

    let summaries = Summary::list_for_phase(&state.db, phase_id, &auth.crypto).await?;
    let has_ai = has_ai_for_user(&state, auth.user_id).await;
    let examples = ContributionExample::list_for_phase(&state.db, phase_id, &auth.crypto).await?;
    let entries = BragEntry::list_for_phase(&state.db, phase_id, &auth.crypto).await?;
    let dept_goals = DepartmentGoal::list_for_phase(&state.db, phase_id, &auth.crypto).await?;
    let priorities = Priority::list_for_phase(&state.db, phase_id, &auth.crypto).await?;
    let jira_links = jira_issue_links(&entries);
    let sections = build_primary_section_json(&summaries, &jira_links);
    let primary_section_key = primary_section_slug()
        .unwrap_or("impact_examples")
        .to_string();
    let (review_quarter, review_label) = review_marker_for_phase(&phase);

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("review_quarter", &review_quarter);
    ctx.insert("review_label", &review_label);
    ctx.insert("sections", &sections);
    ctx.insert("primary_section_key", &primary_section_key);
    ctx.insert("has_ai", &has_ai);
    ctx.insert("current_page", "summary");
    ctx.insert("examples", &examples);
    ctx.insert("entries", &entries);
    ctx.insert("dept_goals", &dept_goals);
    ctx.insert("priorities", &priorities);

    let html = state.templates.render("pages/summary.html", &ctx)?;
    Ok(Html(html))
}

/// Renders the review guide page with assessment guidelines and rating scale.
pub async fn review_guide_page(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let phase = BragPhase::get_active(&state.db, auth.user_id).await?;
    let assessment = assessment_config();
    let rating_scale = rating_scale_config();

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("current_page", "review");
    if let Some(ref p) = phase {
        ctx.insert("phase", p);
    }
    ctx.insert("assessment_mid_year", &assessment.mid_year);
    ctx.insert("assessment_year_end", &assessment.year_end);
    ctx.insert("rating_scale", &rating_scale.ratings);

    let html = state.templates.render("pages/review_guide.html", &ctx)?;
    Ok(Html(html))
}

/// HTMX handler: AI-generates all configured summary sections and re-renders the full page.
pub async fn generate_all(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(phase_id): Path<i64>,
) -> Result<Html<String>, AppError> {
    let ai_client = get_ai_client(&state, auth.user_id).await?;

    let phase = BragPhase::find_by_id(&state.db, phase_id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Phase not found".to_string()))?;

    let data = load_summary_data(&state, phase_id, auth.user_id).await?;

    if let Some(section) = primary_section_slug() {
        let prompt = build_self_reflection_prompt(
            section,
            &data.dept_goals,
            &data.entries,
            &data.priorities,
            &data.contribution_examples,
            &data.example_entry_ids,
            &[],
            &phase.name,
            data.clg_level,
            data.wants_promotion,
        );

        let content = ai_client.generate(&prompt).await?;

        Summary::upsert(
            &state.db,
            phase_id,
            section,
            &content,
            Some(&prompt),
            Some(&state.config.ai_model),
            &auth.crypto,
        )
        .await?;
    }

    // Re-render only the summary sections partial (target is #summary-sections innerHTML)
    let summaries = Summary::list_for_phase(&state.db, phase_id, &auth.crypto).await?;

    let mut ctx = tera::Context::new();
    let jira_links = jira_issue_links(&data.entries);
    ctx.insert(
        "sections",
        &build_primary_section_json(&summaries, &jira_links),
    );
    ctx.insert("phase", &phase);
    ctx.insert("has_ai", &true);
    ctx.insert("dept_goals", &data.dept_goals);
    ctx.insert("priorities", &data.priorities);

    let html = state
        .templates
        .render("components/summary_sections.html", &ctx)?;
    Ok(Html(html))
}

/// HTMX handler: generates an AI draft for a section and returns plain text (does NOT persist).
pub async fn ai_draft_section(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((phase_id, section)): Path<(i64, String)>,
    Query(query): Query<AiDraftQuery>,
) -> Result<Json<DraftResponse>, AppError> {
    let ai_client = get_ai_client(&state, auth.user_id).await?;

    let phase = BragPhase::find_by_id(&state.db, phase_id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Phase not found".to_string()))?;

    let data = load_summary_data(&state, phase_id, auth.user_id).await?;
    let focused_dept_goal_ids =
        parse_department_goal_ids(query.dept_goal_ids.as_deref(), &data.dept_goals);

    let prompt = build_self_reflection_prompt(
        &section,
        &data.dept_goals,
        &data.entries,
        &data.priorities,
        &data.contribution_examples,
        &data.example_entry_ids,
        &focused_dept_goal_ids,
        &phase.name,
        data.clg_level,
        data.wants_promotion,
    );

    let content = ai_client.generate(&prompt).await?;
    let jira_links = jira_issue_links(&data.entries);
    let rendered_html = render_summary_content(&content, &jira_links);

    // Return the draft and rendered preview — do NOT save to database.
    Ok(Json(DraftResponse {
        content,
        rendered_html,
    }))
}

/// Form payload for saving or updating a summary section's content.
#[derive(serde::Deserialize)]
pub struct UpdateSummaryForm {
    pub content: String,
}

/// HTMX handler: upserts a summary section's content (creates if new) and returns the section fragment.
pub async fn save_section(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((phase_id, section)): Path<(i64, String)>,
    Form(input): Form<UpdateSummaryForm>,
) -> Result<Html<String>, AppError> {
    let phase = BragPhase::find_by_id(&state.db, phase_id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Phase not found".to_string()))?;

    // Upsert: creates if not exists, updates if exists
    let summary = Summary::upsert(
        &state.db,
        phase_id,
        &section,
        &input.content,
        None,
        None,
        &auth.crypto,
    )
    .await?;

    let has_ai = has_ai_for_user(&state, auth.user_id).await;
    let dept_goals = DepartmentGoal::list_for_phase(&state.db, phase_id, &auth.crypto).await?;
    let priorities = Priority::list_for_phase(&state.db, phase_id, &auth.crypto).await?;
    let entries = BragEntry::list_for_phase(&state.db, phase_id, &auth.crypto).await?;
    let jira_links = jira_issue_links(&entries);

    let mut ctx = tera::Context::new();
    ctx.insert(
        "section",
        &build_section_json(&section, Some(&summary), &jira_links),
    );
    ctx.insert("phase", &phase);
    ctx.insert("has_ai", &has_ai);
    ctx.insert("dept_goals", &dept_goals);
    ctx.insert("priorities", &priorities);

    let html = state
        .templates
        .render("components/summary_section.html", &ctx)?;
    Ok(Html(html))
}

/// HTMX helper: renders unsaved review Markdown so generated or edited drafts can
/// be previewed without creating another review section.
pub async fn preview_section(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((phase_id, _section)): Path<(i64, String)>,
    Form(input): Form<UpdateSummaryForm>,
) -> Result<Json<DraftResponse>, AppError> {
    let _phase = BragPhase::find_by_id(&state.db, phase_id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Phase not found".to_string()))?;

    let entries = BragEntry::list_for_phase(&state.db, phase_id, &auth.crypto).await?;
    let jira_links = jira_issue_links(&entries);
    let rendered_html = render_summary_content(&input.content, &jira_links);

    Ok(Json(DraftResponse {
        content: input.content,
        rendered_html,
    }))
}

// The Review page intentionally mirrors Lattice's single-answer self-review
// flow. Custom configs may still define historical sections, but they should
// not fan out into multiple textboxes on this page.
fn primary_section_slug() -> Option<&'static str> {
    crate::review::model::section_slugs().first().copied()
}

fn build_primary_section_json(
    summaries: &[Summary],
    jira_links: &HashMap<String, String>,
) -> Vec<serde_json::Value> {
    let Some(section) = primary_section_slug() else {
        return Vec::new();
    };
    let summary = summaries.iter().find(|s| s.section == section);
    vec![build_section_json(section, summary, jira_links)]
}

fn build_section_json(
    section: &str,
    summary: Option<&Summary>,
    jira_links: &HashMap<String, String>,
) -> serde_json::Value {
    let config = get_section(section);
    let content = summary.map(|s| s.content.clone());
    let rendered_content = content
        .as_deref()
        .map(|c| render_summary_content(c, jira_links))
        .unwrap_or_default();

    serde_json::json!({
        "key": section,
        "title": section_title(section),
        "question": section_question(section),
        "form_question_number": config.and_then(|s| s.form_question_number.clone()),
        "form_required": config.is_some_and(|s| s.form_required),
        "form_question": config.and_then(|s| s.form_question.clone()),
        "form_guidance": config.and_then(|s| s.form_guidance.clone()),
        "form_bullets": config.map_or_else(Vec::new, |s| s.form_bullets.clone()),
        "form_tip": config.and_then(|s| s.form_tip.clone()),
        "form_placeholder": config.and_then(|s| s.form_placeholder.clone()),
        "focus_priorities": config.is_some_and(|s| s.focus_priorities),
        "focus_priority_help": config.and_then(|s| s.focus_priority_help.clone()),
        "content": content,
        "rendered_content": rendered_content,
        "generated_at": summary.map(|s| s.generated_at.clone()),
        "id": summary.map(|s| s.id),
    })
}

fn render_summary_content(content: &str, jira_links: &HashMap<String, String>) -> String {
    let linked = linkify_jira_issue_keys(content, jira_links);
    render_markdown(&linked)
}

fn jira_issue_links(entries: &[BragEntry]) -> HashMap<String, String> {
    let mut links = HashMap::new();

    for entry in entries {
        let Some(url) = entry.source_url.as_deref().filter(|s| !s.trim().is_empty()) else {
            continue;
        };
        if entry.source != "jira" && !url.contains("/browse/") {
            continue;
        }

        let key = find_jira_issue_keys(url)
            .into_iter()
            .next()
            .or_else(|| {
                entry
                    .source_id
                    .as_deref()
                    .and_then(|source_id| find_jira_issue_keys(source_id).into_iter().next())
            })
            .or_else(|| find_jira_issue_keys(&entry.title).into_iter().next());

        if let Some(key) = key {
            links.entry(key).or_insert_with(|| url.to_string());
        }
    }

    links
}

fn linkify_jira_issue_keys(content: &str, links: &HashMap<String, String>) -> String {
    if content.is_empty() || links.is_empty() {
        return content.to_string();
    }

    let bytes = content.as_bytes();
    let mut out = String::with_capacity(content.len());
    let mut cursor = 0;
    let mut i = 0;

    while i < bytes.len() {
        if !bytes[i].is_ascii_uppercase() {
            i += 1;
            continue;
        }

        let start = i;
        i += 1;
        while i < bytes.len() && (bytes[i].is_ascii_uppercase() || bytes[i].is_ascii_digit()) {
            i += 1;
        }

        if i >= bytes.len() || bytes[i] != b'-' || i - start < 2 {
            continue;
        }

        let number_start = i + 1;
        let mut end = number_start;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if end == number_start {
            continue;
        }

        let key = &content[start..end];
        if let Some(url) = links.get(key)
            && can_link_issue_key(content, start, end)
        {
            out.push_str(&content[cursor..start]);
            out.push('[');
            out.push_str(key);
            out.push_str("](");
            out.push_str(&escape_markdown_url(url));
            out.push(')');
            cursor = end;
        }

        i = end;
    }

    if cursor == 0 {
        content.to_string()
    } else {
        out.push_str(&content[cursor..]);
        out
    }
}

fn find_jira_issue_keys(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut keys = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        if !bytes[i].is_ascii_uppercase() {
            i += 1;
            continue;
        }

        let start = i;
        i += 1;
        while i < bytes.len() && (bytes[i].is_ascii_uppercase() || bytes[i].is_ascii_digit()) {
            i += 1;
        }

        if i >= bytes.len() || bytes[i] != b'-' || i - start < 2 {
            continue;
        }

        let number_start = i + 1;
        let mut end = number_start;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }

        if end > number_start {
            keys.push(text[start..end].to_string());
        }
        i = end;
    }

    keys
}

fn can_link_issue_key(content: &str, start: usize, end: usize) -> bool {
    let previous = content[..start].chars().next_back();
    let next = content[end..].chars().next();

    if previous == Some('[') || next == Some(']') {
        return false;
    }

    let is_embedded = |ch: char| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '_' | '-' | '.');
    !previous.is_some_and(is_embedded) && !next.is_some_and(is_embedded)
}

fn escape_markdown_url(url: &str) -> String {
    url.replace(')', "%29").replace(' ', "%20")
}

fn parse_department_goal_ids(raw: Option<&str>, dept_goals: &[DepartmentGoal]) -> Vec<i64> {
    let allowed: HashSet<i64> = dept_goals.iter().map(|g| g.id).collect();

    raw.unwrap_or("")
        .split(',')
        .filter_map(|value| value.trim().parse::<i64>().ok())
        .filter(|id| allowed.contains(id))
        .collect()
}
