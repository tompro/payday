-- a single table is used for all events in the cqrs system
CREATE TABLE IF NOT EXISTS events
(
    aggregate_type text                         NOT NULL,
    aggregate_id   text                         NOT NULL,
    sequence       bigint CHECK (sequence >= 0) NOT NULL,
    event_type     text                         NOT NULL,
    event_version  text                         NOT NULL,
    payload        json                         NOT NULL,
    metadata       json                         NOT NULL,
    PRIMARY KEY (aggregate_type, aggregate_id, sequence)
);

-- this table is only needed if snapshotting is employed
CREATE TABLE IF NOT EXISTS snapshots
(
    aggregate_type   text                                 NOT NULL,
    aggregate_id     text                                 NOT NULL,
    last_sequence    bigint CHECK (last_sequence >= 0)    NOT NULL,
    current_snapshot bigint CHECK (current_snapshot >= 0) NOT NULL,
    payload          json                                 NOT NULL,
    PRIMARY KEY (aggregate_type, aggregate_id, last_sequence)
);

-- stores offset of different event streams
CREATE TABLE IF NOT EXISTS offsets
(
    id      text    NOT NULL PRIMARY KEY,
    current_offset  bigint  NOT NULL
);
