ALTER TABLE series ADD COLUMN alternate_titles TEXT NOT NULL DEFAULT '[]'; -- JSON array of strings
ALTER TABLE movies ADD COLUMN alternate_titles TEXT NOT NULL DEFAULT '[]'; -- JSON array of strings
