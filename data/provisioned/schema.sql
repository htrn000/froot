PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS sample_sets (
    id TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    purpose TEXT NOT NULL CHECK (purpose IN ('test', 'validation', 'training', 'reference')),
    exclude_from_training INTEGER NOT NULL DEFAULT 1 CHECK (exclude_from_training IN (0, 1)),
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS sample_records (
    id TEXT PRIMARY KEY,
    sample_set_id TEXT NOT NULL REFERENCES sample_sets(id) ON DELETE CASCADE,
    image_uri TEXT NOT NULL,
    image_sha256 TEXT,
    perceptual_hash TEXT,
    board_signature TEXT,
    width INTEGER CHECK (width IS NULL OR width > 0),
    height INTEGER CHECK (height IS NULL OR height > 0),
    captured_at TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (
        image_sha256 IS NOT NULL
        OR perceptual_hash IS NOT NULL
        OR board_signature IS NOT NULL
    )
);

CREATE INDEX IF NOT EXISTS idx_sample_records_set_id
    ON sample_records(sample_set_id);
CREATE INDEX IF NOT EXISTS idx_sample_records_image_sha256
    ON sample_records(image_sha256);
CREATE INDEX IF NOT EXISTS idx_sample_records_perceptual_hash
    ON sample_records(perceptual_hash);
CREATE INDEX IF NOT EXISTS idx_sample_records_board_signature
    ON sample_records(board_signature);
