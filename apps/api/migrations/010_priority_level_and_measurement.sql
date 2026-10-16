ALTER TABLE priorities ADD COLUMN priority_level TEXT;
ALTER TABLE priorities ADD COLUMN measure_type TEXT;
ALTER TABLE priorities ADD COLUMN measure_start REAL;
ALTER TABLE priorities ADD COLUMN measure_target REAL;
ALTER TABLE priorities ADD COLUMN measure_current REAL;
ALTER TABLE priorities ADD COLUMN description BLOB;
