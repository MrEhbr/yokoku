CREATE TABLE series (
    id             TEXT    PRIMARY KEY,
    source_kind    TEXT    NOT NULL CHECK (source_kind IN ('tmdb', 'tvdb')),
    source_id      INTEGER NOT NULL,
    title          TEXT    NOT NULL,
    original_title TEXT    NOT NULL,
    year           INTEGER,
    poster_path    TEXT,
    source_status  TEXT    NOT NULL,
    numbering      TEXT    NOT NULL,
    monitored      INTEGER NOT NULL,
    added_at       TEXT    NOT NULL,
    refreshed_at   TEXT    NOT NULL,
    UNIQUE (source_kind, source_id)
) STRICT;

CREATE TABLE seasons (
    series_id TEXT    NOT NULL REFERENCES series (id) ON DELETE CASCADE,
    number    INTEGER NOT NULL,
    monitored INTEGER NOT NULL,
    PRIMARY KEY (series_id, number)
) STRICT;

CREATE TABLE episodes (
    id            TEXT    PRIMARY KEY,
    series_id     TEXT    NOT NULL,
    season_number INTEGER NOT NULL,
    source_id     INTEGER NOT NULL,
    number        INTEGER NOT NULL,
    title         TEXT    NOT NULL,
    air_date      TEXT,
    monitored     INTEGER NOT NULL,
    has_file      INTEGER NOT NULL,
    FOREIGN KEY (series_id, season_number) REFERENCES seasons (series_id, number) ON DELETE CASCADE,
    UNIQUE (series_id, source_id)
) STRICT;

CREATE TABLE movies (
    id             TEXT    PRIMARY KEY,
    source_kind    TEXT    NOT NULL CHECK (source_kind IN ('tmdb', 'tvdb')),
    source_id      INTEGER NOT NULL,
    title          TEXT    NOT NULL,
    original_title TEXT    NOT NULL,
    year           INTEGER,
    poster_path    TEXT,
    cinema_date    TEXT,
    digital_date   TEXT,
    physical_date  TEXT,
    monitored      INTEGER NOT NULL,
    has_file       INTEGER NOT NULL,
    added_at       TEXT    NOT NULL,
    refreshed_at   TEXT    NOT NULL,
    UNIQUE (source_kind, source_id)
) STRICT;
