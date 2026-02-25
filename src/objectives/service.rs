use std::collections::HashMap;

use crate::objectives::model::{DepartmentGoal, Priority};

/// Status display ordering for priority sorting: active first, terminal states last.
fn status_sort_key(status: &str) -> u8 {
    match status {
        "active" => 0,
        "not_started" => 1,
        "on_hold" => 2,
        "completed" => 3,
        "cancelled" => 4,
        _ => 1,
    }
}

/// Sorts priorities by status (active → not_started → on_hold → completed → cancelled),
/// then by sort_order, then by id as tiebreaker.
pub fn sort_priorities(priorities: &mut [Priority]) {
    priorities.sort_by(|a, b| {
        status_sort_key(&a.status)
            .cmp(&status_sort_key(&b.status))
            .then(a.sort_order.cmp(&b.sort_order))
            .then(a.id.cmp(&b.id))
    });
}

/// Groups sorted priorities by their department goal. Returns a map of
/// `goal_id -> Vec<&Priority>` and a separate vec of unassigned priorities.
pub fn group_by_department_goal<'a>(
    priorities: &'a [Priority],
    _dept_goals: &[DepartmentGoal],
) -> (HashMap<String, Vec<&'a Priority>>, Vec<&'a Priority>) {
    let mut goal_priorities: HashMap<String, Vec<&Priority>> = HashMap::new();
    let mut unassigned: Vec<&Priority> = Vec::new();

    for p in priorities {
        if let Some(gid) = p.department_goal_id {
            goal_priorities
                .entry(gid.to_string())
                .or_default()
                .push(p);
        } else {
            unassigned.push(p);
        }
    }

    (goal_priorities, unassigned)
}

impl Priority {
    /// Whether this priority is in an active status.
    pub fn is_active(&self) -> bool {
        self.status == "active"
    }

    /// Whether this priority has reached a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self.status.as_str(), "completed" | "cancelled")
    }
}
