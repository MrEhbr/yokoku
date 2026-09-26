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
