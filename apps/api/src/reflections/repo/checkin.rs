use sqlx::SqlitePool;

use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;
use crate::reflections::model::{QuarterlyCheckin, QuarterlyCheckinRow};

// ---------------------------------------------------------------------------
// QuarterlyCheckin
// ---------------------------------------------------------------------------

impl QuarterlyCheckin {
    /// List all quarterly check-ins for a phase.
    pub async fn list_for_phase(
        pool: &SqlitePool,
        phase_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, QuarterlyCheckinRow>(
            "SELECT * FROM quarterly_checkins WHERE phase_id = ? ORDER BY year, quarter",
        )
        .bind(phase_id)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }
}
