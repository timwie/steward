-- added by 0.1.0

CREATE TABLE steward.player (
    login        TEXT,
    display_name TEXT    NOT NULL,

    PRIMARY KEY (login)
);

CREATE TABLE steward.map (
    uid                 TEXT,
    file_name           TEXT      NOT NULL,     -- relative path in /UserData/Maps/
    name                TEXT      NOT NULL,
    author_login        TEXT      NOT NULL,     -- in TMNext this is an ID
    author_display_name TEXT      NOT NULL,     -- in TMNext this is the UPlay username
    author_millis       INTEGER   NOT NULL,
    added_since         TIMESTAMP NOT NULL,
    exchange_id         INTEGER   DEFAULT NULL, -- for maps imported from trackmania.exchange

    PRIMARY KEY (uid),
    UNIQUE (file_name)
);

CREATE TABLE steward.map_file (
    map_uid TEXT,
    file    BYTEA NOT NULL,

    PRIMARY KEY (map_uid),
    FOREIGN KEY (map_uid) REFERENCES steward.map (uid)
);

CREATE TABLE steward.record (
    player_login  TEXT,
    map_uid       TEXT,
    millis        INTEGER   NOT NULL,
    timestamp     TIMESTAMP NOT NULL,
    nb_laps       INTEGER   NOT NULL, -- use '0' if not multi-lap or for flying laps

    PRIMARY KEY (player_login, map_uid, nb_laps),
    FOREIGN KEY (player_login) REFERENCES steward.player (login),
    FOREIGN KEY (map_uid)      REFERENCES steward.map (uid),

    CONSTRAINT nb_laps_positive CHECK (nb_laps >= 0)
);

CREATE TABLE steward.ta_history (
    player_login TEXT,
    map_uid      TEXT,
    last_played  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    PRIMARY KEY (player_login, map_uid),
    FOREIGN KEY (player_login) REFERENCES steward.player (login),
    FOREIGN KEY (map_uid)      REFERENCES steward.map (uid)
);

CREATE TYPE steward.Pref AS ENUM (
    'Pick',
    'Veto',
    'Remove'
);

CREATE TABLE steward.ta_preference (
    player_login TEXT,
    map_uid      TEXT,
    value        steward.Pref DEFAULT NULL,

    PRIMARY KEY (player_login, map_uid),
    FOREIGN KEY (player_login) REFERENCES steward.player (login),
    FOREIGN KEY (map_uid)      REFERENCES steward.map (uid)
);

UPDATE steward.meta SET at_migration = 1;
