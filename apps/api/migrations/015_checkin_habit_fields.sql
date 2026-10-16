-- Add habit tracker fields to weekly check-ins for 10x engineer habits.
ALTER TABLE weekly_checkins ADD COLUMN protected_time INTEGER;
ALTER TABLE weekly_checkins ADD COLUMN curiosity_conversation BLOB;
ALTER TABLE weekly_checkins ADD COLUMN communicated_blockers INTEGER DEFAULT 0;
ALTER TABLE weekly_checkins ADD COLUMN end_to_end_ownership INTEGER DEFAULT 0;
