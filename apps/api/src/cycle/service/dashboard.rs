use chrono::{Local, NaiveDate};
use serde::Serialize;

use crate::worklog::model::{BragEntry, is_finished_status};

/// A contiguous free block in a workday.
#[derive(Debug, Serialize)]
pub struct FocusBlock {
    pub hours: String,
    pub start: String,
    pub end: String,
}

/// A single day in the week calendar widget.
#[derive(Debug, Serialize)]
pub struct FocusDay {
    pub day_abbr: String,
    pub date: String,
    pub full_date: String,
    pub day_name: String,
    pub meetings: i32,
    pub focus_minutes: i32,
    pub blocks: Vec<FocusBlock>,
    pub is_best: bool,
}

/// Parse "HH:MM" to minutes since midnight.
fn hhmm_to_minutes(s: &str) -> Option<i32> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() == 2 {
        let h = parts[0].parse::<i32>().ok()?;
        let m = parts[1].parse::<i32>().ok()?;
        Some(h * 60 + m)
    } else {
        None
    }
}

/// Format minutes as "Xh" or "X.Yh".
fn format_minutes(mins: i32) -> String {
    if mins <= 0 {
        return "0h".to_string();
    }
    let hours = mins / 60;
    let remainder = mins % 60;
    if remainder == 0 {
        format!("{}h", hours)
    } else {
        format!("{:.1}h", mins as f64 / 60.0)
    }
}

/// Format minutes since midnight as a compact time string.
fn minutes_to_hhmm(mins: i32) -> String {
    let h = mins / 60;
    let m = mins % 60;
    if m == 0 {
        format!("{}", h)
    } else {
        format!("{}:{:02}", h, m)
    }
}

/// Compute focus blocks for each weekday (Mon-Fri) of the given week.
/// Identifies free blocks of >= 2 hours between meetings during work hours,
/// and marks the top 3 days by focus time.
pub fn compute_focus_days(
    week_start: &str,
    meetings: &[&BragEntry],
    work_start: &str,
    work_end: &str,
) -> Vec<FocusDay> {
    let work_start_min = hhmm_to_minutes(work_start).unwrap_or(9 * 60);
    let work_end_min = hhmm_to_minutes(work_end).unwrap_or(17 * 60);

    let start_date = NaiveDate::parse_from_str(week_start, "%Y-%m-%d")
        .unwrap_or_else(|_| Local::now().date_naive());

    let day_abbrs = ["Mo", "Tu", "We", "Th", "Fr"];
    let day_names = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday"];
    let mut days: Vec<FocusDay> = Vec::with_capacity(5);

    for i in 0..5 {
        let date = start_date + chrono::Duration::days(i);
        let date_str = date.format("%Y-%m-%d").to_string();

        let mut intervals: Vec<(i32, i32)> = meetings
            .iter()
            .filter(|m| m.occurred_at == date_str)
            .filter_map(|m| {
                let start = m.start_time.as_deref().and_then(hhmm_to_minutes)?;
                let end = m.end_time.as_deref().and_then(hhmm_to_minutes)?;
                Some((start.max(work_start_min), end.min(work_end_min)))
            })
            .filter(|(s, e)| s < e)
            .collect();

        intervals.sort_by_key(|&(s, _)| s);

        let meeting_count = meetings
            .iter()
            .filter(|m| m.occurred_at == date_str)
            .count() as i32;

        let mut free_blocks: Vec<(i32, i32)> = Vec::new();
        let mut prev_end = work_start_min;
        for (s, e) in &intervals {
            let gap = s - prev_end;
            if gap > 0 {
                free_blocks.push((prev_end, *s));
            }
            if *e > prev_end {
                prev_end = *e;
            }
        }
        if work_end_min > prev_end {
            free_blocks.push((prev_end, work_end_min));
        }

        free_blocks.retain(|(s, e)| (e - s) >= 120);
        free_blocks.sort_by_key(|b| std::cmp::Reverse(b.1 - b.0));
        let top_blocks: Vec<FocusBlock> = free_blocks
            .iter()
            .take(2)
            .map(|(s, e)| FocusBlock {
                hours: format_minutes(e - s),
                start: minutes_to_hhmm(*s),
                end: minutes_to_hhmm(*e),
            })
            .collect();

        let meeting_mins: i32 = intervals.iter().map(|(s, e)| e - s).sum();
        let focus_mins = (work_end_min - work_start_min) - meeting_mins;

        days.push(FocusDay {
            day_abbr: day_abbrs[i as usize].to_string(),
            date: date.format("%d").to_string(),
            full_date: date_str,
            day_name: day_names[i as usize].to_string(),
            meetings: meeting_count,
            focus_minutes: focus_mins.max(0),
            blocks: top_blocks,
            is_best: false,
        });
    }

    let mut sorted_indices: Vec<usize> = (0..days.len()).collect();
    sorted_indices.sort_by(|a, b| days[*b].focus_minutes.cmp(&days[*a].focus_minutes));
    for &idx in sorted_indices.iter().take(3) {
        if days[idx].focus_minutes > 0 {
            days[idx].is_best = true;
        }
    }

    days
}

/// Entry types that can be open work. Each one is open until its synced status is finished.
/// Types that are finished by definition (`bug_fixed`, `jira_completed`, `pr_merged`), day
/// markers (`pr_development`, `jira_progress`) and Jira containers (`jira_epic`: epics,
/// themes, initiatives, whose work shows through their child issues) never are.
pub const OPEN_WORK_TYPES: &[&str] = &[
    "pr_authored",
    "pr_reviewed",
    "pr_review_requested",
    "revision_authored",
    "bug_filed",
    "jira_task",
    "jira_story",
];

/// Filter entries to "active work": open PRs, pending review requests, revisions, bugs and
/// Jira tickets that haven't reached a finished status.
pub fn filter_active_work(entries: &[BragEntry]) -> Vec<&BragEntry> {
    entries
        .iter()
        .filter(|e| {
            OPEN_WORK_TYPES.contains(&e.entry_type.as_str())
                && !is_finished_status(e.status.as_deref())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(source: &str, entry_type: &str, status: Option<&str>) -> BragEntry {
        BragEntry {
            source: source.into(),
            entry_type: entry_type.into(),
            status: status.map(Into::into),
            ..BragEntry::default()
        }
    }

    fn active(entries: &[BragEntry]) -> Vec<(&str, Option<&str>)> {
        filter_active_work(entries)
            .into_iter()
            .map(|e| (e.entry_type.as_str(), e.status.as_deref()))
            .collect()
    }

    #[test]
    fn keeps_open_items_and_drops_finished_ones() {
        let entries = [
            entry("jira", "jira_story", Some("In Progress")),
            entry("jira", "jira_task", Some("In Review")),
            entry("jira", "jira_task", Some("Done")),
            entry("jira", "jira_epic", Some("Cancelled")),
            entry("jira", "jira_epic", Some("In Progress")),
            entry("jira", "jira_completed", Some("Done")),
            entry("github", "pr_review_requested", Some("OPEN")),
            entry("github", "pr_authored", Some("MERGED")),
            entry("phabricator", "revision_authored", Some("Abandoned")),
            entry("bugzilla", "bug_filed", Some("RESOLVED WONTFIX")),
            entry("bugzilla", "bug_filed", Some("ASSIGNED")),
            entry("bugzilla", "bug_fixed", Some("RESOLVED FIXED")),
            entry("github", "pr_development", Some("OPEN")),
        ];
        assert_eq!(
            active(&entries),
            [
                ("jira_story", Some("In Progress")),
                ("jira_task", Some("In Review")),
                ("pr_review_requested", Some("OPEN")),
                ("bug_filed", Some("ASSIGNED")),
            ]
        );
    }

    #[test]
    fn unknown_status_counts_as_open() {
        assert_eq!(active(&[entry("bugzilla", "bug_filed", None)]), [("bug_filed", None)]);
    }
}
