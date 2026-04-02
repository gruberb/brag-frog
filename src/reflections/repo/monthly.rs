use sqlx::SqlitePool;

use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;
use crate::reflections::model::{MonthlyCheckin, MonthlyCheckinRow, SaveMonthlyCheckin};

impl MonthlyCheckin {
    /// Loads the monthly check-in for a given phase/user/month/year, if one exists.
    pub async fn find(
        pool: &SqlitePool,
        phase_id: i64,
        user_id: i64,
        month: i64,
        year: i64,
        crypto: &UserCrypto,
    ) -> Result<Option<Self>, AppError> {
        let row = sqlx::query_as::<_, MonthlyCheckinRow>(
            "SELECT * FROM monthly_checkins WHERE phase_id = ? AND user_id = ? AND month = ? AND year = ?",
        )
        .bind(phase_id)
        .bind(user_id)
        .bind(month)
        .bind(year)
        .fetch_optional(pool)
        .await?;
        row.map(|r| r.decrypt(crypto)).transpose()
    }

    /// List all monthly check-ins for a user, newest first.
    pub async fn list_for_user(
        pool: &SqlitePool,
        user_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, MonthlyCheckinRow>(
            "SELECT * FROM monthly_checkins WHERE user_id = ? ORDER BY year DESC, month DESC",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

    /// Delete a monthly check-in.
    pub async fn delete(
        pool: &SqlitePool,
        phase_id: i64,
        user_id: i64,
        month: i64,
        year: i64,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM monthly_checkins WHERE phase_id = ? AND user_id = ? AND month = ? AND year = ?")
            .bind(phase_id)
            .bind(user_id)
            .bind(month)
            .bind(year)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Inserts or updates a monthly check-in (keyed on phase + user + month + year).
    pub async fn upsert(
        pool: &SqlitePool,
        phase_id: i64,
        user_id: i64,
        input: &SaveMonthlyCheckin,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        let enc_learning = crypto.encrypt_opt(&input.learning_or_coasting)?;
        let enc_reconnect = crypto.encrypt_opt(&input.reconnect_list)?;
        let enc_energy = crypto.encrypt_opt(&input.energy_trend_note)?;
        let enc_letting = crypto.encrypt_opt(&input.letting_go)?;

        let row = sqlx::query_as::<_, MonthlyCheckinRow>(
            r#"
            INSERT INTO monthly_checkins (phase_id, user_id, month, year, learning_or_coasting, reconnect_list, energy_trend_note, letting_go)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(phase_id, user_id, month, year) DO UPDATE SET
                learning_or_coasting = excluded.learning_or_coasting,
                reconnect_list = excluded.reconnect_list,
                energy_trend_note = excluded.energy_trend_note,
                letting_go = excluded.letting_go,
                updated_at = datetime('now')
            RETURNING *
            "#,
        )
        .bind(phase_id)
        .bind(user_id)
        .bind(input.month)
        .bind(input.year)
        .bind(&enc_learning)
        .bind(&enc_reconnect)
        .bind(&enc_energy)
        .bind(&enc_letting)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }
}
