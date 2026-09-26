CREATE TABLE settings (
    key   TEXT PRIMARY KEY, -- dotted path such as `import.mode`
    value TEXT NOT NULL     -- JSON
) STRICT;
