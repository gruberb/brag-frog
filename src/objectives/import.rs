use serde::Deserialize;

/// One row from a Lattice OKR CSV export.
/// All fields except `goal_name` are optional to accommodate different Lattice export versions.
#[derive(Debug, Deserialize)]
pub struct LatticeRow {
    #[serde(alias = "Goal name", alias = "Goal Name", alias = "Name")]
    pub goal_name: String,
    #[serde(alias = "Description", default)]
    pub description: Option<String>,
    #[serde(alias = "Goal type", alias = "Goal Type", alias = "Type", default)]
    pub goal_type: Option<String>,
    #[serde(alias = "Status", default)]
    pub status: Option<String>,
    #[serde(alias = "Start date", alias = "Start Date", default)]
    pub start_date: Option<String>,
    #[serde(alias = "Due date", alias = "Due Date", alias = "End date", alias = "End Date", default)]
    pub due_date: Option<String>,
    #[serde(alias = "Goal ID", alias = "Goal Id", alias = "ID", alias = "Id", default)]
    pub goal_id: Option<String>,
    #[serde(alias = "Parent ID", alias = "Parent Id", default)]
    pub parent_id: Option<String>,
    #[serde(alias = "Parent goal", alias = "Parent Goal", alias = "Parent", default)]
    pub parent_goal: Option<String>,
}

/// Parses a Lattice CSV export from raw bytes.
/// Strips a UTF-8 BOM if present (common in Excel exports).
pub fn parse_lattice_csv(bytes: &[u8]) -> Result<Vec<LatticeRow>, csv::Error> {
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(bytes);
    let mut rows = Vec::new();
    for result in reader.deserialize() {
        let row: LatticeRow = result?;
        rows.push(row);
    }
    Ok(rows)
}

/// Maps a Lattice status string to a Brag Frog department goal status.
pub fn map_status_dept(lattice_status: Option<&str>) -> &'static str {
    match lattice_status.map(|s| s.trim().to_lowercase()).as_deref() {
        Some("on track") | Some("on-track") => "in_progress",
        Some("behind") | Some("at risk") | Some("at-risk") => "in_progress",
        Some("closed") | Some("achieved") | Some("completed") => "completed",
        Some("not started") | Some("not-started") => "not_started",
        Some("paused") | Some("on hold") | Some("on-hold") => "on_hold",
        _ => "in_progress",
    }
}

/// Maps a Lattice status string to a Brag Frog priority status.
pub fn map_status_priority(lattice_status: Option<&str>) -> &'static str {
    match lattice_status.map(|s| s.trim().to_lowercase()).as_deref() {
        Some("on track") | Some("on-track") => "active",
        Some("behind") | Some("at risk") | Some("at-risk") => "active",
        Some("closed") | Some("achieved") | Some("completed") => "completed",
        Some("not started") | Some("not-started") => "not_started",
        Some("paused") | Some("on hold") | Some("on-hold") => "on_hold",
        Some("cancelled") | Some("canceled") => "cancelled",
        _ => "active",
    }
}

/// Returns true if the goal type indicates a department-level goal.
pub fn is_department_goal(goal_type: &str) -> bool {
    let lower = goal_type.to_lowercase();
    lower.contains("department") || lower.contains("team") || lower.contains("company")
}

/// Maps a Lattice status to a tracking_status (trajectory signal).
/// Returns `None` for statuses that don't map to a trajectory.
pub fn map_tracking_status(lattice_status: Option<&str>) -> Option<&'static str> {
    match lattice_status.map(|s| s.trim().to_lowercase()).as_deref() {
        Some("on track") | Some("on-track") => Some("on_track"),
        Some("progressing") => Some("progressing"),
        Some("behind") | Some("at risk") | Some("at-risk") | Some("off track") | Some("off-track") => Some("off_track"),
        Some("closed") | Some("achieved") | Some("completed") | Some("complete") => Some("complete"),
        Some("incomplete") => Some("incomplete"),
        Some("not started") | Some("not-started") | Some("no update") | Some("no-update") => Some("no_update"),
        _ => None,
    }
}

/// Maps a Lattice goal_type to a priority tier.
pub fn map_tier(goal_type: Option<&str>) -> Option<&'static str> {
    match goal_type.map(|s| s.trim().to_lowercase()).as_deref() {
        Some(t) if t.contains("department") || t.contains("company") => Some("department"),
        Some(t) if t.contains("team") => Some("team"),
        Some(t) if t.contains("individual") || t.contains("personal") => Some("individual"),
        _ => None,
    }
}
