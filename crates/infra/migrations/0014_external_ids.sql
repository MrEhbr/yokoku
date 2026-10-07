-- An item's ids at other sites as one JSON object, {"imdb": "tt0903747"}; ids its metadata source
-- doesn't know are left out. Every item is made due for a metadata refresh, which fills them.
ALTER TABLE series ADD COLUMN external_ids TEXT NOT NULL DEFAULT '{}';
ALTER TABLE movies ADD COLUMN external_ids TEXT NOT NULL DEFAULT '{}';
UPDATE series SET refreshed_at = '1970-01-01T00:00:00Z';
UPDATE movies SET refreshed_at = '1970-01-01T00:00:00Z';
