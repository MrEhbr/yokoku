-- An item's ratings as one JSON object by source, {"imdb": {"value": 7.8, "votes": 1200000}}, each on
-- the source's scale; a count the source doesn't give is left out.
ALTER TABLE series ADD COLUMN ratings TEXT NOT NULL DEFAULT '{}';
ALTER TABLE movies ADD COLUMN ratings TEXT NOT NULL DEFAULT '{}';
