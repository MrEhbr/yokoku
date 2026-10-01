-- Library files the media server's user has played, as of the last watched sync.
CREATE TABLE watched_files (
    file_id    TEXT PRIMARY KEY REFERENCES media_files (id) ON DELETE CASCADE,
    watched_at TEXT -- last played; NULL when the media server gives no date
) STRICT;
