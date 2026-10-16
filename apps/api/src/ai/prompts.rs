use std::collections::{HashMap, HashSet};

use crate::cycle::model::MeetingPrepNote;
use crate::identity::clg::ClgLevel;
use crate::objectives::model::{DepartmentGoal, Priority, PriorityUpdate};
use crate::review::model::{AiDocument, ContributionExample, get_section};
use crate::worklog::model::BragEntry;

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
    contribution_examples: &[ContributionExample],
    example_entry_ids: &HashMap<i64, Vec<i64>>,
    focused_dept_goal_ids: &[i64],
    phase_name: &str,
    clg_level: Option<&ClgLevel>,
    wants_promotion: bool,
) -> String {
    let scoped = scope_self_reflection_data(
        dept_goals,
        entries,
        priorities,
        contribution_examples,
        example_entry_ids,
        focused_dept_goal_ids,
    );
    let stats = compute_stats(&scoped.entries);
    let entries_by_priority =
        group_entries_by_priority(&scoped.entries, &scoped.dept_goals, &scoped.priorities);
    let contribution_examples_text = format_contribution_examples(
        &scoped.contribution_examples,
        &scoped.example_entry_ids,
        &scoped.entries,
    );
    let focused_dept_goal_context = format_focused_department_goals(
        &scoped.dept_goals,
        &scoped.priorities,
        focused_dept_goal_ids,
    );

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

    let review_platform = &crate::review::model::review_config().review_platform;

    let context = format!(
        r#"You are helping a software engineer write their half-year self-review for a {review_platform} performance review cycle.

Phase: {phase_name}

## Statistics
{stats}

## Department Goals & Priorities
{goals_text}

{focused_dept_goal_context}
## Entries grouped by priority
{entries_text}

## Contribution examples
{contribution_examples_text}

## Unlinked entries (no priority assigned)
{unlinked_text}
{clg_context}
"#,
        review_platform = review_platform,
        phase_name = phase_name,
        stats = stats,
        goals_text = format_dept_goals_with_priorities(&scoped.dept_goals, &scoped.priorities),
        focused_dept_goal_context = focused_dept_goal_context,
        entries_text = entries_by_priority.0,
        contribution_examples_text = contribution_examples_text,
        unlinked_text = entries_by_priority.1,
        clg_context = clg_context,
    );

    let instruction = if let Some(sec) = get_section(section) {
        let base_prompt = if clg_level.is_some() {
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

struct ScopedSelfReflectionData {
    dept_goals: Vec<DepartmentGoal>,
    priorities: Vec<Priority>,
    entries: Vec<BragEntry>,
    contribution_examples: Vec<ContributionExample>,
    example_entry_ids: HashMap<i64, Vec<i64>>,
}

fn scope_self_reflection_data(
    dept_goals: &[DepartmentGoal],
    entries: &[BragEntry],
    priorities: &[Priority],
    contribution_examples: &[ContributionExample],
    example_entry_ids: &HashMap<i64, Vec<i64>>,
    focused_dept_goal_ids: &[i64],
) -> ScopedSelfReflectionData {
    if focused_dept_goal_ids.is_empty() {
        return ScopedSelfReflectionData {
            dept_goals: dept_goals.to_vec(),
            priorities: priorities.to_vec(),
            entries: entries.to_vec(),
            contribution_examples: contribution_examples.to_vec(),
            example_entry_ids: example_entry_ids.clone(),
        };
    }

    let focused_goal_ids: HashSet<i64> = focused_dept_goal_ids.iter().copied().collect();
    let scoped_dept_goals: Vec<DepartmentGoal> = dept_goals
        .iter()
        .filter(|goal| focused_goal_ids.contains(&goal.id))
        .cloned()
        .collect();

    if scoped_dept_goals.is_empty() {
        return ScopedSelfReflectionData {
            dept_goals: dept_goals.to_vec(),
            priorities: priorities.to_vec(),
            entries: entries.to_vec(),
            contribution_examples: contribution_examples.to_vec(),
            example_entry_ids: example_entry_ids.clone(),
        };
    }

    let scoped_priority_ids: HashSet<i64> = priorities
        .iter()
        .filter(|priority| {
            priority
                .department_goal_id
                .is_some_and(|id| focused_goal_ids.contains(&id))
        })
        .map(|priority| priority.id)
        .collect();

    let scoped_priorities: Vec<Priority> = priorities
        .iter()
        .filter(|priority| scoped_priority_ids.contains(&priority.id))
        .cloned()
        .collect();

    let scoped_entries: Vec<BragEntry> = entries
        .iter()
        .filter(|entry| {
            entry
                .priority_id
                .is_some_and(|id| scoped_priority_ids.contains(&id))
        })
        .cloned()
        .collect();
    let scoped_entry_ids: HashSet<i64> = scoped_entries.iter().map(|entry| entry.id).collect();

    let scoped_contribution_examples: Vec<ContributionExample> = contribution_examples
        .iter()
        .filter(|example| {
            example_entry_ids
                .get(&example.id)
                .is_some_and(|ids| ids.iter().any(|id| scoped_entry_ids.contains(id)))
        })
        .cloned()
        .collect();

    let scoped_example_entry_ids = scoped_contribution_examples
        .iter()
        .filter_map(|example| {
            let scoped_ids: Vec<i64> = example_entry_ids
                .get(&example.id)?
                .iter()
                .copied()
                .filter(|id| scoped_entry_ids.contains(id))
                .collect();
            Some((example.id, scoped_ids))
        })
        .collect();

    ScopedSelfReflectionData {
        dept_goals: scoped_dept_goals,
        priorities: scoped_priorities,
        entries: scoped_entries,
        contribution_examples: scoped_contribution_examples,
        example_entry_ids: scoped_example_entry_ids,
    }
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
                    let tracking = p
                        .tracking_status
                        .as_deref()
                        .map(|t| format!(" ({})", t.replace('_', " ")))
                        .unwrap_or_default();
                    format!(
                        "  - [{}{}] {}",
                        p.status.replace('_', " "),
                        tracking,
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

fn format_focused_department_goals(
    dept_goals: &[DepartmentGoal],
    priorities: &[Priority],
    focused_dept_goal_ids: &[i64],
) -> String {
    if focused_dept_goal_ids.is_empty() {
        return String::new();
    }

    let selected: Vec<String> = focused_dept_goal_ids
        .iter()
        .filter_map(|id| dept_goals.iter().find(|g| g.id == *id))
        .map(|goal| {
            let description = goal
                .description
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .map(|s| format!(" - {}", s))
                .unwrap_or_default();
            let mut lines = vec![format!(
                "- {} [{}]{}",
                goal.title,
                goal.status.replace('_', " "),
                description,
            )];

            let linked_priorities: Vec<String> = priorities
                .iter()
                .filter(|p| p.department_goal_id == Some(goal.id))
                .map(|p| {
                    let tracking = p
                        .tracking_status
                        .as_deref()
                        .map(|t| format!(", tracking: {}", t.replace('_', " ")))
                        .unwrap_or_default();
                    let description = p
                        .description
                        .as_deref()
                        .filter(|s| !s.trim().is_empty())
                        .map(|s| format!(" - {}", s))
                        .unwrap_or_default();
                    let impact = p
                        .impact_narrative
                        .as_deref()
                        .filter(|s| !s.trim().is_empty())
                        .map(|s| format!(" Impact: {}", s))
                        .unwrap_or_default();

                    format!(
                        "  - Priority: {} [{}{}]{}{}",
                        p.title,
                        p.status.replace('_', " "),
                        tracking,
                        description,
                        impact
                    )
                })
                .collect();

            if linked_priorities.is_empty() {
                lines.push("  - (no linked priorities)".to_string());
            } else {
                lines.extend(linked_priorities);
            }

            lines.join("\n")
        })
        .collect();

    if selected.is_empty() {
        return String::new();
    }

    format!(
        "## User-selected focus department goals\n{}\n\nOnly use these selected department goals as the scope for this generated answer. Use their linked priorities and entries as evidence. Do not draft around or borrow examples from unselected department goals.\n",
        selected.join("\n")
    )
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
                    let tracking = p
                        .tracking_status
                        .as_deref()
                        .map(|t| format!(", tracking: {}", t.replace('_', " ")))
                        .unwrap_or_default();
                    format!(
                        "  ### Priority: {} [{}{}]\n{}\n  ({} entries)",
                        p.title,
                        p.status.replace('_', " "),
                        tracking,
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
            let tracking = p
                .tracking_status
                .as_deref()
                .map(|t| format!(", tracking: {}", t.replace('_', " ")))
                .unwrap_or_default();
            format!(
                "### Priority: {} [{}{}]\n{}\n({} entries)",
                p.title,
                p.status.replace('_', " "),
                tracking,
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
        format!(
            "{}\n\n## Standalone Priorities\n{}",
            grouped,
            standalone.join("\n")
        )
    };

    let unlinked_text = unlinked
        .iter()
        .map(|e| format!("- [{}] {}", e.entry_type, e.title))
        .collect::<Vec<_>>()
        .join("\n");

    (full_grouped, unlinked_text)
}

fn format_contribution_examples(
    examples: &[ContributionExample],
    example_entry_ids: &HashMap<i64, Vec<i64>>,
    entries: &[BragEntry],
) -> String {
    if examples.is_empty() {
        return "(none recorded)".to_string();
    }

    let entries_by_id: HashMap<i64, &BragEntry> =
        entries.iter().map(|entry| (entry.id, entry)).collect();

    examples
        .iter()
        .map(|example| {
            let mut lines = vec![format!("### {}", example.title)];

            let mut metadata = Vec::new();
            metadata.push(format!("status: {}", example.status));
            if let Some(assessment_type) = &example.assessment_type {
                metadata.push(format!("assessment: {}", assessment_type.replace('_', " ")));
            }
            if let Some(impact_level) = &example.impact_level {
                metadata.push(format!("impact level: {}", impact_level.replace('_', " ")));
            }
            lines.push(format!("Metadata: {}", metadata.join("; ")));

            if let Some(outcome) = example.outcome.as_deref().filter(|s| !s.trim().is_empty()) {
                lines.push(format!("Outcome: {}", outcome));
            }
            if let Some(behaviors) = example
                .behaviors
                .as_deref()
                .filter(|s| !s.trim().is_empty())
            {
                lines.push(format!("Behaviors: {}", behaviors));
            }
            if let Some(learnings) = example
                .learnings
                .as_deref()
                .filter(|s| !s.trim().is_empty())
            {
                lines.push(format!("Learnings: {}", learnings));
            }

            let linked_entries: Vec<String> = example_entry_ids
                .get(&example.id)
                .into_iter()
                .flatten()
                .filter_map(|entry_id| entries_by_id.get(entry_id))
                .map(|entry| format_entry_evidence(entry))
                .collect();

            if !linked_entries.is_empty() {
                lines.push(format!("Linked evidence:\n{}", linked_entries.join("\n")));
            }

            lines.join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn format_entry_evidence(entry: &BragEntry) -> String {
    let mut details = Vec::new();
    if let Some(repository) = entry.repository.as_deref().filter(|s| !s.trim().is_empty()) {
        details.push(format!("repo: {}", repository));
    }
    if let Some(status) = entry.status.as_deref().filter(|s| !s.trim().is_empty()) {
        details.push(format!("status: {}", status));
    }
    if let Some(outcome) = entry
        .outcome_statement
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        details.push(format!("outcome: {}", outcome));
    }
    if let Some(reach) = entry.reach.as_deref().filter(|s| !s.trim().is_empty()) {
        details.push(format!("reach: {}", reach));
    }
    if let Some(complexity) = entry.complexity.as_deref().filter(|s| !s.trim().is_empty()) {
        details.push(format!("complexity: {}", complexity));
    }
    if let Some(role) = entry.role.as_deref().filter(|s| !s.trim().is_empty()) {
        details.push(format!("role: {}", role));
    }
    if let Some(collaborators) = entry
        .collaborators
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        details.push(format!("collaborators: {}", collaborators));
    }
    if let Some(url) = entry.source_url.as_deref().filter(|s| !s.trim().is_empty()) {
        details.push(format!("url: {}", url));
    }

    if details.is_empty() {
        format!(
            "- [{} on {}] {}",
            entry.entry_type, entry.occurred_at, entry.title
        )
    } else {
        format!(
            "- [{} on {}] {} ({})",
            entry.entry_type,
            entry.occurred_at,
            entry.title,
            details.join("; ")
        )
    }
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
            trimmed.is_empty() || trimmed.starts_with("http://") || trimmed.starts_with("https://")
        });

    // Thin context: no user context, no goal, no existing notes, no linked priority,
    // no calendar description, no prior preps, and no recent work.
    let has_thin_context = context_text.is_empty()
        && meeting_goal.is_none_or(|g| g.is_empty())
        && existing_text.is_empty()
        && linked_priority.is_none()
        && entry.description.as_ref().is_none_or(|d| d.is_empty())
        && prior_preps.is_empty()
        && recent_work_section.is_empty();

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
        "manager" => {
            "For this manager 1:1, consider: status on current work, blockers, growth/career topics, feedback, and follow-ups."
        }
        "skip_level" => {
            "For this skip-level, consider: high-impact work visibility, career goals, org-level impact, and strategic alignment."
        }
        "peer" => {
            "For this peer meeting, consider: collaboration updates, shared work, alignment on approach, and knowledge sharing."
        }
        "stakeholder" => {
            "For this stakeholder meeting, consider: project milestones, decisions needing input, risks, and timeline updates."
        }
        "tech_lead" => {
            "For this tech lead meeting, consider: technical decisions, architecture discussions, code quality, and tech debt."
        }
        _ => {
            "Consider: key discussion points, updates to share, questions to ask, decisions needed, and follow-ups."
        }
    };

    format!(
        r#"You are helping a software engineer prepare for an upcoming meeting. Focus on the specific context provided below rather than producing generic templates.

## Meeting Details
- Title: {title}
- Date: {date}
- Time: {time_range}
- Role: {role}
- Series: {recurring}
{goal_section}{calendar_desc_section}{context_section}{prior_preps_section}{recent_work_section}{existing_text}{priority_context}{link_caveat}{thin_context_guidance}
## Role Guidance
{role_hint}

---

Generate structured meeting prep notes in Markdown. ALWAYS start with a numbered Talking Points list — these map directly to Lattice's 1:1 talking points format. Then add supporting context sections as appropriate.

## Talking Points
1. [First talking point — concise, actionable]
2. [Second talking point]
3. [...]

## Questions to Ask
- Specific questions for this meeting

## Updates to Share
- Status updates and accomplishments to mention

## Supporting Context
- Background details, relevant data points

Keep talking points concise (1-2 sentences each) so they can be directly pasted as individual talking points in Lattice's 1:1 interface. Use provided context, calendar description, meeting goal, and prior preps to make output specific to this meeting. Do not infer or fabricate content from URLs."#,
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
        existing_text = existing_text,
        priority_context = priority_context,
        link_caveat = link_caveat,
        thin_context_guidance = thin_context_guidance,
        role_hint = role_hint,
    )
}

/// A slice of entries all rolled up under the same priority (or `None` for
/// un-linked work). Passed to the last-week summary prompt so the model can
/// narrate progress per priority rather than per entry-type.
pub struct EntryGroup<'a> {
    pub priority: Option<&'a Priority>,
    pub dept_goal: Option<&'a DepartmentGoal>,
    pub entries: Vec<&'a BragEntry>,
}

/// Builds an AI prompt for a "What did I do last week?" summary, grouped
/// by the priority / department goal each entry rolls up to. The caller is
/// responsible for pre-grouping entries — the prompt reflects that grouping
/// directly so the model can emit one narrative per priority.
pub fn build_last_week_summary_prompt(
    groups: &[EntryGroup<'_>],
    week_start: &str,
    week_end: &str,
) -> String {
    let mut ctx = String::new();
    ctx.push_str(&format!("Window: {} to {}\n\n", week_start, week_end));

    // Emit each priority bucket with its entries. Un-linked work falls under
    // "Unassigned" so the model still has somewhere to report it.
    for group in groups {
        if group.entries.is_empty() {
            continue;
        }
        let heading = match (group.dept_goal, group.priority) {
            (Some(dg), Some(p)) => format!("## {} — {}", dg.title, p.title),
            (None, Some(p)) => format!("## {}", p.title),
            _ => "## Unassigned".to_string(),
        };
        ctx.push_str(&heading);
        ctx.push('\n');
        if let Some(p) = group.priority {
            let tracking = p.tracking_status.as_deref().unwrap_or("no update");
            ctx.push_str(&format!(
                "_Priority status: {} · tracking: {}_\n",
                p.status, tracking
            ));
        }
        for e in group.entries.iter().take(30) {
            ctx.push_str(&format!("- [{}] {}", e.entry_type, e.title));
            if let Some(ref status) = e.status {
                ctx.push_str(&format!(" ({})", status));
            }
            ctx.push('\n');
        }
        if group.entries.len() > 30 {
            ctx.push_str(&format!(
                "... and {} more entries in this group\n",
                group.entries.len() - 30
            ));
        }
        ctx.push('\n');
    }

    ctx.push_str("---\n\n");
    ctx.push_str(
        "Generate a summary of work since the window start, organised by priority.\n\n\
         For each priority heading above, emit a matching `## [priority title]` section \
         with 2–4 first-person bullet points answering: what shipped, what progressed, \
         and any key meetings or help given tied to that priority. For the Unassigned \
         group, surface anything notable (reviews, cross-team work, interrupts) under \
         an `## Unassigned` heading.\n\n\
         Rules: First person. Bullet points. Be specific and evidence-based — cite \
         entries rather than inventing context. Skip priorities with no real activity. \
         Keep it scannable — a manager should read this in 60 seconds.",
    );

    ctx
}

/// Builds an AI prompt for generating a stakeholder status update.
pub fn build_status_update_prompt(
    entries: &[BragEntry],
    priorities: &[Priority],
    blocker_updates: &[PriorityUpdate],
    week_start: &str,
    week_end: &str,
) -> String {
    let mut ctx = String::new();
    ctx.push_str(&format!("Week: {} to {}\n\n", week_start, week_end));

    // Priorities
    if !priorities.is_empty() {
        ctx.push_str("## Active Priorities\n");
        for p in priorities {
            if p.status == "active" {
                let tracking = p.tracking_status.as_deref().unwrap_or("no update");
                ctx.push_str(&format!("- {} (tracking: {})\n", p.title, tracking));
            }
        }
        ctx.push('\n');
    }

    // Blockers
    if !blocker_updates.is_empty() {
        ctx.push_str("## Active Blockers\n");
        for b in blocker_updates {
            if let Some(ref comment) = b.comment {
                ctx.push_str(&format!("- {}\n", comment));
            }
            if let Some(ref tradeoff) = b.tradeoff_text {
                ctx.push_str(&format!("  Tradeoff: {}\n", tradeoff));
            }
        }
        ctx.push('\n');
    }

    // Entries summary
    if !entries.is_empty() {
        ctx.push_str("## Work Done This Week\n");
        for e in entries.iter().take(40) {
            ctx.push_str(&format!("- [{}] {}", e.entry_type, e.title));
            if let Some(ref status) = e.status {
                ctx.push_str(&format!(" ({})", status));
            }
            ctx.push('\n');
        }
        if entries.len() > 40 {
            ctx.push_str(&format!("... and {} more entries\n", entries.len() - 40));
        }
        ctx.push('\n');
    }

    ctx.push_str("---\n\n");
    ctx.push_str(
        "Generate a concise stakeholder status update from the context above. Format:\n\n\
         ## Progress\n\
         Bullet points of what shipped, what progressed, key outcomes. Be specific.\n\n\
         ## Blockers & Tradeoffs\n\
         For each blocker, frame it as a decision for stakeholders:\n\
         \"We can [option A] if we [sacrifice], or [option B] but [consequence]\"\n\
         If no blockers, say \"No blockers this week.\"\n\n\
         ## Next Week\n\
         Top 3 focus areas for next week.\n\n\
         Rules: First person, concise, professional. Stakeholders should be able to scan this in 30 seconds. \
         Never say \"we're behind\" — frame delays as tradeoffs with options.",
    );

    ctx
}

/// Builds the AI prompt for a single quarterly check-in section.
///
/// The model gets the section's question and instruction from
/// `checkin_sections.toml` plus brag entries logged during the quarter. Returns
/// plain text that the caller can drop into the textarea; no persistence happens here.
pub fn build_quarterly_checkin_prompt(
    section_question: &str,
    section_instruction: &str,
    entries: &[BragEntry],
    quarter: &str,
    year: i64,
) -> String {
    let mut ctx = String::new();
    ctx.push_str(&format!(
        "You are helping a software engineer prepare for their {} {} quarterly conversation with their manager.\n\n",
        quarter, year
    ));
    ctx.push_str(&format!("## Question\n{}\n\n", section_question));

    // Brag entries from the quarter give the model concrete artefacts to anchor the
    // narrative. Cap the list so we do not blow past the model's context window on
    // active quarters.
    if !entries.is_empty() {
        ctx.push_str(&format!(
            "## Work Logged This Quarter ({} entries)\n",
            entries.len()
        ));
        for e in entries.iter().take(60) {
            ctx.push_str(&format!("- [{}] {}", e.entry_type, e.title));
            if let Some(ref status) = e.status {
                ctx.push_str(&format!(" ({})", status));
            }
            ctx.push('\n');
        }
        if entries.len() > 60 {
            ctx.push_str(&format!("... and {} more entries\n", entries.len() - 60));
        }
        ctx.push('\n');
    }

    ctx.push_str("---\n\n");
    ctx.push_str(section_instruction);

    ctx
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Once;

    use super::*;

    fn init_review_config() {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            crate::review::model::load_review_config("config/review_sections.toml");
        });
    }

    fn goal(id: i64, title: &str) -> DepartmentGoal {
        DepartmentGoal {
            id,
            phase_id: 1,
            title: title.to_string(),
            description: None,
            status: "in_progress".to_string(),
            sort_order: id,
            source: "manual".to_string(),
            created_at: "2026-01-01".to_string(),
        }
    }

    fn priority(id: i64, department_goal_id: i64, title: &str) -> Priority {
        Priority {
            id,
            phase_id: 1,
            user_id: 1,
            title: title.to_string(),
            status: "active".to_string(),
            color: None,
            sort_order: id,
            scope: None,
            started_at: None,
            completed_at: None,
            impact_narrative: None,
            department_goal_id: Some(department_goal_id),
            created_at: "2026-01-01".to_string(),
            priority_level: None,
            measure_type: None,
            measure_start: None,
            measure_target: None,
            measure_current: None,
            description: None,
            tracking_status: None,
            due_date: None,
            tier: None,
        }
    }

    fn entry(id: i64, priority_id: Option<i64>, title: &str) -> BragEntry {
        BragEntry {
            id,
            week_id: 1,
            priority_id,
            source: "manual".to_string(),
            source_id: None,
            source_url: None,
            title: title.to_string(),
            description: None,
            entry_type: "other".to_string(),
            status: None,
            repository: None,
            occurred_at: "2026-01-01".to_string(),
            teams: None,
            collaborators: None,
            outcome_statement: None,
            evidence_urls: None,
            role: None,
            impact_tags: None,
            reach: None,
            complexity: None,
            decision_alternatives: None,
            decision_reasoning: None,
            decision_outcome: None,
            meeting_role: None,
            recurring_group: None,
            start_time: None,
            end_time: None,
            created_at: "2026-01-01".to_string(),
            updated_at: "2026-01-01".to_string(),
            deleted_at: None,
        }
    }

    fn example(id: i64, title: &str) -> ContributionExample {
        ContributionExample {
            id,
            phase_id: 1,
            title: title.to_string(),
            outcome: None,
            behaviors: None,
            impact_level: None,
            learnings: None,
            assessment_type: None,
            status: "draft".to_string(),
            sort_order: id,
            created_at: "2026-01-01".to_string(),
            updated_at: "2026-01-01".to_string(),
        }
    }

    #[test]
    fn selected_department_goals_scope_self_review_prompt_context() {
        init_review_config();

        let goals = vec![
            goal(1, "Ship Soccer World Cup"),
            goal(2, "Help ship Autopush"),
        ];
        let priorities = vec![
            priority(10, 1, "World Cup schedule API"),
            priority(20, 2, "Autopush memory fixes"),
        ];
        let entries = vec![
            entry(100, Some(10), "World Cup standings entry"),
            entry(200, Some(20), "Autopush performance entry"),
            entry(300, None, "Unlinked planning entry"),
        ];
        let examples = vec![
            example(1000, "World Cup launch example"),
            example(2000, "Autopush performance example"),
        ];
        let example_entry_ids = HashMap::from([(1000, vec![100]), (2000, vec![200])]);

        let prompt = build_self_reflection_prompt(
            "impact_examples",
            &goals,
            &entries,
            &priorities,
            &examples,
            &example_entry_ids,
            &[1],
            "2026 H1",
            None,
            false,
        );

        assert!(prompt.contains("Ship Soccer World Cup"));
        assert!(prompt.contains("World Cup schedule API"));
        assert!(prompt.contains("World Cup standings entry"));
        assert!(prompt.contains("World Cup launch example"));
        assert!(!prompt.contains("Autopush"));
        assert!(!prompt.contains("Unlinked planning entry"));
    }
}
