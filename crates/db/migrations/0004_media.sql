CREATE TABLE root_folders (
    path TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('series', 'movies'))
) STRICT;

CREATE TABLE media_files (
    id            TEXT    PRIMARY KEY,
    path          TEXT    NOT NULL UNIQUE,
    size          INTEGER NOT NULL,
    series_id     TEXT,
    season        INTEGER,
    first_episode INTEGER,
    last_episode  INTEGER,
    movie_id      TEXT,
    added_at      TEXT    NOT NULL,
    CHECK ((movie_id IS NULL) = (series_id IS NOT NULL AND season IS NOT NULL
                                 AND first_episode IS NOT NULL AND last_episode IS NOT NULL))
) STRICT;

CREATE TABLE imports (
    id         TEXT PRIMARY KEY,
    source     TEXT NOT NULL,
    status     TEXT NOT NULL CHECK (status IN ('needs_review', 'done')),
    created_at TEXT NOT NULL
) STRICT;

CREATE INDEX imports_by_status ON imports (status, created_at);

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
    PRIMARY KEY (import_id, position)
) STRICT;
