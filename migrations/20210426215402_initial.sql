-- Foreign keys might not work
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS user
(
    id          INTEGER PRIMARY KEY NOT NULL UNIQUE,
    username    TEXT                NOT NULL
);

CREATE TABLE IF NOT EXISTS connection
(
    id          INTEGER PRIMARY KEY NOT NULL UNIQUE,
    user_id     INTEGER             NOT NULL,
    channel     TEXT                NOT NULL,
    platform    TEXT                NOT NULL,
    FOREIGN KEY (user_id) REFERENCES user (id)
        ON UPDATE CASCADE ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS channel_admin
(
    connection_id    INTEGER NOT NULL,
    username         TEXT    NOT NULL,
    PRIMARY KEY (connection_id, username),
    FOREIGN KEY (connection_id) REFERENCES connection (id)
        ON UPDATE CASCADE ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS chat_settings
(
    id                        INTEGER PRIMARY KEY NOT NULL UNIQUE,
    user_id                   INTEGER             NOT NULL,
    enable_public_commands    INTEGER             NOT NULL DEFAULT 0,
    enable_mod_commands       INTEGER             NOT NULL DEFAULT 0,
    prefix                    TEXT                NOT NULL DEFAULT "!",
    language                  TEXT                NOT NULL DEFAULT "en",
    FOREIGN KEY (user_id) REFERENCES user (id)
        ON UPDATE CASCADE ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS stream_server
(
    id                 INTEGER PRIMARY KEY NOT NULL UNIQUE,
    user_id            INTEGER             NOT NULL,
    server             TEXT                NOT NULL,
    stats_url          TEXT                NOT NULL,
    application        TEXT                NOT NULL,
    key                TEXT,
    udp_listener_id    TEXT,
    FOREIGN KEY (user_id) REFERENCES user (id)
        ON UPDATE CASCADE ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS broadcasting_software
(
    id          INTEGER PRIMARY KEY NOT NULL UNIQUE,
    user_id     INTEGER             NOT NULL UNIQUE,
    host        TEXT                NOT NULL,
    port        INTEGER             NOT NULL DEFAULT 4444,
    password    TEXT,
    FOREIGN KEY (user_id) REFERENCES user (id)
        ON UPDATE CASCADE ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS switching_scenes
(
    id         INTEGER PRIMARY KEY NOT NULL UNIQUE,
    user_id    INTEGER             NOT NULL UNIQUE,
    normal     TEXT                NOT NULL DEFAULT "live",
    low        TEXT                NOT NULL DEFAULT "low",
    offline    TEXT                NOT NULL DEFAULT "offline",
    FOREIGN KEY (user_id) REFERENCES user (id)
        ON UPDATE CASCADE ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS switcher_state
(
    id                            INTEGER PRIMARY KEY NOT NULL UNIQUE,
    user_id                       INTEGER             NOT NULL UNIQUE,
    request_interval              INTEGER             NOT NULL DEFAULT 2,
    bitrate_switcher_enabled      INTEGER             NOT NULL DEFAULT 1,
    only_switch_when_streaming    INTEGER             NOT NULL DEFAULT 1,
    auto_switch_notification      INTEGER             NOT NULL DEFAULT 1,
    FOREIGN KEY (user_id) REFERENCES user (id)
        ON UPDATE CASCADE ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS triggers
(
    id         INTEGER PRIMARY KEY NOT NULL UNIQUE,
    user_id    INTEGER             NOT NULL UNIQUE,
    low        INTEGER             DEFAULT 800,
    rtt        INTEGER             DEFAULT 2500,
    offline    INTEGER             DEFAULT NULL,
    FOREIGN KEY (user_id) REFERENCES user (id)
        ON UPDATE CASCADE ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS command_permission
(
    user_id       INTEGER    NOT NULL,
    command       TEXT       NOT NULL,
    permission    TEXT       NOT NULL,
    PRIMARY KEY (user_id, command),
    FOREIGN KEY (user_id) REFERENCES user (id)
        ON UPDATE CASCADE ON DELETE CASCADE
);
