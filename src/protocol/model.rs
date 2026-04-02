use serde::{Deserialize, Serialize};

/// A single protocol checklist item check state.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProtocolCheck {
    pub id: i64,
    pub week_id: i64,
    pub user_id: i64,
    pub slug: String,
    pub checked: i64,
}

/// Static definition of a checklist item.
#[derive(Debug, Clone, Serialize)]
pub struct ChecklistItem {
    pub slug: &'static str,
    pub label: &'static str,
    pub group: &'static str,
}

/// All checklist items from the 10x Engineer article.
pub const CHECKLIST_ITEMS: &[ChecklistItem] = &[
    ChecklistItem { slug: "mon_priorities", label: "Review your #1, #2, #3 priorities for the week", group: "Monday" },
    ChecklistItem { slug: "mon_breakdown", label: "Break down any new project at a high level", group: "Monday" },
    ChecklistItem { slug: "mon_status", label: "Send a status update to stakeholders", group: "Monday" },
    ChecklistItem { slug: "daily_protect", label: "Protect your non-negotiable block (family, focus time)", group: "Daily" },
    ChecklistItem { slug: "daily_understand", label: "Before asking another team, understand their problem space first", group: "Daily" },
    ChecklistItem { slug: "daily_tradeoffs", label: "Frame blockers as tradeoffs, communicate immediately", group: "Daily" },
    ChecklistItem { slug: "mid_curiosity", label: "Have a curiosity-driven conversation (PM, designer, other team)", group: "Mid-week" },
    ChecklistItem { slug: "mid_ownership", label: "Check: am I owning my tasks end-to-end?", group: "Mid-week" },
    ChecklistItem { slug: "fri_energy", label: "Reflect: Did I get energy from my work this week?", group: "Friday" },
    ChecklistItem { slug: "fri_help", label: "Did I help someone else this week?", group: "Friday" },
    ChecklistItem { slug: "fri_disconnect", label: "Minimize laptop time this weekend", group: "Friday" },
    ChecklistItem { slug: "monthly_learning", label: "Am I learning something new, or coasting?", group: "Monthly" },
    ChecklistItem { slug: "monthly_reconnect", label: "Check in on key professional relationships", group: "Monthly" },
    ChecklistItem { slug: "monthly_conflict", label: "Any ongoing conflict to let go of?", group: "Monthly" },
];
