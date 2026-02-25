use crate::worklog::model::BragEntry;
use crate::objectives::model::{DepartmentGoal, Priority};
use crate::identity::clg::ClgLevel;
use crate::cycle::model::{AiDocument, MeetingPrepNote, WeeklyCheckin, WeeklyFocus};
use crate::cycle::model::get_section;

/// Assembles a complete prompt for one self-review section.
///
/// Combines phase context (stats, department goals, priorities, entries) with
/// a section-specific instruction loaded from the review_sections config.
/// Optionally embeds CLG level expectations and promotion framing.
#[allow(clippy::too_many_arguments)]
pub fn build_self_reflection_prompt(
    section: &str,
    dept_goals: &[DepartmentGoal],
    entries: &[BragEntry],
    priorities: &[Priority],
    phase_name: &str,
    clg_level: Option<&ClgLevel>,
    wants_promotion: bool,
) -> String {
    let stats = compute_stats(entries);
    let entries_by_priority = group_entries_by_priority(entries, dept_goals, priorities);

    let clg_context = if let Some(level) = clg_level {
        let mut ctx = format!(
            r#"
## Level: {} ({})
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

    let review_platform = &crate::cycle::model::review_config().review_platform;

    let context = format!(
        r#"You are helping a software engineer write their half-year self-review for a {review_platform} performance review cycle.

Phase: {phase_name}

## Statistics
{stats}

## Department Goals & Priorities
{goals_text}

## Entries grouped by priority
{entries_text}

## Unlinked entries (no priority assigned)
{unlinked_text}
{clg_context}
"#,
        review_platform = review_platform,
        phase_name = phase_name,
        stats = stats,
        goals_text = format_dept_goals_with_priorities(dept_goals, priorities),
        entries_text = entries_by_priority.0,
        unlinked_text = entries_by_priority.1,
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
        prs_authored, prs_reviewed, prs_merged, bugs_fixed, bugs_filed, revisions,
        jira_completed, jira_stories, jira_tasks, jira_epics, confluence_pages,
        meetings, workshops, mentoring, presentations, design_docs, code_reviews,
        onboarding, learning, interviews, other
    )
}

fn format_dept_goals_with_priorities(
    dept_goals: &[DepartmentGoal],
    priorities: &[Priority],
) -> String {
    dept_goals
        .iter()
        .map(|g| {
            let status = g.status.replace('_', " ").to_uppercase();
            let desc = g.description.as_deref().unwrap_or("");
            let pris: Vec<String> = priorities
                .iter()
                .filter(|p| p.department_goal_id == Some(g.id))
                .map(|p| {
                    format!(
                        "  - [{}] {}",
                        p.status.replace('_', " "),
                        p.title,
                    )
                })
                .collect();
            let pri_text = if pris.is_empty() {
                "  (no priorities)".to_string()
            } else {
                pris.join("\n")
            };
            format!(
                "- [{}] {}: {}\n  Priorities:\n{}",
                status, g.title, desc, pri_text
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// Groups entries under their parent priority (via entry.priority_id).
// Returns (grouped_text, unlinked_text).
fn group_entries_by_priority(
    entries: &[BragEntry],
    dept_goals: &[DepartmentGoal],
    priorities: &[Priority],
) -> (String, String) {
    let mut priority_entries: std::collections::HashMap<i64, Vec<&BragEntry>> =
        std::collections::HashMap::new();
    let mut unlinked: Vec<&BragEntry> = Vec::new();

    for entry in entries {
        if let Some(pid) = entry.priority_id {
            priority_entries.entry(pid).or_default().push(entry);
        } else {
            unlinked.push(entry);
        }
    }

    // Group priorities under department goals
    let grouped = dept_goals
        .iter()
        .map(|goal| {
            let goal_priorities: Vec<&Priority> = priorities
                .iter()
                .filter(|p| p.department_goal_id == Some(goal.id))
                .collect();

            let priority_text: Vec<String> = goal_priorities
                .iter()
                .map(|p| {
                    let entries = priority_entries.get(&p.id).cloned().unwrap_or_default();
                    let entry_text = entries
                        .iter()
                        .map(|e| {
                            format!(
                                "    - [{}] {}: {}",
                                e.entry_type,
                                e.title,
                                e.description.as_deref().unwrap_or("")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    format!(
                        "  ### Priority: {} [{}]\n{}\n  ({} entries)",
                        p.title,
                        p.status.replace('_', " "),
                        if entry_text.is_empty() {
                            "    (no entries)".to_string()
                        } else {
                            entry_text
                        },
                        entries.len()
                    )
                })
                .collect();

            format!(
                "## Department Goal: {} [{}]\n{}",
                goal.title,
                goal.status.replace('_', " ").to_uppercase(),
                if priority_text.is_empty() {
                    "  (no priorities)".to_string()
                } else {
                    priority_text.join("\n")
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    // Also include priorities without a department goal
    let standalone: Vec<String> = priorities
        .iter()
        .filter(|p| p.department_goal_id.is_none())
        .map(|p| {
            let entries = priority_entries.get(&p.id).cloned().unwrap_or_default();
            let entry_text = entries
                .iter()
                .map(|e| {
                    format!(
                        "  - [{}] {}: {}",
                        e.entry_type,
                        e.title,
                        e.description.as_deref().unwrap_or("")
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "### Priority: {} [{}]\n{}\n({} entries)",
                p.title,
                p.status.replace('_', " "),
                if entry_text.is_empty() {
                    "  (no entries)".to_string()
                } else {
                    entry_text
                },
                entries.len()
            )
        })
        .collect();

    let full_grouped = if standalone.is_empty() {
        grouped
    } else {
        format!("{}\n\n## Standalone Priorities\n{}", grouped, standalone.join("\n"))
    };

    let unlinked_text = unlinked
        .iter()
        .map(|e| format!("- [{}] {}", e.entry_type, e.title))
        .collect::<Vec<_>>()
        .join("\n");

    (full_grouped, unlinked_text)
}

/// Assembles a context-first prompt for AI-generated meeting prep notes.
///
/// Prioritizes concrete context (meeting goal, calendar description, user notes,
/// prior preps) over generic role templates, producing output tailored to the
/// specific meeting rather than boilerplate.
#[allow(clippy::too_many_arguments)]
pub fn build_meeting_prep_prompt(
    entry: &BragEntry,
    linked_dept_goal: Option<&DepartmentGoal>,
    linked_priority: Option<&Priority>,
    recent_entries: &[BragEntry],
    other_recent_entries: &[BragEntry],
    checkins: &[&WeeklyCheckin],
    focus_items: &[WeeklyFocus],
    context_text: &str,
    existing_note: Option<&MeetingPrepNote>,
    meeting_goal: Option<&str>,
    prior_preps: &[AiDocument],
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

    // --- Build context sections in priority order ---

    // Meeting goal — highest priority signal
    let goal_section = match meeting_goal {
        Some(goal) if !goal.is_empty() => {
            format!("\n## Meeting Goal\n{}\n", goal)
        }
        _ => String::new(),
    };

    // Calendar description (synced agenda + attendee names)
    let calendar_desc_section = entry
        .description
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|desc| format!("\n## Calendar Description\n{}\n", desc))
        .unwrap_or_default();

    // User-provided context — full text, no line limit
    let context_section = if context_text.is_empty() {
        String::new()
    } else {
        format!("\n## User-Provided Context\n{}\n", context_text)
    };

    // Prior meeting preps from the same series (truncated)
    let prior_preps_section = if prior_preps.is_empty() {
        String::new()
    } else {
        let items: String = prior_preps
            .iter()
            .take(2)
            .map(|doc| {
                let truncated: String = doc.content.chars().take(500).collect();
                let suffix = if doc.content.len() > 500 { "..." } else { "" };
                format!(
                    "### {} ({})\n{}{}\n",
                    doc.title, doc.generated_at, truncated, suffix
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!("\n## Prior Meeting Preps\n{}", items)
    };

    // Recent work across all priorities (excluding meetings, capped at 30)
    let recent_work_section = {
        let items: Vec<String> = other_recent_entries
            .iter()
            .filter(|e| e.entry_type != "meeting")
            .take(30)
            .map(|e| format!("- [{}] {}", e.entry_type, e.title))
            .collect();
        if items.is_empty() {
            String::new()
        } else {
            format!("\n## Recent Work (Last 3 Weeks)\n{}\n", items.join("\n"))
        }
    };

    // Current week's focus items
    let focus_section = if focus_items.is_empty() {
        String::new()
    } else {
        let items: String = focus_items
            .iter()
            .map(|f| format!("- {}", f.title))
            .collect::<Vec<_>>()
            .join("\n");
        format!("\n## This Week's Focus\n{}\n", items)
    };

    // Recent check-in highlights (most recent only, non-empty fields, truncated)
    let checkin_section = checkins.first().map(|c| {
        let mut parts = Vec::new();
        if let Some(h) = c.highlights_impact.as_deref().filter(|s| !s.is_empty()) {
            let truncated: String = h.chars().take(200).collect();
            let suffix = if h.len() > 200 { "..." } else { "" };
            parts.push(format!("**Highlights & Impact:** {}{}", truncated, suffix));
        }
        if let Some(s) = c.support_feedback.as_deref().filter(|s| !s.is_empty()) {
            let truncated: String = s.chars().take(200).collect();
            let suffix = if s.len() > 200 { "..." } else { "" };
            parts.push(format!("**Blockers & Support:** {}{}", truncated, suffix));
        }
        if let Some(a) = c.looking_ahead.as_deref().filter(|s| !s.is_empty()) {
            let truncated: String = a.chars().take(200).collect();
            let suffix = if a.len() > 200 { "..." } else { "" };
            parts.push(format!("**Looking Ahead:** {}{}", truncated, suffix));
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!("\n## Recent Check-in Highlights\n{}\n", parts.join("\n"))
        }
    }).unwrap_or_default();

    // Existing draft notes
    let existing_text = existing_note
        .and_then(|n| n.notes.as_deref())
        .filter(|s| !s.is_empty())
        .map(|notes| format!("\n## Current Draft Notes\n{}\n", notes))
        .unwrap_or_default();

    // Linked priority + recent work
    let priority_context = match (linked_dept_goal, linked_priority) {
        (Some(goal), Some(priority)) => {
            let recent_work: String = recent_entries
                .iter()
                .take(10)
                .map(|e| format!("  - [{}] {}", e.entry_type, e.title))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                r#"
## Linked Priority
Department Goal: {} ({})
Priority: {} — status: {}
{}
"#,
                goal.title,
                goal.status,
                priority.title,
                priority.status,
                if recent_work.is_empty() {
                    String::new()
                } else {
                    format!("\nRecent work:\n{}", recent_work)
                },
            )
        }
        (None, Some(priority)) => {
            format!(
                "\n## Linked Priority\n{} — status: {}\n",
                priority.title, priority.status
            )
        }
        _ => String::new(),
    };

    // --- Detect context quality and add caveats ---

    // Check if user context is link-only
    let has_only_links = !context_text.is_empty()
        && context_text.lines().all(|line| {
            let trimmed = line.trim();
            trimmed.is_empty()
                || trimmed.starts_with("http://")
                || trimmed.starts_with("https://")
        });

    // Thin context: no user context, no goal, no existing notes, no linked priority,
    // no calendar description, no prior preps, no recent work/checkins/focus
    let has_thin_context = context_text.is_empty()
        && meeting_goal.is_none_or(|g| g.is_empty())
        && existing_text.is_empty()
        && linked_priority.is_none()
        && entry.description.as_ref().is_none_or(|d| d.is_empty())
        && prior_preps.is_empty()
        && recent_work_section.is_empty()
        && focus_section.is_empty()
        && checkin_section.is_empty();

    let link_caveat = if has_only_links {
        "\n## Important: Link-Only Context\n\
         You cannot access URLs or links. Treat them as reference labels only.\n\
         The user provided only links with no accompanying text. Before generating prep notes, \
         include a short section titled \"Context Needed\" explaining that you can't read linked documents \
         and asking the user to paste the relevant content (agenda, discussion points, key decisions) \
         from those docs directly into the context field.\n\
         Do NOT fabricate or infer document content from URL paths.\n"
    } else if !context_text.is_empty() {
        "\nNote: You cannot access URLs or links — treat any URLs in the context as reference labels only. \
         Never fabricate content based on URL paths.\n"
    } else {
        ""
    };

    let thin_context_guidance = if has_thin_context {
        "\n## Limited Context Available\n\
         Very little context was provided for this meeting. Before the main prep sections, include a \
         \"What I'd Need to Make This Useful\" section listing:\n\
         - What's the goal for this meeting? (use the Meeting Goal field)\n\
         - What's on the agenda?\n\
         - Key decisions or topics to cover?\n\
         - Relevant context from any linked documents?\n\n\
         Still generate the standard prep sections below, but label them as generic placeholders \
         that should be refined once more context is available.\n"
    } else {
        ""
    };

    // Role guidance — placed after context so the model grounds on specifics first
    let role_hint = match role {
        "manager" => "For this manager 1:1, consider: status on current work, blockers, growth/career topics, feedback, and follow-ups.",
        "skip_level" => "For this skip-level, consider: high-impact work visibility, career goals, org-level impact, and strategic alignment.",
        "peer" => "For this peer meeting, consider: collaboration updates, shared work, alignment on approach, and knowledge sharing.",
        "stakeholder" => "For this stakeholder meeting, consider: project milestones, decisions needing input, risks, and timeline updates.",
        "tech_lead" => "For this tech lead meeting, consider: technical decisions, architecture discussions, code quality, and tech debt.",
        _ => "Consider: key discussion points, updates to share, questions to ask, decisions needed, and follow-ups.",
    };

    format!(
        r#"You are helping a software engineer prepare for an upcoming meeting. Focus on the specific context provided below rather than producing generic templates.

## Meeting Details
- Title: {title}
- Date: {date}
- Time: {time_range}
- Role: {role}
- Series: {recurring}
{goal_section}{calendar_desc_section}{context_section}{prior_preps_section}{recent_work_section}{focus_section}{checkin_section}{existing_text}{priority_context}{link_caveat}{thin_context_guidance}
## Role Guidance
{role_hint}

---

Generate structured meeting prep notes in Markdown. Choose sections that fit this specific meeting — not every meeting needs the same structure. Common sections include:

## Talking Points
- Topics to raise, grounded in the context above

## Questions to Ask
- Specific questions for this meeting

## Updates to Share
- Status updates and accomplishments to mention

Keep it concise and actionable. Use any provided context, calendar description, meeting goal, and prior preps to make the output specific to this meeting. Do not infer or fabricate content from URLs."#,
        title = title,
        date = date,
        time_range = time_range,
        role = role,
        recurring = recurring,
        goal_section = goal_section,
        calendar_desc_section = calendar_desc_section,
        context_section = context_section,
        prior_preps_section = prior_preps_section,
        recent_work_section = recent_work_section,
        focus_section = focus_section,
        checkin_section = checkin_section,
        existing_text = existing_text,
        priority_context = priority_context,
        link_caveat = link_caveat,
        thin_context_guidance = thin_context_guidance,
        role_hint = role_hint,
    )
}
