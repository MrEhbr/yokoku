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

CREATE TABLE media_info (
    file_id     TEXT    PRIMARY KEY REFERENCES media_files (id) ON DELETE CASCADE,
    duration_ms INTEGER,
    video_codec TEXT,
    width       INTEGER,
    height      INTEGER,
    CHECK ((video_codec IS NULL) = (width IS NULL) AND (width IS NULL) = (height IS NULL))
) STRICT;

CREATE TABLE media_streams (
    file_id  TEXT    NOT NULL REFERENCES media_info (file_id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    kind     TEXT    NOT NULL CHECK (kind IN ('audio', 'subtitle')),
    codec    TEXT    NOT NULL,
    language TEXT,
    channels INTEGER CHECK ((kind = 'audio') = (channels IS NOT NULL)),
    forced   INTEGER NOT NULL,
    PRIMARY KEY (file_id, position)
) STRICT;

CREATE TABLE media_server_rescan (
    id           INTEGER PRIMARY KEY CHECK (id = 1),
    requested_at INTEGER NOT NULL -- milliseconds since the Unix epoch
) STRICT;
