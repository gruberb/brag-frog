-- Remove legacy reflection and 10x protocol persistence.
-- Review check-ins now live solely in quarterly_checkins.

DROP TABLE IF EXISTS protocol_checks;
DROP TABLE IF EXISTS monthly_checkins;
DROP TABLE IF EXISTS weekly_checkins;
DROP TABLE IF EXISTS annual_alignment;
