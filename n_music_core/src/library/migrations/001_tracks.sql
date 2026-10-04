-- What scanning reads from the files. Every row can be rebuilt by rescanning.
CREATE TABLE tracks (
    id INTEGER PRIMARY KEY,
    -- 0: local path, 1: Android document (with its display name in `name`).
    kind INTEGER NOT NULL,
    location TEXT NOT NULL UNIQUE,
    name TEXT,
    -- Provider version stamp (size + modification time) the row was read at.
    version INTEGER NOT NULL,
    -- Version of the metadata reader that produced the row.
    format INTEGER NOT NULL,
    -- 0 when the file could not be read; the other columns are then empty.
    readable INTEGER NOT NULL,
    title TEXT,
    album TEXT,
    album_artist TEXT,
    track_number INTEGER,
    track_total INTEGER,
    disc_number INTEGER,
    disc_total INTEGER,
    year INTEGER,
    length REAL NOT NULL DEFAULT 0,
    codec TEXT,
    sample_rate INTEGER,
    channels INTEGER,
    bits_per_sample INTEGER,
    track_gain REAL,
    track_peak REAL,
    album_gain REAL,
    album_peak REAL,
    -- File name in the cover store.
    cover TEXT
);

CREATE INDEX tracks_album ON tracks (album_artist, album);

CREATE TABLE track_artists (
    track_id INTEGER NOT NULL REFERENCES tracks (id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    name TEXT NOT NULL,
    PRIMARY KEY (track_id, position)
) WITHOUT ROWID;

CREATE INDEX track_artists_name ON track_artists (name);

CREATE TABLE track_genres (
    track_id INTEGER NOT NULL REFERENCES tracks (id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    name TEXT NOT NULL,
    PRIMARY KEY (track_id, position)
) WITHOUT ROWID;

CREATE INDEX track_genres_name ON track_genres (name);
