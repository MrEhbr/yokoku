CREATE TABLE downloads (
    id            TEXT    PRIMARY KEY,
    hash          TEXT    NOT NULL UNIQUE,
    name          TEXT    NOT NULL,
    series_id     TEXT,
    movie_id      TEXT,
    state         TEXT    NOT NULL
        CHECK (state IN ('queued', 'checking', 'downloading', 'seeding', 'stopped', 'removed')),
    size          INTEGER NOT NULL,
    done          INTEGER NOT NULL,
    download_rate INTEGER NOT NULL,
    eta           INTEGER,
    download_dir  TEXT    NOT NULL,
    error         TEXT,
    added_at      TEXT    NOT NULL,
    completed_at  TEXT,
    CHECK (series_id IS NULL OR movie_id IS NULL)
) STRICT;
