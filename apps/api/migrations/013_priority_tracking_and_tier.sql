-- Add tracking_status (Lattice trajectory: on_track/progressing/off_track/complete/incomplete/no_update),
-- due_date, and tier (department/team/individual) to priorities.
ALTER TABLE priorities ADD COLUMN tracking_status TEXT;
ALTER TABLE priorities ADD COLUMN due_date TEXT;
ALTER TABLE priorities ADD COLUMN tier TEXT