-- added by 0.1.0

-- There is a possible inconsistency between the 'record' and 'sector' tables,
-- where the last sector's 'millis' is not equal to 'record.millis', which must be true,
-- since the time at the finish line must be equal to the overall record time.
-- We put up with that, because it simplifies record queries, and because it is an
-- unlikely mistake, since that data is produced by the game itself.

-- Validation & ghost replays are stored in the 'record' table, one of the reasons being
-- that Dedimania requires validation replays for all records, and ghost replays
-- for top 1 records. GRs are great because you can actually watch and play against a run.

CREATE TABLE steward.player (
    login     TEXT,
    nick_name TEXT    NOT NULL,

    PRIMARY KEY (login)
);

CREATE TABLE steward.map (
    uid             TEXT,
    file_name       TEXT      NOT NULL,     -- relative path in /UserData/Maps/
    file            BYTEA     NOT NULL,
    name            TEXT      NOT NULL,
    author_login    TEXT      NOT NULL,
    added_since     TIMESTAMP NOT NULL,
    in_playlist     BOOLEAN   NOT NULL DEFAULT true,
    exchange_id     INTEGER   DEFAULT NULL, -- for maps imported from trackmania.exchange

    PRIMARY KEY (uid),
    UNIQUE (file_name)
);

CREATE TYPE steward.Pref AS ENUM (
    'Pick',
    'Veto',
    'Remove'
);

CREATE TABLE steward.preference (
    player_login TEXT,
    map_uid      TEXT,
    value        steward.Pref DEFAULT NULL,

    PRIMARY KEY (player_login, map_uid),
    FOREIGN KEY (player_login) REFERENCES steward.player (login),
    FOREIGN KEY (map_uid)      REFERENCES steward.map (uid)
);

CREATE TABLE steward.record (
    player_login  TEXT,
    map_uid       TEXT,
    millis        INTEGER   NOT NULL,
    validation    BYTEA     NOT NULL, -- validation replays (~5 - 50 KB)
    ghost         BYTEA,              -- ghost replays (~250 KB - 2 MB)
    timestamp     TIMESTAMP NOT NULL,

    PRIMARY KEY (player_login, map_uid),
    FOREIGN KEY (player_login) REFERENCES steward.player (login),
    FOREIGN KEY (map_uid)      REFERENCES steward.map (uid)
);

CREATE TABLE steward.sector (
    player_login  TEXT,
    map_uid       TEXT,
    index         INTEGER NOT NULL, -- first checkpoint has index 0; finish is at the last index
    cp_millis     INTEGER NOT NULL, -- total millis at time of crossing checkpoint
    cp_speed      REAL    NOT NULL, -- speed at time of crossing checkpoint
    cp_distance   REAL    NOT NULL, -- total driven distance at time of crossing checkpoint

    PRIMARY KEY (player_login, map_uid, index),
    FOREIGN KEY (player_login, map_uid) REFERENCES steward.record (player_login, map_uid),

    CONSTRAINT index_positive    check (index >= 0),
    CONSTRAINT millis_positive   check (cp_millis > 0),
    CONSTRAINT speed_positive    check (cp_speed > 0),
    CONSTRAINT distance_positive check (cp_distance > 0)
);

UPDATE steward.meta SET at_migration = 1;
