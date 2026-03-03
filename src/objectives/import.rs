use serde::Deserialize;

/// One row from a Lattice OKR CSV export.
#[derive(Debug, Deserialize)]
pub struct LatticeRow {
    #[serde(rename = "Goal name")]
    pub goal_name: String,
    #[serde(rename = "Description")]
    pub description: Option<String>,
    #[serde(rename = "Goal type")]
    pub goal_type: String,
    #[serde(rename = "Status")]
    pub status: Option<String>,
    #[serde(rename = "Start date")]
    pub start_date: Option<String>,
    #[serde(rename = "Goal ID")]
    pub goal_id: String,
    #[serde(rename = "Parent ID")]
    pub parent_id: Option<String>,
}

/// Parses a Lattice CSV export from raw bytes.
pub fn parse_lattice_csv(bytes: &[u8]) -> Result<Vec<LatticeRow>, csv::Error> {
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
