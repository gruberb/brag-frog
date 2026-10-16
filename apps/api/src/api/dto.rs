//! Wire types shared by several API modules. Domain-specific DTOs live next to their handlers.
//! Every type here is exported to TypeScript by ts-rs (see `.cargo/config.toml`).

use serde::Serialize;
use ts_rs::TS;

use crate::cycle::model::BragPhase;
use crate::identity::model::User;

/// The signed-in user without internal identifiers such as the Google subject.
#[derive(Serialize, TS)]
#[ts(export)]
pub struct UserDto {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub role: Option<String>,
    pub wants_promotion: bool,
    pub display_name: Option<String>,
    pub team: Option<String>,
    pub manager_name: Option<String>,
    pub skip_level_name: Option<String>,
    pub direct_reports: Option<String>,
    pub timezone: Option<String>,
    pub week_start: Option<String>,
    pub work_start_time: Option<String>,
    pub work_end_time: Option<String>,
}

impl From<User> for UserDto {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            email: u.email,
            name: u.name,
            avatar_url: u.avatar_url,
            role: u.role,
            wants_promotion: u.wants_promotion,
            display_name: u.display_name,
            team: u.team,
            manager_name: u.manager_name,
            skip_level_name: u.skip_level_name,
            direct_reports: u.direct_reports,
            timezone: u.timezone,
            week_start: u.week_start,
            work_start_time: u.work_start_time,
            work_end_time: u.work_end_time,
        }
    }
}

/// A performance cycle (stored as `brag_phases`).
#[derive(Serialize, TS)]
#[ts(export)]
pub struct CycleDto {
    pub id: i64,
    pub name: String,
    pub start_date: String,
    pub end_date: String,
    pub is_active: bool,
}

impl From<BragPhase> for CycleDto {
    fn from(p: BragPhase) -> Self {
        Self {
            id: p.id,
            name: p.name,
            start_date: p.start_date,
            end_date: p.end_date,
            is_active: p.is_active,
        }
    }
}
