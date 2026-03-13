use chrono::NaiveDate;

use crate::cycle::model::BragPhase;
use crate::kernel::error::AppError;

impl BragPhase {
    /// Validates that a date falls within this phase's date range.
    pub fn validate_date_in_range(&self, date: NaiveDate) -> Result<(), AppError> {
        let phase_start = NaiveDate::parse_from_str(&self.start_date, "%Y-%m-%d")
            .map_err(|_| AppError::Internal("Invalid phase start date".to_string()))?;
        let phase_end = NaiveDate::parse_from_str(&self.end_date, "%Y-%m-%d")
            .map_err(|_| AppError::Internal("Invalid phase end date".to_string()))?;

        if date < phase_start || date > phase_end {
            return Err(AppError::BadRequest(format!(
                "Date must be within your review cycle ({} to {})",
                self.start_date, self.end_date
            )));
        }
        Ok(())
    }
}
