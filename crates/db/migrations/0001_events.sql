CREATE TABLE events (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    payload     TEXT    NOT NULL CHECK (json_valid(payload)),
    occurred_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;

CREATE TABLE subscriber_positions (
    subscriber    TEXT    PRIMARY KEY,
    last_event_id INTEGER NOT NULL REFERENCES events (id)
) STRICT;

CREATE TABLE failed_deliveries (
    subscriber TEXT    NOT NULL,
    event_id   INTEGER NOT NULL REFERENCES events (id),
    error      TEXT    NOT NULL,
    attempts   INTEGER NOT NULL,
    failed_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (subscriber, event_id)
) STRICT;
