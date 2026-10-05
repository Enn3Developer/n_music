-- What the user calls each library; NULL goes by its folder's or playlist's name. A library can
-- be named before it could be listed: its row then links no tracks, and `listed` is 0 until it
-- is listed.

ALTER TABLE libraries ADD COLUMN display_name TEXT;

ALTER TABLE libraries ADD COLUMN listed INTEGER NOT NULL DEFAULT 1;
