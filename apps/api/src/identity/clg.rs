use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// A single Career Level Guide level with expectations
/// across the "Problems" and "People" competency dimensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClgLevel {
    /// Short identifier (e.g., `"ic3"`, `"senior"`).
    pub id: String,
    /// Human-readable title (e.g., "Senior Software Engineer").
    pub title: String,
    /// One-sentence summary of the level's scope.
    pub one_liner: String,
    /// Problem-solving competency expectations.
    pub problems: ClgProblems,
    /// People/leadership competency expectations.
    pub people: ClgPeople,
}

/// CLG "Problems" dimension: task scope, complexity, risk, domain knowledge, strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClgProblems {
    pub task_size: String,
    pub complexity: String,
    pub risk_management: String,
    pub domain_expertise: String,
    pub strategy: String,
}

/// CLG "People" dimension: influence, responsibility, communication, change, team, DEI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClgPeople {
    pub influence: String,
    pub responsibility: String,
    pub communication: String,
    pub change_management: String,
    pub team_contributions: String,
    pub dei: String,
}

/// TOML file root structure.
#[derive(Debug, Deserialize)]
struct ClgConfig {
    levels: Vec<ClgLevel>,
}

static LEVELS: OnceLock<Vec<ClgLevel>> = OnceLock::new();

/// Loads CLG levels from the TOML config file. Must be called once at startup.
/// Panics if the file cannot be read or parsed.
pub fn load_levels(path: &str) {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read CLG config at {}: {}", path, e));
    let config: ClgConfig = toml::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse CLG config at {}: {}", path, e));
    LEVELS
        .set(config.levels)
        .unwrap_or_else(|_| panic!("CLG levels already loaded"));
}

/// Loads CLG levels from a TOML string (for testing).
#[cfg(test)]
pub fn load_levels_from_str(toml_str: &str) {
    let config: ClgConfig =
        toml::from_str(toml_str).expect("Failed to parse CLG config from string");
    // In tests, we may call this multiple times across tests, so we just ignore if already set
    let _ = LEVELS.set(config.levels);
}

/// Looks up a CLG level by its short id (e.g., `"ic3"`).
pub fn get_level(id: &str) -> Option<&'static ClgLevel> {
    all_levels().iter().find(|l| l.id == id)
}

/// Returns the level immediately above the given id, or `None` if already at the top.
pub fn get_next_level(id: &str) -> Option<&'static ClgLevel> {
    let levels = all_levels();
    let idx = levels.iter().position(|l| l.id == id)?;
    levels.get(idx + 1)
}

/// Returns the full CLG level table, ordered as defined in the config file.
pub fn all_levels() -> &'static [ClgLevel] {
    LEVELS
        .get()
        .expect("CLG levels not loaded. Call clg::load_levels() at startup.")
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_TOML: &str = r#"
[[levels]]
id = "ic3"
title = "Senior Software Engineer"
one_liner = "Break down multi week tasks."
[levels.problems]
task_size = "1 month"
complexity = "Multi-component tasks."
risk_management = "Resolve blocks independently."
domain_expertise = "Advanced in domain."
strategy = "Solid understanding of product strategy."
[levels.people]
influence = "Peers (2-5)"
responsibility = "Largely self directed."
communication = "Go-to person for peer review."
change_management = "Advocate for team changes."
team_contributions = "Go-to person on the team."
dei = "Fosters inclusive environment."

[[levels]]
id = "ic4"
title = "Staff Software Engineer"
one_liner = "Domain expert leading multi-month projects."
[levels.problems]
task_size = "2+ months"
complexity = "Ambiguous tasks requiring planning."
risk_management = "Manage team project risk."
domain_expertise = "Expert in domain."
strategy = "Turn strategy into action."
[levels.people]
influence = "Team (3-10)"
responsibility = "Tech-lead for single team."
communication = "Communicate clearly with team."
change_management = "Help navigate org change."
team_contributions = "Mentor others."
dei = "Ensures broad perspectives."

[[levels]]
id = "ic5"
title = "Senior Staff Software Engineer"
one_liner = "Lead cross-team projects."
[levels.problems]
task_size = "6+ months"
complexity = "High impact, company-visible."
risk_management = "Multi-team risk management."
domain_expertise = "Strong architectural thinking."
strategy = "Coordinate action across teams."
[levels.people]
influence = "Department (10-50)"
responsibility = "Tech-lead for multi-team projects."
communication = "Write engineering proposals."
change_management = "Lead multi-team changes."
team_contributions = "Drive cross-team collaboration."
dei = "Champion diversity across org."
"#;

    fn init_test_levels() {
        // OnceLock means this only works once per test process, but that's fine
        let _ = LEVELS.set({
            let config: ClgConfig = toml::from_str(TEST_TOML).unwrap();
            config.levels
        });
    }

    #[test]
    fn get_known_level() {
        init_test_levels();
        let level = get_level("ic3").unwrap();
        assert_eq!(level.title, "Senior Software Engineer");
    }

    #[test]
    fn get_unknown_level() {
        init_test_levels();
        assert!(get_level("ic99").is_none());
    }

    #[test]
    fn get_next_level_works() {
        init_test_levels();
        let next = get_next_level("ic3").unwrap();
        assert_eq!(next.id, "ic4");
    }

    #[test]
    fn max_level_has_no_next() {
        init_test_levels();
        assert!(get_next_level("ic5").is_none());
    }

    #[test]
    fn all_levels_count() {
        init_test_levels();
        assert_eq!(all_levels().len(), 3);
    }
}
