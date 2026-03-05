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

/// Creates or updates department goals and priorities from parsed Lattice CSV rows.
/// Uses `external_id` for dedup — re-importing the same CSV updates existing records.
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
        if import::is_department_goal(row.goal_type.as_deref().unwrap_or("")) {
            dept_rows.push(row);
        } else {
            priority_rows.push(row);
        }
    }

    // Upsert department goals, building a lattice_id -> brag_frog_id map
    let mut lattice_to_bf: HashMap<String, i64> = HashMap::new();
    for row in &dept_rows {
        let enc_title = crypto.encrypt(&row.goal_name)?;
        let enc_description = crypto.encrypt_opt(&row.description)?;
        let status = import::map_status_dept(row.status.as_deref());
        let external_id = row.goal_id.as_deref();

        let id: i64 = if let Some(ext_id) = external_id {
            sqlx::query_scalar(
                r#"
                INSERT INTO department_goals (phase_id, title, description, status, sort_order, source, external_id)
                VALUES (?, ?, ?, ?, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM department_goals WHERE phase_id = ?), 'lattice', ?)
                ON CONFLICT(phase_id, external_id) DO UPDATE SET
                    title = excluded.title,
                    description = excluded.description,
                    status = excluded.status
                RETURNING id
                "#,
            )
            .bind(phase_id)
            .bind(&enc_title)
            .bind(&enc_description)
            .bind(status)
            .bind(phase_id)
            .bind(ext_id)
            .fetch_one(pool)
            .await?
        } else {
            let goal = DepartmentGoal::create(
                pool, phase_id, user_id,
                &CreateDepartmentGoal {
                    title: row.goal_name.clone(),
                    description: row.description.clone(),
                    status: Some(status.to_string()),
                },
                Some("lattice"),
                crypto,
            ).await?;
            goal.id
        };

        if let Some(ref gid) = row.goal_id {
            lattice_to_bf.insert(gid.clone(), id);
        }
    }

    // Also load existing department goals with external_ids in case they were
    // imported in a previous run and the current CSV references them as parents.
    let existing: Vec<(i64, Option<String>)> = sqlx::query_as(
        "SELECT id, external_id FROM department_goals WHERE phase_id = ? AND external_id IS NOT NULL",
    )
    .bind(phase_id)
    .fetch_all(pool)
    .await?;
    for (id, ext_id) in existing {
        if let Some(eid) = ext_id {
            lattice_to_bf.entry(eid).or_insert(id);
        }
    }

    // Auto-create department goals from parent references that aren't in the export.
    // Lattice individual-only exports include `Parent ID` + `Parent goal` name columns.
    for row in &priority_rows {
        if let (Some(pid), Some(pname)) = (&row.parent_id, &row.parent_goal) {
            let pname = pname.trim();
            if !pname.is_empty() && !lattice_to_bf.contains_key(pid) {
                let enc_title = crypto.encrypt(pname)?;
                let id: i64 = sqlx::query_scalar(
                    r#"
                    INSERT INTO department_goals (phase_id, title, status, sort_order, source, external_id)
                    VALUES (?, ?, 'in_progress', (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM department_goals WHERE phase_id = ?), 'lattice', ?)
                    ON CONFLICT(phase_id, external_id) DO UPDATE SET
                        title = excluded.title
                    RETURNING id
                    "#,
                )
                .bind(phase_id)
                .bind(&enc_title)
                .bind(phase_id)
                .bind(pid)
                .fetch_one(pool)
                .await?;
                lattice_to_bf.insert(pid.clone(), id);
            }
        }
    }

    // Upsert priorities, resolving parent_id to department_goal_id
    for row in &priority_rows {
        let dept_goal_id = row
            .parent_id
            .as_ref()
            .and_then(|pid| lattice_to_bf.get(pid))
            .copied();

        let enc_title = crypto.encrypt(&row.goal_name)?;
        let enc_narrative = crypto.encrypt_opt(&row.description)?;
        let status = import::map_status_priority(row.status.as_deref());
        let external_id = row.goal_id.as_deref();

        if let Some(ext_id) = external_id {
            sqlx::query(
                r#"
                INSERT INTO priorities (phase_id, user_id, title, status, color, sort_order,
                    impact_narrative, department_goal_id, external_id)
                VALUES (?, ?, ?, ?, ?, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM priorities WHERE phase_id = ?),
                    ?, ?, ?)
                ON CONFLICT(phase_id, external_id) DO UPDATE SET
                    title = excluded.title,
                    status = excluded.status,
                    impact_narrative = excluded.impact_narrative,
                    department_goal_id = excluded.department_goal_id
                "#,
            )
            .bind(phase_id)
            .bind(user_id)
            .bind(&enc_title)
            .bind(status)
            .bind(super::repo::priority::random_color())
            .bind(phase_id)
            .bind(&enc_narrative)
            .bind(dept_goal_id)
            .bind(ext_id)
            .execute(pool)
            .await?;
        } else {
            Priority::create(
                pool, phase_id, user_id,
                &CreatePriority {
                    title: row.goal_name.clone(),
                    status: Some(status.to_string()),
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
            ).await?;
        }
    }

    Ok(())
}

