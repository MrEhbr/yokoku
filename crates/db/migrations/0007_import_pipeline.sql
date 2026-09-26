CREATE TABLE imports_next (
    id          TEXT PRIMARY KEY,
    source      TEXT NOT NULL,
    download_id TEXT,
    status      TEXT NOT NULL CHECK (status IN ('needs_review', 'approved', 'importing', 'done', 'failed')),
    error       TEXT,
    created_at  TEXT NOT NULL
) STRICT;

INSERT INTO imports_next (id, source, status, created_at) SELECT id, source, status, created_at FROM imports;

CREATE TABLE import_rows_next (
    import_id     TEXT    NOT NULL REFERENCES imports_next (id) ON DELETE CASCADE,
    position      INTEGER NOT NULL,
    path          TEXT    NOT NULL,
    size          INTEGER NOT NULL,
    series_id     TEXT,
    season        INTEGER,
    first_episode INTEGER,
    last_episode  INTEGER,
    movie_id      TEXT,
    confidence    TEXT    NOT NULL CHECK (confidence IN ('unknown', 'guess', 'certain')),
    skipped       INTEGER NOT NULL,
    replace_file  INTEGER NOT NULL,
    PRIMARY KEY (import_id, position)
) STRICT;

INSERT INTO import_rows_next
SELECT import_id, position, path, size, series_id, season, first_episode, last_episode, movie_id, confidence, skipped, 0
FROM import_rows;

DROP TABLE import_rows;
DROP TABLE imports;
ALTER TABLE imports_next RENAME TO imports;
ALTER TABLE import_rows_next RENAME TO import_rows;

CREATE INDEX imports_by_status ON imports (status, created_at);
CREATE UNIQUE INDEX imports_by_download ON imports (download_id) WHERE download_id IS NOT NULL;
