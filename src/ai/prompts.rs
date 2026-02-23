use crate::entries::model::BragEntry;
use crate::identity::clg::ClgLevel;
use crate::okr::model::{Goal, KeyResult};
use crate::review::model::MeetingPrepNote;
use crate::review::model::get_section;

/// Assembles a complete prompt for one self-review section.
///
/// Combines phase context (stats, goals, entries) with a section-specific
/// instruction loaded from the review_sections config. Optionally embeds CLG
/// level expectations and promotion framing when the user is targeting the
/// next level.
#[allow(clippy::too_many_arguments)]
pub fn build_self_reflection_prompt(
    section: &str,
    goals: &[Goal],
    entries: &[BragEntry],
    key_results: &[KeyResult],
    phase_name: &str,
    clg_level: Option<&ClgLevel>,
    wants_promotion: bool,
) -> String {
    let stats = compute_stats(entries);

    let entries_by_goal = group_entries_by_goal(entries, goals, key_results);

    let clg_context = if let Some(level) = clg_level {
        let mut ctx = format!(
            r#"
## CLG Level: {} ({})
One-liner: {}

### Problems expectations:
- Task Size: {}
- Complexity: {}
- Risk Management: {}
- Domain Expertise: {}
- Strategy: {}

### People expectations:
- Influence: {}
- Responsibility: {}
- Communication: {}
- Change Management: {}
- Team Contributions: {}
- DEI: {}
"#,
            level.title,
            level.id,
            level.one_liner,
            level.problems.task_size,
            level.problems.complexity,
            level.problems.risk_management,
            level.problems.domain_expertise,
            level.problems.strategy,
            level.people.influence,
            level.people.responsibility,
            level.people.communication,
            level.people.change_management,
            level.people.team_contributions,
            level.people.dei,
        );

        if wants_promotion && let Some(next) = crate::identity::clg::get_next_level(&level.id) {
            ctx.push_str(&format!(
                    r#"
## PROMOTION TARGET: {} ({})
The engineer is targeting promotion to the next level. When writing, highlight examples that demonstrate readiness for:
One-liner: {}

### Next-level Problems expectations:
- Task Size: {}
- Complexity: {}
- Risk Management: {}
- Domain Expertise: {}
- Strategy: {}

### Next-level People expectations:
- Influence: {}
- Responsibility: {}
- Communication: {}
- Change Management: {}
- Team Contributions: {}
- DEI: {}
"#,
                    next.title, next.id,
                    next.one_liner,
                    next.problems.task_size,
                    next.problems.complexity,
                    next.problems.risk_management,
                    next.problems.domain_expertise,
                    next.problems.strategy,
                    next.people.influence,
                    next.people.responsibility,
                    next.people.communication,
                    next.people.change_management,
                    next.people.team_contributions,
                    next.people.dei,
                ));
        }
        ctx
    } else {
        String::new()
    };

    let review_platform = &crate::review::model::review_config().review_platform;

    let context = format!(
        r#"You are helping a software engineer write their half-year self-review for a {review_platform} performance review cycle.

Phase: {phase_name}

## Statistics
{stats}

## Goals and Key Results
{goals_text}

## Entries grouped by goal
{entries_text}

## Unlinked entries (no goal assigned via key result)
{unlinked_text}
{clg_context}
"#,
        review_platform = review_platform,
        phase_name = phase_name,
        stats = stats,
        goals_text = format_goals_with_key_results(goals, key_results),
        entries_text = entries_by_goal.0,
        unlinked_text = entries_by_goal.1,
        clg_context = clg_context,
    );

    let instruction = if let Some(sec) = get_section(section) {
        let base_prompt = if section == "clg_alignment" && clg_level.is_some() {
            sec.prompt_with_clg.as_deref().unwrap_or(&sec.prompt)
        } else {
            &sec.prompt
        };

        if wants_promotion && !sec.promotion_addendum.is_empty() {
            format!("{}\n\n{}", base_prompt, sec.promotion_addendum)
        } else {
            base_prompt.to_string()
        }
    } else {
        "Write a general summary of accomplishments for this period.".to_string()
    };

    format!("{}\n\n---\n\n{}", context, instruction)
}

// Tallies entries by type into a human-readable stats block for the prompt.
fn compute_stats(entries: &[BragEntry]) -> String {
    let mut prs_authored = 0;
    let mut prs_reviewed = 0;
    let mut prs_merged = 0;
    let mut bugs_fixed = 0;
    let mut bugs_filed = 0;
    let mut revisions = 0;
    let mut jira_completed = 0;
    let mut jira_stories = 0;
    let mut jira_tasks = 0;
    let mut jira_epics = 0;
    let mut confluence_pages = 0;
    let mut meetings = 0;
    let mut workshops = 0;
    let mut mentoring = 0;
    let mut presentations = 0;
    let mut design_docs = 0;
    let mut code_reviews = 0;
    let mut onboarding = 0;
    let mut learning = 0;
    let mut interviews = 0;
    let mut other = 0;

    for entry in entries {
        match entry.entry_type.as_str() {
            "pr_authored" => prs_authored += 1,
            "pr_reviewed" => prs_reviewed += 1,
            "pr_merged" => prs_merged += 1,
            "bug_fixed" => bugs_fixed += 1,
            "bug_filed" => bugs_filed += 1,
            "revision_authored" | "revision_reviewed" => revisions += 1,
            "jira_completed" => jira_completed += 1,
            "jira_story" => jira_stories += 1,
            "jira_task" => jira_tasks += 1,
            "jira_epic" => jira_epics += 1,
            "confluence_page" => confluence_pages += 1,
            "meeting" => meetings += 1,
            "workshop" => workshops += 1,
            "mentoring" => mentoring += 1,
            "presentation" => presentations += 1,
            "design_doc" => design_docs += 1,
            "code_review" => code_reviews += 1,
            "onboarding" => onboarding += 1,
            "learning" => learning += 1,
            "interview" => interviews += 1,
            _ => other += 1,
        }
    }

    format!(
        "- PRs authored: {}\n- PRs reviewed: {}\n- PRs merged: {}\n- Bugs fixed: {}\n- Bugs filed: {}\n- Phabricator revisions: {}\n- Jira tasks completed: {}\n- Jira stories: {}\n- Jira tasks: {}\n- Jira epics: {}\n- Confluence pages: {}\n- Meetings: {}\n- Workshops: {}\n- Mentoring sessions: {}\n- Presentations: {}\n- Design docs: {}\n- Code reviews: {}\n- Onboarding: {}\n- Learning: {}\n- Interviews: {}\n- Other: {}",
        prs_authored,
        prs_reviewed,
        prs_merged,
        bugs_fixed,
        bugs_filed,
        revisions,
        jira_completed,
        jira_stories,
        jira_tasks,
        jira_epics,
        confluence_pages,
        meetings,
        workshops,
        mentoring,
        presentations,
        design_docs,
        code_reviews,
        onboarding,
        learning,
        interviews,
        other
    )
}

// Formats goals with their nested key results for prompt context.
fn format_goals_with_key_results(goals: &[Goal], key_results: &[KeyResult]) -> String {
    goals
        .iter()
        .map(|g| {
            let category = g.category.as_deref().unwrap_or("general");
            let status = g.status.replace('_', " ").to_uppercase();
            let desc = g.description.as_deref().unwrap_or("");
            let krs: Vec<String> = key_results
                .iter()
                .filter(|kr| kr.goal_id == Some(g.id))
                .map(|kr| {
                    format!(
                        "  - [{}] {} ({})",
                        kr.status.replace('_', " "),
                        kr.name,
                        kr.status
                    )
                })
                .collect();
            let kr_text = if krs.is_empty() {
                "  (no key results)".to_string()
            } else {
                krs.join("\n")
            };
            format!(
                "- [{} | {}] {}: {}\n  Key Results:\n{}",
                category, status, g.title, desc, kr_text
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// Groups entries under their parent goal (via key_result.goal_id).
// Returns (grouped_text, unlinked_text) for entries with and without a goal.
fn group_entries_by_goal(
    entries: &[BragEntry],
    goals: &[Goal],
    key_results: &[KeyResult],
) -> (String, String) {
    let mut goal_entries: std::collections::HashMap<i64, Vec<&BragEntry>> =
        std::collections::HashMap::new();
    let mut unlinked: Vec<&BragEntry> = Vec::new();

    for entry in entries {
        let mut linked = false;

        // Link entries to goals via their key result's goal_id
        if let Some(kr_id) = entry.key_result_id
            && let Some(kr) = key_results.iter().find(|kr| kr.id == kr_id)
            && let Some(goal_id) = kr.goal_id
        {
            goal_entries.entry(goal_id).or_default().push(entry);
            linked = true;
        }

        if !linked {
            unlinked.push(entry);
        }
    }

    let grouped = goals
        .iter()
        .map(|goal| {
            let entries = goal_entries.get(&goal.id).cloned().unwrap_or_default();
            let entry_text = entries
                .iter()
                .map(|e| {
                    let kr_name = e
                        .key_result_id
                        .and_then(|kr_id| key_results.iter().find(|kr| kr.id == kr_id))
                        .map(|kr| kr.name.as_str())
                        .unwrap_or("?");
                    format!(
                        "  - [{}] [{}] {}: {}",
                        e.entry_type,
                        kr_name,
                        e.title,
                        e.description.as_deref().unwrap_or("")
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");

            let status = goal.status.replace('_', " ").to_uppercase();
            format!(
                "### Goal: {} [{}]\n{}\n({} entries)\n",
                goal.title,
                status,
                if entry_text.is_empty() {
                    "  (no entries)".to_string()
                } else {
                    entry_text
                },
                entries.len()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let unlinked_text = unlinked
        .iter()
        .map(|e| format!("- [{}] {}", e.entry_type, e.title))
        .collect::<Vec<_>>()
        .join("\n");

    (grouped, unlinked_text)
}

/// Assembles a prompt for AI-generated meeting prep notes.
///
/// Combines meeting metadata, OKR context (if linked via key result),
/// recent work entries, user-provided context snippets, and role-specific
/// guidance to produce structured talking points.
#[allow(clippy::too_many_arguments)]
pub fn build_meeting_prep_prompt(
    entry: &BragEntry,
    linked_goal: Option<&Goal>,
    linked_kr: Option<&KeyResult>,
    recent_entries: &[BragEntry],
    context_snippets: &[String],
    existing_note: Option<&MeetingPrepNote>,
) -> String {
    let role = entry.meeting_role.as_deref().unwrap_or("general");
    let title = &entry.title;
    let date = &entry.occurred_at;
    let time_range = match (&entry.start_time, &entry.end_time) {
        (Some(start), Some(end)) => format!("{} – {}", start, end),
        (Some(start), None) => start.clone(),
        _ => String::new(),
    };
    let recurring = entry
        .recurring_group
        .as_deref()
        .unwrap_or("one-off meeting");

    // OKR context
    let okr_context = match (linked_goal, linked_kr) {
        (Some(goal), Some(kr)) => {
            let recent_work: String = recent_entries
                .iter()
                .take(10)
                .map(|e| format!("  - [{}] {}", e.entry_type, e.title))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                r#"
## Linked OKR
Goal: {} ({})
Key Result: {} — status: {}, progress: {}%
{}
"#,
                goal.title,
                goal.status,
                kr.name,
                kr.status,
                kr.progress,
                if recent_work.is_empty() {
                    String::new()
                } else {
                    format!("\nRecent work on this KR:\n{}", recent_work)
                },
            )
        }
        _ => String::new(),
    };

    // User-provided context
    let snippets_text = if context_snippets.is_empty() {
        String::new()
    } else {
        let items: String = context_snippets
            .iter()
            .enumerate()
            .map(|(i, s)| format!("{}. {}", i + 1, s))
            .collect::<Vec<_>>()
            .join("\n");
        format!("\n## Additional Context\n{}\n", items)
    };

    // Existing notes as draft
    let existing_text = existing_note
        .and_then(|n| n.notes.as_deref())
        .filter(|s| !s.is_empty())
        .map(|notes| format!("\n## Current Draft Notes\n{}\n", notes))
        .unwrap_or_default();

    // Role-specific guidance
    let role_guidance = match role {
        "manager" => "Focus on: status updates for current work, blockers and asks, growth/career topics, feedback exchange, action items from last meeting.",
        "skip_level" => "Focus on: visibility items and high-impact work, career goals and progression, org-level impact and cross-team contributions, strategic alignment.",
        "peer" => "Focus on: collaboration updates, shared work and dependencies, alignment on approach, knowledge sharing, mutual support.",
        "stakeholder" => "Focus on: project status and milestones, decisions needed, risks and mitigations, timeline updates, resource needs.",
        "tech_lead" => "Focus on: technical decisions and trade-offs, architecture discussions, code quality and standards, technical debt, mentoring topics.",
        _ => "Focus on: key discussion points, updates to share, questions to ask, decisions needed, follow-ups from previous meetings.",
    };

    format!(
        r#"You are helping a software engineer prepare for an upcoming meeting.

## Meeting Details
- Title: {title}
- Date: {date}
- Time: {time_range}
- Role: {role}
- Series: {recurring}

{role_guidance}
{okr_context}{snippets_text}{existing_text}
---

Generate structured meeting prep notes in Markdown with these sections:

## Talking Points
- Bullet points of topics to raise

## Questions to Ask
- Specific questions for this meeting

## Updates to Share
- Status updates and accomplishments to mention

Keep it concise and actionable. Use the linked OKR data and recent work entries to make updates specific. If additional context (links, notes) was provided, incorporate that into relevant sections."#,
        title = title,
        date = date,
        time_range = time_range,
        role = role,
        recurring = recurring,
        role_guidance = role_guidance,
        okr_context = okr_context,
        snippets_text = snippets_text,
        existing_text = existing_text,
    )
}
