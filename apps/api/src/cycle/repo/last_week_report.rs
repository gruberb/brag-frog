use sqlx::SqlitePool;

use crate::cycle::model::{LastWeekReport, LastWeekReportRow};
use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;

impl LastWeekReport {
    /// All saved Last Week reports of a phase, oldest week first.
    pub async fn list_for_phase(
        pool: &SqlitePool,
        phase_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, LastWeekReportRow>(
            "SELECT * FROM last_week_reports WHERE phase_id = ? ORDER BY week_id",
        )
        .bind(phase_id)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

}
