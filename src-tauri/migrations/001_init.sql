CREATE TABLE source_root (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    path          TEXT NOT NULL UNIQUE,
    kind          TEXT NOT NULL,
    scan_policy   TEXT NOT NULL,
    poll_secs     INTEGER,
    last_scan_at  INTEGER,
    status        TEXT NOT NULL DEFAULT 'idle',
    smb_host      TEXT,
    smb_share     TEXT,
    smb_username  TEXT,
    smb_mounted   INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE asset (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    root_id          INTEGER NOT NULL REFERENCES source_root(id) ON DELETE CASCADE,
    rel_path         TEXT NOT NULL,
    file_name        TEXT NOT NULL,
    ext              TEXT NOT NULL,
    kind             TEXT NOT NULL,
    size             INTEGER NOT NULL,
    mtime_ns         INTEGER NOT NULL,
    content_hash     TEXT,
    thumb_key        TEXT,
    sync_state       TEXT NOT NULL DEFAULT 'new',
    deleted_at       INTEGER,
    has_duplicate    INTEGER NOT NULL DEFAULT 0,
    indexed_mtime_ns INTEGER,
    UNIQUE(root_id, rel_path)
);

CREATE TABLE asset_meta (
    asset_id      INTEGER PRIMARY KEY REFERENCES asset(id) ON DELETE CASCADE,
    capture_at    INTEGER,
    camera        TEXT,
    lens          TEXT,
    rating        INTEGER,
    latitude      REAL,
    longitude     REAL,
    keywords_json TEXT
);

CREATE TABLE tag (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    name      TEXT NOT NULL UNIQUE,
    parent_id INTEGER REFERENCES tag(id),
    color     TEXT
);

CREATE TABLE asset_tag (
    asset_id INTEGER NOT NULL REFERENCES asset(id) ON DELETE CASCADE,
    tag_id   INTEGER NOT NULL REFERENCES tag(id) ON DELETE CASCADE,
    PRIMARY KEY (asset_id, tag_id)
);

CREATE TABLE asset_raw_tag (
    asset_id INTEGER NOT NULL REFERENCES asset(id) ON DELETE CASCADE,
    name     TEXT NOT NULL,
    value    TEXT NOT NULL
);

CREATE TABLE export_job (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    status        TEXT NOT NULL,
    manifest_json TEXT,
    created_at    INTEGER NOT NULL
);

CREATE TABLE smart_collection (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL UNIQUE,
    filter_json TEXT NOT NULL
);

CREATE TABLE album (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    name      TEXT NOT NULL,
    sort_mode TEXT NOT NULL DEFAULT 'date:desc',
    emoji     TEXT
);

CREATE TABLE album_item (
    album_id  INTEGER NOT NULL REFERENCES album(id) ON DELETE CASCADE,
    asset_id  INTEGER NOT NULL REFERENCES asset(id) ON DELETE CASCADE,
    position  INTEGER NOT NULL,
    PRIMARY KEY (album_id, asset_id)
);

CREATE TABLE asset_link (
    src_id INTEGER NOT NULL REFERENCES asset(id) ON DELETE CASCADE,
    dst_id INTEGER NOT NULL REFERENCES asset(id) ON DELETE CASCADE,
    kind   TEXT NOT NULL,
    PRIMARY KEY (src_id, dst_id, kind)
);

CREATE TABLE activity_log (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    seq             INTEGER NOT NULL,
    occurred_at     INTEGER NOT NULL,
    event_type      TEXT NOT NULL,
    actor           TEXT NOT NULL,
    correlation_id  TEXT,
    subject_type    TEXT,
    subject_id      INTEGER,
    subject_key     TEXT,
    summary         TEXT,
    payload_json    TEXT NOT NULL,
    revert_json     TEXT,
    undone_at       INTEGER,
    undone_by_id    INTEGER REFERENCES activity_log(id)
);

CREATE INDEX idx_asset_meta_capture ON asset_meta(capture_at);
CREATE INDEX idx_asset_meta_rating ON asset_meta(rating);
CREATE INDEX idx_asset_sync ON asset(sync_state) WHERE deleted_at IS NULL;
CREATE INDEX idx_asset_root ON asset(root_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_asset_hash ON asset(content_hash) WHERE content_hash IS NOT NULL AND deleted_at IS NULL;
CREATE INDEX idx_asset_has_duplicate ON asset(has_duplicate)
    WHERE deleted_at IS NULL AND has_duplicate = 1;
CREATE INDEX idx_asset_raw_tag_asset ON asset_raw_tag(asset_id);
CREATE INDEX idx_asset_raw_tag_name ON asset_raw_tag(name);
CREATE INDEX idx_asset_raw_tag_value ON asset_raw_tag(value);
CREATE UNIQUE INDEX idx_activity_seq ON activity_log(seq);
CREATE INDEX idx_activity_subject ON activity_log(subject_type, subject_id, occurred_at DESC);
CREATE INDEX idx_activity_subject_key ON activity_log(subject_key, occurred_at DESC);
CREATE INDEX idx_activity_type ON activity_log(event_type, occurred_at DESC);
CREATE INDEX idx_activity_correlation ON activity_log(correlation_id);
