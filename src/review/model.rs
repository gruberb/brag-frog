//! Domain types and pure logic for the review bounded context.
//! No SQL lives here — all persistence is in `repo.rs`.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::shared::crypto::UserCrypto;
use crate::shared::error::AppError;
use crate::shared::serde_helpers::deserialize_optional_string;

// ---------------------------------------------------------------------------
// Phase
// ---------------------------------------------------------------------------

/// A performance review cycle (e.g., "H1 2025"). At most one is active per user.
/// Owns weeks, goals, key results, and summaries; deletion cascades aggressively.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BragPhase {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    /// ISO 8601 date string (`YYYY-MM-DD`).
    pub start_date: String,
    /// ISO 8601 date string (`YYYY-MM-DD`).
    pub end_date: String,
    pub is_active: bool,
    pub created_at: String,
}

/// Form input for creating a new phase.
#[derive(Debug, Deserialize)]
pub struct CreatePhase {
    pub name: String,
    pub start_date: String,
    pub end_date: String,
}

// ---------------------------------------------------------------------------
// Week
// ---------------------------------------------------------------------------

/// An ISO week within a phase. Created implicitly when the logbook is visited
/// or when a sync produces entries for a new week.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Week {
    pub id: i64,
    pub phase_id: i64,
    /// 1-based ordinal within this phase (not ISO week number).
    pub week_number: i64,
    pub iso_week: i64,
    pub year: i64,
    pub start_date: String,
    pub end_date: String,
}

impl Week {
    /// Builds a `week_id -> JSON` lookup map for template consumption.
    pub fn to_json_map(weeks: &[Week]) -> HashMap<i64, serde_json::Value> {
        let mut map = HashMap::new();
        for w in weeks {
            map.insert(
                w.id,
                serde_json::json!({
                    "id": w.id,
                    "iso_week": w.iso_week,
                    "year": w.year,
                    "start_date": w.start_date,
                    "end_date": w.end_date,
                }),
            );
        }
        map
    }
}

// Converts ISO year + week number to the Monday of that week.
pub(crate) fn iso_week_to_date(year: i32, week: u32) -> chrono::NaiveDate {
    use chrono::{Datelike, NaiveDate};
    // ISO week 1 contains January 4th
    let jan4 = NaiveDate::from_ymd_opt(year, 1, 4).unwrap();
    let jan4_weekday = jan4.weekday().num_days_from_monday();
    let week1_monday = jan4 - chrono::Duration::days(jan4_weekday as i64);
    week1_monday + chrono::Duration::weeks((week - 1) as i64)
}

// ---------------------------------------------------------------------------
// Checkin
// ---------------------------------------------------------------------------

/// Raw DB row with encrypted reflection fields.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WeeklyCheckinRow {
    pub id: i64,
    pub week_id: i64,
    pub user_id: i64,
    pub proud_of: Option<Vec<u8>>,
    pub learned: Option<Vec<u8>>,
    pub wants_to_change: Option<Vec<u8>>,
    pub frustrations: Option<Vec<u8>>,
    pub notes: Option<Vec<u8>>,
    pub energy_level: Option<i64>,
    pub productivity_rating: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

impl WeeklyCheckinRow {
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<WeeklyCheckin, AppError> {
        Ok(WeeklyCheckin {
            id: self.id,
            week_id: self.week_id,
            user_id: self.user_id,
            proud_of: crypto.decrypt_opt(&self.proud_of)?,
            learned: crypto.decrypt_opt(&self.learned)?,
            wants_to_change: crypto.decrypt_opt(&self.wants_to_change)?,
            frustrations: crypto.decrypt_opt(&self.frustrations)?,
            notes: crypto.decrypt_opt(&self.notes)?,
            energy_level: self.energy_level,
            productivity_rating: self.productivity_rating,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

/// Weekly structured reflection — one per week per user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyCheckin {
    pub id: i64,
    pub week_id: i64,
    pub user_id: i64,
    pub proud_of: Option<String>,
    pub learned: Option<String>,
    pub wants_to_change: Option<String>,
    pub frustrations: Option<String>,
    pub notes: Option<String>,
    /// 1-5 energy rating.
    pub energy_level: Option<i64>,
    /// 1-5 productivity rating.
    pub productivity_rating: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveCheckin {
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub proud_of: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub learned: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub wants_to_change: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub frustrations: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub notes: Option<String>,
    #[serde(default)]
    pub energy_level: Option<i64>,
    #[serde(default)]
    pub productivity_rating: Option<i64>,
}

/// Parameters for upserting a KR snapshot within a weekly check-in.
pub struct UpsertKrSnapshot<'a> {
    pub checkin_id: i64,
    pub key_result_id: i64,
    pub current_value: Option<f64>,
    pub confidence: &'a str,
    pub blockers: Option<&'a str>,
    pub next_week_bet: Option<&'a str>,
}

/// Per-KR snapshot within a weekly check-in.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct KrCheckinSnapshotRow {
    pub id: i64,
    pub checkin_id: i64,
    pub key_result_id: i64,
    pub current_value: Option<f64>,
    pub confidence: String,
    pub blockers: Option<Vec<u8>>,
    pub next_week_bet: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KrCheckinSnapshot {
    pub id: i64,
    pub checkin_id: i64,
    pub key_result_id: i64,
    pub current_value: Option<f64>,
    /// One of: `green`, `yellow`, `red`.
    pub confidence: String,
    pub blockers: Option<String>,
    pub next_week_bet: Option<String>,
}

impl KrCheckinSnapshotRow {
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<KrCheckinSnapshot, AppError> {
        Ok(KrCheckinSnapshot {
            id: self.id,
            checkin_id: self.checkin_id,
            key_result_id: self.key_result_id,
            current_value: self.current_value,
            confidence: self.confidence,
            blockers: crypto.decrypt_opt(&self.blockers)?,
            next_week_bet: crypto.decrypt_opt(&self.next_week_bet)?,
        })
    }
}

/// Raw DB row for checkin + week join.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CheckinWithWeekRow {
    pub id: i64,
    pub week_id: i64,
    pub user_id: i64,
    pub proud_of: Option<Vec<u8>>,
    pub learned: Option<Vec<u8>>,
    pub wants_to_change: Option<Vec<u8>>,
    pub frustrations: Option<Vec<u8>>,
    pub notes: Option<Vec<u8>>,
    pub energy_level: Option<i64>,
    pub productivity_rating: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
    pub iso_week: i64,
    pub year: i64,
    pub week_start: String,
    pub week_end: String,
    pub phase_name: String,
}

/// Decrypted checkin with week metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckinWithWeek {
    pub id: i64,
    pub week_id: i64,
    pub proud_of: Option<String>,
    pub learned: Option<String>,
    pub wants_to_change: Option<String>,
    pub frustrations: Option<String>,
    pub notes: Option<String>,
    pub energy_level: Option<i64>,
    pub productivity_rating: Option<i64>,
    pub created_at: String,
    pub iso_week: i64,
    pub year: i64,
    pub week_start: String,
    pub week_end: String,
    pub phase_name: String,
}

impl CheckinWithWeekRow {
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<CheckinWithWeek, AppError> {
        Ok(CheckinWithWeek {
            id: self.id,
            week_id: self.week_id,
            proud_of: crypto.decrypt_opt(&self.proud_of)?,
            learned: crypto.decrypt_opt(&self.learned)?,
            wants_to_change: crypto.decrypt_opt(&self.wants_to_change)?,
            frustrations: crypto.decrypt_opt(&self.frustrations)?,
            notes: crypto.decrypt_opt(&self.notes)?,
            energy_level: self.energy_level,
            productivity_rating: self.productivity_rating,
            created_at: self.created_at,
            iso_week: self.iso_week,
            year: self.year,
            week_start: self.week_start,
            week_end: self.week_end,
            phase_name: self.phase_name,
        })
    }
}

// ---------------------------------------------------------------------------
// Impact Story
// ---------------------------------------------------------------------------

/// Raw DB row with encrypted STAR fields.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ImpactStoryRow {
    pub id: i64,
    pub phase_id: i64,
    pub title: Vec<u8>,
    pub situation: Option<Vec<u8>>,
    pub actions: Option<Vec<u8>>,
    pub result: Option<Vec<u8>>,
    pub status: String,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl ImpactStoryRow {
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<ImpactStory, AppError> {
        Ok(ImpactStory {
            id: self.id,
            phase_id: self.phase_id,
            title: crypto.decrypt(&self.title)?,
            situation: crypto.decrypt_opt(&self.situation)?,
            actions: crypto.decrypt_opt(&self.actions)?,
            result: crypto.decrypt_opt(&self.result)?,
            status: self.status,
            sort_order: self.sort_order,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

/// A narrative impact story grouping entries (STAR format without the T).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactStory {
    pub id: i64,
    pub phase_id: i64,
    pub title: String,
    pub situation: Option<String>,
    pub actions: Option<String>,
    pub result: Option<String>,
    pub status: String,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateImpactStory {
    pub title: String,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub situation: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub actions: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub result: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateImpactStory {
    pub title: String,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub situation: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub actions: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub result: Option<String>,
    pub status: Option<String>,
}

// ---------------------------------------------------------------------------
// Summary
// ---------------------------------------------------------------------------

/// Raw database row with AES-256-GCM encrypted `content` and `prompt_used` BLOBs.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SummaryRow {
    pub id: i64,
    pub phase_id: i64,
    pub section: String,
    pub content: Vec<u8>,
    pub prompt_used: Option<Vec<u8>>,
    pub model_used: Option<String>,
    pub generated_at: String,
}

impl SummaryRow {
    /// Decrypts content and prompt into a [`Summary`].
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<Summary, AppError> {
        Ok(Summary {
            id: self.id,
            phase_id: self.phase_id,
            section: self.section,
            content: crypto.decrypt(&self.content)?,
            prompt_used: crypto.decrypt_opt(&self.prompt_used)?,
            model_used: self.model_used,
            generated_at: self.generated_at,
        })
    }
}

/// AI-generated self-reflection summary for one review section within a phase.
/// Each phase has at most one summary per section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub id: i64,
    pub phase_id: i64,
    pub section: String,
    pub content: String,
    pub prompt_used: Option<String>,
    pub model_used: Option<String>,
    pub generated_at: String,
}

/// A single review section definition loaded from config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewSection {
    pub slug: String,
    pub title: String,
    pub question: String,
    pub prompt: String,
    /// Optional alternate prompt used when CLG level information is available.
    #[serde(default)]
    pub prompt_with_clg: Option<String>,
    /// Extra text appended to the prompt when the user is targeting promotion.
    #[serde(default)]
    pub promotion_addendum: String,
}

/// Top-level review config loaded from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewConfig {
    pub review_platform: String,
    pub sections: Vec<ReviewSection>,
}

static REVIEW_CONFIG: OnceLock<ReviewConfig> = OnceLock::new();

/// Loads review sections from the TOML config file. Must be called once at startup.
pub fn load_review_config(path: &str) {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read review config at {}: {}", path, e));
    let config: ReviewConfig = toml::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse review config at {}: {}", path, e));
    REVIEW_CONFIG
        .set(config)
        .unwrap_or_else(|_| panic!("Review config already loaded"));
}

/// Returns the loaded review configuration.
pub fn review_config() -> &'static ReviewConfig {
    REVIEW_CONFIG
        .get()
        .expect("Review config not loaded. Call load_review_config() at startup.")
}

/// Returns section slugs from the config.
pub fn section_slugs() -> Vec<&'static str> {
    review_config()
        .sections
        .iter()
        .map(|s| s.slug.as_str())
        .collect()
}

/// Returns `true` if `s` is a recognized section slug.
pub fn is_valid_section(s: &str) -> bool {
    review_config().sections.iter().any(|sec| sec.slug == s)
}

/// Human-readable heading for a section slug.
pub fn section_title(section: &str) -> &'static str {
    review_config()
        .sections
        .iter()
        .find(|s| s.slug == section)
        .map(|s| s.title.as_str())
        .unwrap_or("Unknown")
}

/// Review prompt/question for a section slug.
pub fn section_question(section: &str) -> &'static str {
    review_config()
        .sections
        .iter()
        .find(|s| s.slug == section)
        .map(|s| s.question.as_str())
        .unwrap_or("")
}

/// Returns the ReviewSection for a given slug.
pub fn get_section(slug: &str) -> Option<&'static ReviewSection> {
    review_config().sections.iter().find(|s| s.slug == slug)
}

// ---------------------------------------------------------------------------
// AI Document
// ---------------------------------------------------------------------------

/// Raw DB row with encrypted content/prompt fields.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AiDocumentRow {
    pub id: i64,
    pub user_id: i64,
    pub phase_id: i64,
    pub doc_type: String,
    pub title: String,
    pub content: Vec<u8>,
    pub prompt_used: Option<Vec<u8>>,
    pub model_used: Option<String>,
    pub context_week_id: Option<i64>,
    pub meeting_entry_id: Option<i64>,
    pub meeting_role: Option<String>,
    pub recurring_group: Option<String>,
    pub generated_at: String,
}

impl AiDocumentRow {
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<AiDocument, AppError> {
        Ok(AiDocument {
            id: self.id,
            user_id: self.user_id,
            phase_id: self.phase_id,
            doc_type: self.doc_type,
            title: self.title,
            content: crypto.decrypt(&self.content)?,
            prompt_used: crypto.decrypt_opt(&self.prompt_used)?,
            model_used: self.model_used,
            context_week_id: self.context_week_id,
            meeting_entry_id: self.meeting_entry_id,
            meeting_role: self.meeting_role,
            recurring_group: self.recurring_group,
            generated_at: self.generated_at,
        })
    }
}

/// An AI-generated document (meeting prep, weekly digest, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiDocument {
    pub id: i64,
    pub user_id: i64,
    pub phase_id: i64,
    /// One of: `meeting_prep`, `weekly_digest`.
    pub doc_type: String,
    pub title: String,
    pub content: String,
    pub prompt_used: Option<String>,
    pub model_used: Option<String>,
    pub context_week_id: Option<i64>,
    pub meeting_entry_id: Option<i64>,
    pub meeting_role: Option<String>,
    pub recurring_group: Option<String>,
    pub generated_at: String,
}

// ---------------------------------------------------------------------------
// Meeting Prep
// ---------------------------------------------------------------------------

/// Raw DB row with encrypted notes.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MeetingPrepNoteRow {
    pub id: i64,
    pub user_id: i64,
    pub week_id: i64,
    pub entry_id: Option<i64>,
    pub notes: Option<Vec<u8>>,
    pub doc_urls: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Meeting prep note — per meeting entry per week.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingPrepNote {
    pub id: i64,
    pub user_id: i64,
    pub week_id: i64,
    pub entry_id: Option<i64>,
    pub notes: Option<String>,
    pub doc_urls: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl MeetingPrepNoteRow {
    /// Decrypt an encrypted note row into a plaintext meeting prep note struct.
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<MeetingPrepNote, AppError> {
        Ok(MeetingPrepNote {
            id: self.id,
            user_id: self.user_id,
            week_id: self.week_id,
            entry_id: self.entry_id,
            notes: crypto.decrypt_opt(&self.notes)?,
            doc_urls: self.doc_urls,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

// ---------------------------------------------------------------------------
// Meeting Rule
// ---------------------------------------------------------------------------

/// A rule for auto-classifying recurring meetings by role.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MeetingRule {
    pub id: i64,
    pub user_id: i64,
    /// One of: `recurring_group`, `title_contains`.
    pub match_type: String,
    pub match_value: String,
    /// One of: `manager`, `tech_lead`, `skip_level`, `peer`, `stakeholder`.
    pub meeting_role: String,
    pub person_name: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateMeetingRule {
    pub match_type: String,
    pub match_value: String,
    pub meeting_role: String,
    pub person_name: Option<String>,
}

// ---------------------------------------------------------------------------
// Weekly Focus
// ---------------------------------------------------------------------------

/// Raw DB row with encrypted title.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WeeklyFocusRow {
    pub id: i64,
    pub week_id: i64,
    pub user_id: i64,
    pub sort_order: i64,
    pub title: Vec<u8>,
    pub linked_type: Option<String>,
    pub linked_id: Option<i64>,
    pub link_1: Option<String>,
    pub link_2: Option<String>,
    pub link_3: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Weekly focus item — up to 3 per week per user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyFocus {
    pub id: i64,
    pub week_id: i64,
    pub user_id: i64,
    pub sort_order: i64,
    pub title: String,
    pub linked_type: Option<String>,
    pub linked_id: Option<i64>,
    pub link_1: Option<String>,
    pub link_2: Option<String>,
    pub link_3: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl WeeklyFocusRow {
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<WeeklyFocus, AppError> {
        Ok(WeeklyFocus {
            id: self.id,
            week_id: self.week_id,
            user_id: self.user_id,
            sort_order: self.sort_order,
            title: crypto.decrypt(&self.title)?,
            linked_type: self.linked_type,
            linked_id: self.linked_id,
            link_1: self.link_1,
            link_2: self.link_2,
            link_3: self.link_3,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

/// Parameters for creating a weekly focus item.
pub struct CreateFocusParams<'a> {
    pub week_id: i64,
    pub user_id: i64,
    pub sort_order: i64,
    pub title: &'a str,
    pub linked_type: Option<&'a str>,
    pub linked_id: Option<i64>,
    pub link_1: Option<&'a str>,
    pub link_2: Option<&'a str>,
    pub link_3: Option<&'a str>,
}

/// Parameters for updating a weekly focus item.
pub struct UpdateFocusParams<'a> {
    pub title: &'a str,
    pub linked_type: Option<&'a str>,
    pub linked_id: Option<i64>,
    pub link_1: Option<&'a str>,
    pub link_2: Option<&'a str>,
    pub link_3: Option<&'a str>,
}

/// Join table linking a focus item to brag entries.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WeeklyFocusEntry {
    pub focus_id: i64,
    pub entry_id: i64,
}
