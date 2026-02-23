use serde::{Deserialize, Serialize};

use crate::shared::crypto::UserCrypto;
use crate::shared::error::AppError;
use crate::shared::serde_helpers::{deserialize_optional_i64, deserialize_optional_string};

/// All recognized entry types across integrated services and manual input.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EntryType {
    #[serde(rename = "pr_authored")]
    PrAuthored,
    #[serde(rename = "pr_reviewed")]
    PrReviewed,
    #[serde(rename = "pr_merged")]
    PrMerged,
    #[serde(rename = "pr_development")]
    PrDevelopment,
    #[serde(rename = "bug_fixed")]
    BugFixed,
    #[serde(rename = "bug_filed")]
    BugFiled,
    #[serde(rename = "revision_authored")]
    RevisionAuthored,
    #[serde(rename = "revision_reviewed")]
    RevisionReviewed,
    #[serde(rename = "jira_completed")]
    JiraCompleted,
    #[serde(rename = "jira_story")]
    JiraStory,
    #[serde(rename = "jira_task")]
    JiraTask,
    #[serde(rename = "jira_epic")]
    JiraEpic,
    #[serde(rename = "confluence_page")]
    ConfluencePage,
    #[serde(rename = "meeting")]
    Meeting,
    #[serde(rename = "workshop")]
    Workshop,
    #[serde(rename = "mentoring")]
    Mentoring,
    #[serde(rename = "presentation")]
    Presentation,
    #[serde(rename = "pairing")]
    Pairing,
    #[serde(rename = "cross_team")]
    CrossTeam,
    #[serde(rename = "document")]
    Document,
    #[serde(rename = "design_doc")]
    DesignDoc,
    #[serde(rename = "code_review")]
    CodeReview,
    #[serde(rename = "development")]
    Development,
    #[serde(rename = "onboarding")]
    Onboarding,
    #[serde(rename = "learning")]
    Learning,
    #[serde(rename = "interview")]
    Interview,
    #[serde(rename = "drive_created")]
    DriveCreated,
    #[serde(rename = "drive_edited")]
    DriveEdited,
    #[serde(rename = "drive_commented")]
    DriveCommented,
    #[serde(rename = "other")]
    Other,
    #[serde(rename = "decision")]
    Decision,
    #[serde(rename = "process_improvement")]
    ProcessImprovement,
    #[serde(rename = "unblocking")]
    Unblocking,
    #[serde(rename = "incident_response")]
    IncidentResponse,
    #[serde(rename = "investigation")]
    Investigation,
}

/// Human-readable display name for a sync source.
pub fn source_display_name(source: &str) -> &'static str {
    match source {
        "github" => "GitHub",
        "phabricator" => "Phabricator",
        "bugzilla" => "Bugzilla",
        "jira" => "Jira",
        "confluence" => "Confluence",
        "google_drive" => "Google Drive",
        "google_calendar" => "Google Calendar",
        "atlassian" => "Atlassian",
        "manual" => "Manual",
        _ => "Other",
    }
}

/// Map an entry type slug to its originating service/source.
/// Delegates to `EntryType::source()` — the enum is the single source of truth.
pub fn entry_type_to_source(entry_type: &str) -> &'static str {
    EntryType::ALL
        .iter()
        .find(|v| v.slug() == entry_type)
        .map(|v| v.source())
        .unwrap_or("manual")
}

impl EntryType {
    /// All entry type variants in display order.
    pub const ALL: &[EntryType] = &[
        Self::PrAuthored,
        Self::PrReviewed,
        Self::PrMerged,
        Self::PrDevelopment,
        Self::BugFixed,
        Self::BugFiled,
        Self::RevisionAuthored,
        Self::RevisionReviewed,
        Self::JiraCompleted,
        Self::JiraStory,
        Self::JiraTask,
        Self::JiraEpic,
        Self::ConfluencePage,
        Self::Meeting,
        Self::Workshop,
        Self::Mentoring,
        Self::Presentation,
        Self::Pairing,
        Self::CrossTeam,
        Self::Document,
        Self::DesignDoc,
        Self::CodeReview,
        Self::Development,
        Self::Onboarding,
        Self::Learning,
        Self::Interview,
        Self::DriveCreated,
        Self::DriveEdited,
        Self::DriveCommented,
        Self::Other,
        Self::Decision,
        Self::ProcessImprovement,
        Self::Unblocking,
        Self::IncidentResponse,
        Self::Investigation,
    ];

    /// Database/serde slug for this entry type.
    pub fn slug(&self) -> &'static str {
        match self {
            Self::PrAuthored => "pr_authored",
            Self::PrReviewed => "pr_reviewed",
            Self::PrMerged => "pr_merged",
            Self::PrDevelopment => "pr_development",
            Self::BugFixed => "bug_fixed",
            Self::BugFiled => "bug_filed",
            Self::RevisionAuthored => "revision_authored",
            Self::RevisionReviewed => "revision_reviewed",
            Self::JiraCompleted => "jira_completed",
            Self::JiraStory => "jira_story",
            Self::JiraTask => "jira_task",
            Self::JiraEpic => "jira_epic",
            Self::ConfluencePage => "confluence_page",
            Self::Meeting => "meeting",
            Self::Workshop => "workshop",
            Self::Mentoring => "mentoring",
            Self::Presentation => "presentation",
            Self::Pairing => "pairing",
            Self::CrossTeam => "cross_team",
            Self::Document => "document",
            Self::DesignDoc => "design_doc",
            Self::CodeReview => "code_review",
            Self::Development => "development",
            Self::Onboarding => "onboarding",
            Self::Learning => "learning",
            Self::Interview => "interview",
            Self::DriveCreated => "drive_created",
            Self::DriveEdited => "drive_edited",
            Self::DriveCommented => "drive_commented",
            Self::Other => "other",
            Self::Decision => "decision",
            Self::ProcessImprovement => "process_improvement",
            Self::Unblocking => "unblocking",
            Self::IncidentResponse => "incident_response",
            Self::Investigation => "investigation",
        }
    }

    /// Human-readable label for this entry type.
    pub fn label(&self) -> &'static str {
        match self {
            Self::PrAuthored => "PR Authored",
            Self::PrReviewed => "PR Reviewed",
            Self::PrMerged => "PR Merged",
            Self::PrDevelopment => "PR Development",
            Self::BugFixed => "Bug Fixed",
            Self::BugFiled => "Bug Filed",
            Self::RevisionAuthored => "Revision Authored",
            Self::RevisionReviewed => "Revision Reviewed",
            Self::JiraCompleted => "Jira Completed",
            Self::JiraStory => "Jira Story",
            Self::JiraTask => "Jira Task",
            Self::JiraEpic => "Jira Epic",
            Self::ConfluencePage => "Confluence Page",
            Self::Meeting => "Meeting",
            Self::Workshop => "Workshop",
            Self::Mentoring => "Mentoring",
            Self::Presentation => "Presentation",
            Self::Pairing => "Pairing",
            Self::CrossTeam => "Cross-Team",
            Self::Document => "Document",
            Self::DesignDoc => "Design Doc",
            Self::CodeReview => "Code Review",
            Self::Development => "Development",
            Self::Onboarding => "Onboarding",
            Self::Learning => "Learning",
            Self::Interview => "Interview",
            Self::DriveCreated => "Document Created",
            Self::DriveEdited => "Document Edited",
            Self::DriveCommented => "Document Commented",
            Self::Other => "Other",
            Self::Decision => "Decision",
            Self::ProcessImprovement => "Process Improvement",
            Self::Unblocking => "Unblocking",
            Self::IncidentResponse => "Incident Response",
            Self::Investigation => "Investigation",
        }
    }

    /// Originating service/source for this entry type.
    pub fn source(&self) -> &'static str {
        match self {
            Self::PrAuthored | Self::PrReviewed | Self::PrMerged | Self::PrDevelopment => "github",
            Self::RevisionAuthored | Self::RevisionReviewed => "phabricator",
            Self::BugFixed | Self::BugFiled => "bugzilla",
            Self::JiraCompleted | Self::JiraStory | Self::JiraTask | Self::JiraEpic => "jira",
            Self::ConfluencePage => "confluence",
            Self::DriveCreated | Self::DriveEdited | Self::DriveCommented => "google_drive",
            _ => "manual",
        }
    }

    /// True if this type is user-created (not from an external sync source).
    pub fn is_manual(&self) -> bool {
        self.source() == "manual"
    }

    /// Look up display name by slug string.
    pub fn display_name(type_str: &str) -> &'static str {
        Self::ALL
            .iter()
            .find(|v| v.slug() == type_str)
            .map(|v| v.label())
            .unwrap_or("Unknown")
    }

    /// All entry types as `{value, label}` JSON objects for form dropdowns.
    pub fn as_json_options() -> Vec<serde_json::Value> {
        Self::ALL
            .iter()
            .map(|v| serde_json::json!({"value": v.slug(), "label": v.label()}))
            .collect()
    }

    /// Manual-only entry types as `{value, label}` JSON objects.
    pub fn as_manual_json_options() -> Vec<serde_json::Value> {
        Self::ALL
            .iter()
            .filter(|v| v.is_manual())
            .map(|v| serde_json::json!({"value": v.slug(), "label": v.label()}))
            .collect()
    }

    /// Group label for this entry type (used in `<optgroup>` headers).
    fn group_label(&self) -> &'static str {
        match self.source() {
            "github" => "GitHub",
            "phabricator" => "Phabricator",
            "bugzilla" => "Bugzilla",
            "jira" => "Jira",
            "confluence" => "Confluence",
            "google_drive" => "Google Drive",
            _ => "Custom",
        }
    }

    /// All entry types grouped by source, as `[{group, options: [{value, label}]}]`.
    pub fn as_grouped_json_options() -> Vec<serde_json::Value> {
        let mut groups: Vec<(&str, Vec<serde_json::Value>)> = Vec::new();
        for v in Self::ALL {
            let group = v.group_label();
            if let Some(last) = groups.last_mut()
                && last.0 == group
            {
                last.1
                    .push(serde_json::json!({"value": v.slug(), "label": v.label()}));
            } else {
                groups.push((
                    group,
                    vec![serde_json::json!({"value": v.slug(), "label": v.label()})],
                ));
            }
        }
        groups
            .into_iter()
            .map(|(g, opts)| serde_json::json!({"group": g, "options": opts}))
            .collect()
    }

    /// Manual-only entry types as a single "Custom" group.
    pub fn as_manual_grouped_json_options() -> Vec<serde_json::Value> {
        let options: Vec<serde_json::Value> = Self::ALL
            .iter()
            .filter(|v| v.is_manual())
            .map(|v| serde_json::json!({"value": v.slug(), "label": v.label()}))
            .collect();
        vec![serde_json::json!({"group": "Custom", "options": options})]
    }
}

/// Raw database row with AES-256-GCM encrypted BLOB fields (`title`, `description`, `teams`, `collaborators`).
/// Must be decrypted via [`BragEntryRow::decrypt`] before use.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BragEntryRow {
    pub id: i64,
    pub week_id: i64,
    pub key_result_id: Option<i64>,
    pub initiative_id: Option<i64>,
    pub source: String,
    pub source_id: Option<String>,
    pub source_url: Option<String>,
    pub title: Vec<u8>,
    pub description: Option<Vec<u8>>,
    pub entry_type: String,
    pub status: Option<String>,
    pub repository: Option<String>,
    pub occurred_at: String,
    pub teams: Option<Vec<u8>>,
    pub collaborators: Option<Vec<u8>>,
    // Impact fields
    pub outcome_statement: Option<Vec<u8>>,
    pub evidence_urls: Option<String>,
    pub role: Option<String>,
    pub impact_tags: Option<String>,
    pub reach: Option<String>,
    pub complexity: Option<String>,
    // Decision fields
    pub decision_alternatives: Option<Vec<u8>>,
    pub decision_reasoning: Option<Vec<u8>>,
    pub decision_outcome: Option<Vec<u8>>,
    // Meeting fields
    pub meeting_role: Option<String>,
    pub recurring_group: Option<String>,
    // Time fields (HH:MM)
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

impl BragEntryRow {
    /// Decrypts all encrypted fields and returns a [`BragEntry`].
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<BragEntry, AppError> {
        Ok(BragEntry {
            id: self.id,
            week_id: self.week_id,
            key_result_id: self.key_result_id,
            initiative_id: self.initiative_id,
            source: self.source,
            source_id: self.source_id,
            source_url: self.source_url,
            title: crypto.decrypt(&self.title)?,
            description: crypto.decrypt_opt(&self.description)?,
            entry_type: self.entry_type,
            status: self.status,
            repository: self.repository,
            occurred_at: self.occurred_at,
            teams: crypto.decrypt_opt(&self.teams)?,
            collaborators: crypto.decrypt_opt(&self.collaborators)?,
            outcome_statement: crypto.decrypt_opt(&self.outcome_statement)?,
            evidence_urls: self.evidence_urls,
            role: self.role,
            impact_tags: self.impact_tags,
            reach: self.reach,
            complexity: self.complexity,
            decision_alternatives: crypto.decrypt_opt(&self.decision_alternatives)?,
            decision_reasoning: crypto.decrypt_opt(&self.decision_reasoning)?,
            decision_outcome: crypto.decrypt_opt(&self.decision_outcome)?,
            meeting_role: self.meeting_role,
            recurring_group: self.recurring_group,
            start_time: self.start_time,
            end_time: self.end_time,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
        })
    }
}

/// A single work item (PR, bug, doc, meeting, etc.) tracked in the logbook.
/// This is the decrypted form used in templates and application logic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BragEntry {
    pub id: i64,
    pub week_id: i64,
    pub key_result_id: Option<i64>,
    pub initiative_id: Option<i64>,
    pub source: String,
    pub source_id: Option<String>,
    pub source_url: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub entry_type: String,
    pub status: Option<String>,
    pub repository: Option<String>,
    pub occurred_at: String,
    pub teams: Option<String>,
    pub collaborators: Option<String>,
    // Impact fields
    pub outcome_statement: Option<String>,
    pub evidence_urls: Option<String>,
    pub role: Option<String>,
    pub impact_tags: Option<String>,
    pub reach: Option<String>,
    pub complexity: Option<String>,
    // Decision fields
    pub decision_alternatives: Option<String>,
    pub decision_reasoning: Option<String>,
    pub decision_outcome: Option<String>,
    // Meeting fields
    pub meeting_role: Option<String>,
    pub recurring_group: Option<String>,
    // Time fields (HH:MM)
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

/// Form payload for creating a manual entry.
#[derive(Debug, Deserialize)]
pub struct CreateEntry {
    pub week_id: i64,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub key_result_id: Option<i64>,
    pub title: String,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub description: Option<String>,
    pub entry_type: String,
    pub occurred_at: String,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub teams: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub collaborators: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub source_url: Option<String>,
}

/// Form payload for updating an existing entry.
#[derive(Debug, Deserialize)]
pub struct UpdateEntry {
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub key_result_id: Option<i64>,
    pub title: String,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub description: Option<String>,
    pub entry_type: String,
    pub occurred_at: String,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub teams: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub collaborators: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub source_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{EntryType, entry_type_to_source};

    #[test]
    fn all_count() {
        assert_eq!(EntryType::ALL.len(), 35);
    }

    #[test]
    fn all_have_labels() {
        for v in EntryType::ALL {
            assert!(!v.label().is_empty(), "Missing label for {:?}", v);
            assert!(!v.slug().is_empty(), "Missing slug for {:?}", v);
        }
    }

    #[test]
    fn slug_label_roundtrip() {
        for v in EntryType::ALL {
            assert_eq!(
                EntryType::display_name(v.slug()),
                v.label(),
                "display_name({}) doesn't match label()",
                v.slug()
            );
        }
    }

    #[test]
    fn source_consistency() {
        for v in EntryType::ALL {
            assert_eq!(
                entry_type_to_source(v.slug()),
                v.source(),
                "entry_type_to_source({}) doesn't match source()",
                v.slug()
            );
        }
    }

    #[test]
    fn manual_only_matches_source() {
        let manual: Vec<&str> = EntryType::ALL
            .iter()
            .filter(|v| v.is_manual())
            .map(|v| v.slug())
            .collect();
        let manual_json = EntryType::as_manual_json_options();
        assert_eq!(manual.len(), manual_json.len());
        for (slug, opt) in manual.iter().zip(manual_json.iter()) {
            assert_eq!(*slug, opt["value"].as_str().unwrap());
        }
    }

    #[test]
    fn display_name_known() {
        assert_eq!(EntryType::display_name("pr_authored"), "PR Authored");
        assert_eq!(EntryType::display_name("meeting"), "Meeting");
        assert_eq!(EntryType::display_name("other"), "Other");
    }

    #[test]
    fn display_name_unknown() {
        assert_eq!(EntryType::display_name("nonexistent"), "Unknown");
    }

    #[test]
    fn as_json_options() {
        let opts = EntryType::as_json_options();
        assert_eq!(opts.len(), 35);
        assert_eq!(opts[0]["value"], "pr_authored");
        assert_eq!(opts[0]["label"], "PR Authored");
    }
}
