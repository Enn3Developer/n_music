-- Which tracks each library listed when it was last listed: a track stays while a library lists
-- it. A library has a row once it could be listed, so what one without lists is unknown. Tracks
-- are referenced by locator, not by row id: a library lists them before they are read, and
-- those without a version are never stored.

CREATE TABLE libraries (
    id INTEGER PRIMARY KEY,
    -- 0: local folder, 2: web playlist, 3: Android document tree.
    kind INTEGER NOT NULL,
    location TEXT NOT NULL UNIQUE
);

CREATE TABLE library_tracks (
    library_id INTEGER NOT NULL REFERENCES libraries (id) ON DELETE CASCADE,
    -- As in `tracks`.
    kind INTEGER NOT NULL,
    location TEXT NOT NULL,
    name TEXT,
    PRIMARY KEY (library_id, location)
) WITHOUT ROWID;

CREATE INDEX library_tracks_location ON library_tracks (location);
