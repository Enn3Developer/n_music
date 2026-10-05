-- The play session, kept between launches while resuming is on: one row.
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
