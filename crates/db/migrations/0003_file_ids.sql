ALTER TABLE episodes ADD COLUMN file_id TEXT;
ALTER TABLE episodes DROP COLUMN has_file;

ALTER TABLE movies ADD COLUMN file_id TEXT;
ALTER TABLE movies DROP COLUMN has_file;
