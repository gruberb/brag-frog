use std::collections::HashMap;

use sqlx::SqlitePool;

use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;
use crate::objectives::{import, model::{CreateDepartmentGoal, CreatePriority, DepartmentGoal, Priority}};

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
pub fn group_by_department_goal(
    priorities: &[Priority],
) -> (HashMap<String, Vec<&Priority>>, Vec<&Priority>) {
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

/// Creates department goals and priorities from parsed Lattice CSV rows.
/// Separates rows into department-level goals and individual priorities,
/// creates goals first, then resolves parent IDs for priority assignment.
pub async fn import_lattice_rows(
    pool: &SqlitePool,
    phase_id: i64,
    user_id: i64,
    rows: &[import::LatticeRow],
    crypto: &UserCrypto,
) -> Result<(), AppError> {
    let mut dept_rows = Vec::new();
    let mut priority_rows = Vec::new();
    for row in rows {
        if import::is_department_goal(&row.goal_type) {
            dept_rows.push(row);
        } else {
            priority_rows.push(row);
        }
    }

    // Create department goals first, building a lattice_id -> brag_frog_id map
    let mut lattice_to_bf: HashMap<String, i64> = HashMap::new();
    for row in &dept_rows {
        let goal = DepartmentGoal::create(
            pool,
            phase_id,
            user_id,
            &CreateDepartmentGoal {
                title: row.goal_name.clone(),
                description: row.description.clone(),
                status: Some(import::map_status_dept(row.status.as_deref()).to_string()),
            },
            Some("lattice"),
            crypto,
        )
        .await?;
        lattice_to_bf.insert(row.goal_id.clone(), goal.id);
    }

    // Create priorities, resolving parent_id to department_goal_id
    for row in &priority_rows {
        let dept_goal_id = row
            .parent_id
            .as_ref()
            .and_then(|pid| lattice_to_bf.get(pid))
            .copied();

        Priority::create(
            pool,
            phase_id,
            user_id,
            &CreatePriority {
                title: row.goal_name.clone(),
                status: Some(import::map_status_priority(row.status.as_deref()).to_string()),
                scope: None,
                impact_narrative: row.description.clone(),
                department_goal_id: dept_goal_id,
                priority_level: None,
                measure_type: None,
                measure_start: None,
                measure_target: None,
                description: None,
            },
            crypto,
        )
        .await?;
    }

    Ok(())
}

