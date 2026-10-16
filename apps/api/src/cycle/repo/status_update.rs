use sqlx::SqlitePool;

use crate::cycle::model::{StatusUpdate, StatusUpdateRow};
use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;

impl StatusUpdate {
    /// All status updates of a phase, oldest week first.
    pub async fn list_for_phase(
        pool: &SqlitePool,
        phase_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, StatusUpdateRow>(
            "SELECT * FROM status_updates WHERE phase_id = ? ORDER BY week_id",
        )
        .bind(phase_id)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

}
