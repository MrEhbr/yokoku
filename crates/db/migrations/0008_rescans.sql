CREATE TABLE media_server_rescan (
    id           INTEGER PRIMARY KEY CHECK (id = 1),
    requested_at INTEGER NOT NULL -- milliseconds since the Unix epoch
) STRICT;
