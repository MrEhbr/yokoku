ALTER TABLE downloads ADD COLUMN imported_at TEXT;

UPDATE downloads SET imported_at = completed_at
WHERE EXISTS (SELECT 1 FROM imports WHERE imports.download_id = downloads.id AND imports.status = 'done');
