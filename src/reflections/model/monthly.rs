use serde::{Deserialize, Serialize};

use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;
use crate::kernel::serde_helpers::deserialize_optional_string;

/// Raw DB row with encrypted monthly reflection fields.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MonthlyCheckinRow {
    pub id: i64,
    pub phase_id: i64,
    pub user_id: i64,
    pub month: i64,
    pub year: i64,
    pub learning_or_coasting: Option<Vec<u8>>,
    pub reconnect_list: Option<Vec<u8>>,
    pub energy_trend_note: Option<Vec<u8>>,
    pub letting_go: Option<Vec<u8>>,
    pub created_at: String,
    pub updated_at: String,
}

/// Decrypted monthly growth check-in — one per month per user per phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyCheckin {
    pub id: i64,
    pub phase_id: i64,
    pub user_id: i64,
    pub month: i64,
    pub year: i64,
    pub learning_or_coasting: Option<String>,
    pub reconnect_list: Option<String>,
    pub energy_trend_note: Option<String>,
    pub letting_go: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl MonthlyCheckinRow {
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<MonthlyCheckin, AppError> {
        Ok(MonthlyCheckin {
            id: self.id,
            phase_id: self.phase_id,
            user_id: self.user_id,
            month: self.month,
            year: self.year,
            learning_or_coasting: crypto.decrypt_opt(&self.learning_or_coasting)?,
            reconnect_list: crypto.decrypt_opt(&self.reconnect_list)?,
            energy_trend_note: crypto.decrypt_opt(&self.energy_trend_note)?,
            letting_go: crypto.decrypt_opt(&self.letting_go)?,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

/// Form payload for saving a monthly check-in.
#[derive(Debug, Deserialize)]
pub struct SaveMonthlyCheckin {
    #[serde(default)]
    pub month: i64,
    #[serde(default)]
    pub year: i64,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub learning_or_coasting: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub reconnect_list: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub energy_trend_note: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub letting_go: Option<String>,
}
