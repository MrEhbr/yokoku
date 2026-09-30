-- An item's images as one JSON object, {"poster": …, "backdrop": …, "logo": …}, each a TMDB path
-- or a TVDB URL; kinds without an image are left out.
ALTER TABLE series ADD COLUMN artwork TEXT NOT NULL DEFAULT '{}';
UPDATE series SET artwork = json_object('poster', poster_path) WHERE poster_path IS NOT NULL;
ALTER TABLE series DROP COLUMN poster_path;

ALTER TABLE movies ADD COLUMN artwork TEXT NOT NULL DEFAULT '{}';
UPDATE movies SET artwork = json_object('poster', poster_path) WHERE poster_path IS NOT NULL;
ALTER TABLE movies DROP COLUMN poster_path;
