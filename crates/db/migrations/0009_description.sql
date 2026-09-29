-- An item's description as one JSON object, {"overview": …, "genres": […], "runtime": minutes};
-- missing values are left out. Items get theirs on their next refresh.
ALTER TABLE series ADD COLUMN description TEXT NOT NULL DEFAULT '{}';
ALTER TABLE movies ADD COLUMN description TEXT NOT NULL DEFAULT '{}';
ALTER TABLE episodes ADD COLUMN overview TEXT NOT NULL DEFAULT '';
