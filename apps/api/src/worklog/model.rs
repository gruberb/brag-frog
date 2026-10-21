use serde::{Deserialize, Serialize};

use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;
use crate::kernel::serde_helpers::{deserialize_optional_i64, deserialize_optional_string};

/// All recognized entry types across integrated services and manual input.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EntryType {
    #[serde(rename = "pr_authored")]
    PrAuthored,
    #[serde(rename = "pr_reviewed")]
    PrReviewed,
    #[serde(rename = "pr_review_requested")]
    PrReviewRequested,
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
    #[serde(rename = "jira_progress")]
    JiraProgress,
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

/// Entry categories as `(slug, label)`, in display order. Every entry type belongs to exactly
/// one ([`EntryType::category`]). The logbook filter, the trends weekly chart and the trends
/// category list all read this list, so the three always agree.
pub const CATEGORIES: &[(&str, &str)] = &[
    ("code", "Code"),
    ("reviews", "Reviews"),
    ("tickets", "Tickets"),
    ("docs", "Docs"),
    ("meetings", "Meetings"),
    ("collaboration", "Collaboration"),
    ("learning", "Learning"),
    ("other", "Other"),
];

/// Display label of a category slug.
pub fn category_label(slug: &str) -> Option<&'static str> {
    CATEGORIES.iter().find(|(s, _)| *s == slug).map(|(_, label)| *label)
}

impl EntryType {
    /// All entry type variants in display order.
    pub const ALL: &[EntryType] = &[
        Self::PrAuthored,
        Self::PrReviewed,
        Self::PrReviewRequested,
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
        Self::JiraProgress,
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
            Self::PrReviewRequested => "pr_review_requested",
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
            Self::JiraProgress => "jira_progress",
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
            Self::PrReviewRequested => "PR Review Requested",
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
            Self::JiraProgress => "Jira Progress",
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
            Self::PrAuthored
            | Self::PrReviewed
            | Self::PrReviewRequested
            | Self::PrMerged
            | Self::PrDevelopment => "github",
            Self::RevisionAuthored | Self::RevisionReviewed => "phabricator",
            Self::BugFixed | Self::BugFiled => "bugzilla",
            Self::JiraCompleted
            | Self::JiraStory
            | Self::JiraTask
            | Self::JiraEpic
            | Self::JiraProgress => "jira",
            Self::ConfluencePage => "confluence",
            Self::DriveCreated | Self::DriveEdited | Self::DriveCommented => "google_drive",
            _ => "manual",
        }
    }

    /// True for types that mirror a request still waiting on the user rather than work done.
    /// Every sync replaces them: the sync deletes the ones the source no longer reports, and
    /// trends leave them out so a pending request never counts as output.
    pub fn is_pending_request(&self) -> bool {
        matches!(self, Self::PrReviewRequested)
    }

    /// True if this type is user-created (not from an external sync source).
    pub fn is_manual(&self) -> bool {
        self.source() == "manual"
    }

    /// Category slug (see [`CATEGORIES`]). The match is exhaustive on purpose: a new entry
    /// type does not compile until it picks a category, so it can never fall out of the
    /// logbook filter or into the wrong trends bucket.
    pub fn category(&self) -> &'static str {
        match self {
            Self::PrAuthored
            | Self::PrMerged
            | Self::PrDevelopment
            | Self::RevisionAuthored
            | Self::Development => "code",
            Self::PrReviewed
            | Self::PrReviewRequested
            | Self::RevisionReviewed
            | Self::CodeReview => "reviews",
            Self::BugFixed
            | Self::BugFiled
            | Self::JiraCompleted
            | Self::JiraStory
            | Self::JiraTask
            | Self::JiraEpic
            | Self::JiraProgress => "tickets",
            Self::DesignDoc
            | Self::Document
            | Self::ConfluencePage
            | Self::DriveCreated
            | Self::DriveEdited
            | Self::DriveCommented => "docs",
            Self::Meeting => "meetings",
            Self::Workshop
            | Self::Mentoring
            | Self::Presentation
            | Self::Pairing
            | Self::CrossTeam
            | Self::Interview => "collaboration",
            Self::Learning | Self::Onboarding => "learning",
            Self::Decision
            | Self::ProcessImprovement
            | Self::Unblocking
            | Self::IncidentResponse
            | Self::Investigation
            | Self::Other => "other",
        }
    }

    /// The entry type for a database/serde slug.
    pub fn from_slug(slug: &str) -> Option<EntryType> {
        Self::ALL.iter().copied().find(|t| t.slug() == slug)
    }

    /// Slugs of every entry type in `category`; empty for an unknown category.
    pub fn types_for_category(category: &str) -> Vec<&'static str> {
        Self::ALL
            .iter()
            .filter(|t| t.category() == category)
            .map(|t| t.slug())
            .collect()
    }

    /// Category slug of an entry type slug; `None` only for slugs that are not entry types.
    pub fn category_for_slug(entry_type: &str) -> Option<&'static str> {
        Self::from_slug(entry_type).map(|t| t.category())
    }

    /// See [`EntryType::is_pending_request`]; false for slugs that are not entry types.
    pub fn is_pending_request_slug(entry_type: &str) -> bool {
        Self::from_slug(entry_type).is_some_and(|t| t.is_pending_request())
    }

}

/// True when a synced status means the item needs no more work. Covers Jira status names,
/// GitHub PR states, Phabricator revision statuses and Bugzilla's `"<STATUS> <RESOLUTION>"`
/// form (`RESOLVED FIXED`, `VERIFIED DUPLICATE`), which is matched on its first word.
/// `None` (unknown) counts as not finished.
pub fn is_finished_status(status: Option<&str>) -> bool {
    const FINISHED: &[&str] = &[
        "done", "closed", "resolved", "verified", "complete", "completed", "fixed", "won't do",
        "wont do", "duplicate", "cancelled", "canceled", "declined", "merged", "abandoned",
    ];
    let Some(status) = status else {
        return false;
    };
    let status = status.trim().to_ascii_lowercase();
    let first_word = status.split_whitespace().next().unwrap_or_default();
    FINISHED.contains(&status.as_str()) || FINISHED.contains(&first_word)
}

/// Raw database row with AES-256-GCM encrypted BLOB fields (`title`, `description`, `teams`, `collaborators`).
/// Must be decrypted via [`BragEntryRow::decrypt`] before use.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BragEntryRow {
    pub id: i64,
    pub week_id: i64,
    pub priority_id: Option<i64>,
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
            priority_id: self.priority_id,
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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BragEntry {
    pub id: i64,
    pub week_id: i64,
    pub priority_id: Option<i64>,
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

impl BragEntry {
    /// Whether this entry was created by an external sync (not manual).
    pub fn is_synced(&self) -> bool {
        self.source != "manual"
    }

}

/// Form payload for creating a manual entry.
#[derive(Debug, Deserialize)]
pub struct CreateEntry {
    pub week_id: i64,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub priority_id: Option<i64>,
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
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub reach: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub complexity: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub role: Option<String>,
}

/// Form payload for updating an existing entry.
#[derive(Debug, Deserialize)]
pub struct UpdateEntry {
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub priority_id: Option<i64>,
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
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub reach: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub complexity: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub role: Option<String>,
}

/// Form payload for bulk-updating metadata on multiple entries at once.
#[derive(Debug, Deserialize)]
pub struct BulkUpdateEntries {
    /// Comma-separated entry IDs to update.
    pub entry_ids: String,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub priority_id: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub teams: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub collaborators: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub reach: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub complexity: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub role: Option<String>,
    /// "append" (default) merges teams/collaborators with existing values;
    /// "replace" overwrites them entirely.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub merge_mode: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{CATEGORIES, EntryType, category_label, is_finished_status};

    #[test]
    fn all_count() {
        assert_eq!(EntryType::ALL.len(), 37);
    }

    #[test]
    fn finished_status_across_sources() {
        for done in ["Done", "Cancelled", "Won't Do", "MERGED", "Abandoned", "RESOLVED WONTFIX", "VERIFIED FIXED"] {
            assert!(is_finished_status(Some(done)), "{done}");
        }
        for open in ["In Progress", "In Review", "To Do", "OPEN", "Needs Review", "ASSIGNED", "REOPENED"] {
            assert!(!is_finished_status(Some(open)), "{open}");
        }
        assert!(!is_finished_status(None));
    }

    #[test]
    fn all_have_labels() {
        for v in EntryType::ALL {
            assert!(!v.label().is_empty(), "Missing label for {:?}", v);
            assert!(!v.slug().is_empty(), "Missing slug for {:?}", v);
        }
    }

    #[test]
    fn categories_agree_with_their_type_lists() {
        for (cat, _) in CATEGORIES {
            for slug in EntryType::types_for_category(cat) {
                assert_eq!(EntryType::category_for_slug(slug), Some(*cat), "{slug}");
            }
        }
    }

    #[test]
    fn every_type_has_a_listed_category() {
        for t in EntryType::ALL {
            assert!(category_label(t.category()).is_some(), "{:?} -> {}", t, t.category());
        }
        // Every listed category is reachable, so the filter never offers an empty choice by design.
        for (cat, _) in CATEGORIES {
            assert!(!EntryType::types_for_category(cat).is_empty(), "{cat}");
        }
    }
}
