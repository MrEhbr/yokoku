-- How a row settles a library file that already holds its target.
ALTER TABLE import_rows ADD COLUMN resolution TEXT NOT NULL DEFAULT 'unresolved'
    CHECK (resolution IN ('unresolved', 'replace', 'keep-both'));
UPDATE import_rows SET resolution = 'replace' WHERE replace_file = 1;
ALTER TABLE import_rows DROP COLUMN replace_file;
