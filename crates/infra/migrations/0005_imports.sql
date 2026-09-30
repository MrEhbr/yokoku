CREATE TABLE imports (
    id          TEXT PRIMARY KEY,
    source      TEXT NOT NULL,
    download_id TEXT,
    status      TEXT NOT NULL CHECK (status IN ('needs_review', 'approved', 'importing', 'done', 'failed')),
    error       TEXT,
    created_at  TEXT NOT NULL
) STRICT;

CREATE INDEX imports_by_status ON imports (status, created_at);
CREATE UNIQUE INDEX imports_by_download ON imports (download_id) WHERE download_id IS NOT NULL;

CREATE TABLE import_rows (
    import_id     TEXT    NOT NULL REFERENCES imports (id) ON DELETE CASCADE,
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
