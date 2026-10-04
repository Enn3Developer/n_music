-- What the user creates. Tracks are referenced by locator, not by row id: the references must
-- outlive the scanned rows, which are deleted while a file is missing. The fingerprint finds a
-- moved or renamed file again.

ALTER TABLE tracks ADD COLUMN fingerprint INTEGER;

CREATE INDEX tracks_fingerprint ON tracks (fingerprint);

CREATE TABLE playlists (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    -- JSON filter of a smart playlist; NULL for one of explicitly added tracks.
    rule TEXT,
    -- JSON sort keys.
    sort TEXT NOT NULL,
    -- Unix seconds.
    created INTEGER NOT NULL,
    modified INTEGER NOT NULL
);

CREATE TABLE playlist_items (
    playlist_id INTEGER NOT NULL REFERENCES playlists (id) ON DELETE CASCADE,
    kind INTEGER NOT NULL,
    location TEXT NOT NULL,
    name TEXT,
    fingerprint INTEGER,
    -- Unix seconds.
    added INTEGER NOT NULL,
    PRIMARY KEY (playlist_id, location)
) WITHOUT ROWID;

CREATE TABLE play_stats (
    location TEXT PRIMARY KEY,
    kind INTEGER NOT NULL,
    name TEXT,
    fingerprint INTEGER,
    plays INTEGER NOT NULL,
    -- Unix seconds.
    last_played INTEGER NOT NULL
) WITHOUT ROWID;

-- The play session, restored on launch: one row.
CREATE TABLE session (
    id INTEGER PRIMARY KEY CHECK (id = 0),
    -- JSON query the context was taken from; NULL without one.
    context TEXT,
    -- The current item: its list and position there, see `session_items`.
    current_list INTEGER,
    current_position INTEGER,
    -- Seconds into the current item.
    position REAL NOT NULL DEFAULT 0,
    -- The context ran out: playing again starts over.
    finished INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE session_items (
    -- 0: context, 1: up next, 2: the up-next item playing now.
    list INTEGER NOT NULL,
    position INTEGER NOT NULL,
    -- Context items: their place in the play order.
    slot INTEGER,
    kind INTEGER NOT NULL,
    location TEXT NOT NULL,
    name TEXT,
    fingerprint INTEGER,
    PRIMARY KEY (list, position)
) WITHOUT ROWID;
