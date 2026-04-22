-- 021_remove_focus_and_todos
-- Drops the Weekly Focus planning tables and the standalone Todos table.
-- Weekly focus was replaced by an AI-generated Last Week summary on the
-- dashboard. Todos was replaced by an inline Latest Updates section.

DROP TABLE IF EXISTS weekly_focus_entries;
DROP TABLE IF EXISTS weekly_focus;
DROP TABLE IF EXISTS todos;
