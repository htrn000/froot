from __future__ import annotations

import json
import os
import sqlite3
from collections.abc import Iterable
from dataclasses import dataclass
from pathlib import Path

from fruitbox_api.config import Settings, get_settings
from fruitbox_api.models import SampleRecord, SampleSet, TrainingExclusion


SCHEMA_SQL = """
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
""".strip()


@dataclass(frozen=True)
class SampleCatalogSource:
    name: str
    kind: str
    path: Path
    required: bool = False


class SampleCatalog:
    def __init__(self, sources: Iterable[SampleCatalogSource]) -> None:
        self._sources = tuple(sources)

    def list_sets(self) -> list[SampleSet]:
        sets: list[SampleSet] = []
        for source in self._existing_sources():
            with _open_readonly(source.path) as connection:
                rows = connection.execute(
                    """
                    SELECT
                        s.id,
                        s.label,
                        s.description,
                        s.purpose,
                        s.exclude_from_training,
                        s.metadata_json,
                        COUNT(r.id) AS sample_count
                    FROM sample_sets AS s
                    LEFT JOIN sample_records AS r ON r.sample_set_id = s.id
                    GROUP BY
                        s.id,
                        s.label,
                        s.description,
                        s.purpose,
                        s.exclude_from_training,
                        s.metadata_json
                    ORDER BY s.id
                    """
                ).fetchall()
            sets.extend(
                SampleSet(
                    id=row["id"],
                    label=row["label"],
                    description=row["description"],
                    source_name=source.name,
                    source_kind=source.kind,
                    purpose=row["purpose"],
                    exclude_from_training=bool(row["exclude_from_training"]),
                    sample_count=row["sample_count"],
                    metadata=_metadata(row["metadata_json"]),
                )
                for row in rows
            )
        return sets

    def list_records(self, sample_set_id: str | None = None) -> list[SampleRecord]:
        records: list[SampleRecord] = []
        for source in self._existing_sources():
            query = """
                SELECT
                    id,
                    sample_set_id,
                    image_uri,
                    image_sha256,
                    perceptual_hash,
                    board_signature,
                    width,
                    height,
                    captured_at,
                    metadata_json
                FROM sample_records
            """
            parameters: tuple[str, ...] = ()
            if sample_set_id is not None:
                query += " WHERE sample_set_id = ?"
                parameters = (sample_set_id,)
            query += " ORDER BY sample_set_id, id"
            with _open_readonly(source.path) as connection:
                rows = connection.execute(query, parameters).fetchall()
            records.extend(
                SampleRecord(
                    id=row["id"],
                    sample_set_id=row["sample_set_id"],
                    source_name=source.name,
                    source_kind=source.kind,
                    image_uri=row["image_uri"],
                    image_sha256=row["image_sha256"],
                    perceptual_hash=row["perceptual_hash"],
                    board_signature=row["board_signature"],
                    width=row["width"],
                    height=row["height"],
                    captured_at=row["captured_at"],
                    metadata=_metadata(row["metadata_json"]),
                )
                for row in rows
            )
        return records

    def list_training_exclusions(self) -> list[TrainingExclusion]:
        exclusions: list[TrainingExclusion] = []
        for source in self._existing_sources():
            with _open_readonly(source.path) as connection:
                rows = connection.execute(
                    """
                    SELECT
                        s.id AS sample_set_id,
                        r.id AS record_id,
                        r.image_sha256,
                        r.perceptual_hash,
                        r.board_signature
                    FROM sample_sets AS s
                    JOIN sample_records AS r ON r.sample_set_id = s.id
                    WHERE s.exclude_from_training = 1
                    ORDER BY s.id, r.id
                    """
                ).fetchall()
            exclusions.extend(
                TrainingExclusion(
                    sample_set_id=row["sample_set_id"],
                    record_id=row["record_id"],
                    source_name=source.name,
                    source_kind=source.kind,
                    image_sha256=row["image_sha256"],
                    perceptual_hash=row["perceptual_hash"],
                    board_signature=row["board_signature"],
                )
                for row in rows
            )
        return exclusions

    def _existing_sources(self) -> Iterable[SampleCatalogSource]:
        for source in self._sources:
            if source.path.exists():
                yield source
            elif source.required:
                raise FileNotFoundError(f"sample catalog source does not exist: {source.path}")


def configured_sample_catalog(settings: Settings | None = None) -> SampleCatalog:
    settings = settings or get_settings()
    sources = [
        SampleCatalogSource(
            name="provisioned",
            kind="provisioned",
            path=Path(settings.sample_catalog_path),
            required=False,
        )
    ]
    for index, overlay_path in enumerate(_split_overlay_paths(settings.sample_catalog_overlays), 1):
        sources.append(
            SampleCatalogSource(
                name=f"environment-{index}",
                kind="environment",
                path=Path(overlay_path),
                required=False,
            )
        )
    return SampleCatalog(sources)


def initialize_catalog(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with sqlite3.connect(path) as connection:
        connection.executescript(SCHEMA_SQL)
        connection.commit()


def upsert_sample_set(
    path: Path,
    *,
    sample_set_id: str,
    label: str,
    description: str = "",
    purpose: str = "test",
    exclude_from_training: bool = True,
    metadata: dict[str, object] | None = None,
) -> None:
    initialize_catalog(path)
    with sqlite3.connect(path) as connection:
        connection.execute(
            """
            INSERT INTO sample_sets (
                id,
                label,
                description,
                purpose,
                exclude_from_training,
                metadata_json,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(id) DO UPDATE SET
                label = excluded.label,
                description = excluded.description,
                purpose = excluded.purpose,
                exclude_from_training = excluded.exclude_from_training,
                metadata_json = excluded.metadata_json,
                updated_at = CURRENT_TIMESTAMP
            """,
            (
                sample_set_id,
                label,
                description,
                purpose,
                int(exclude_from_training),
                _dump_metadata(metadata),
            ),
        )
        connection.commit()


def upsert_sample_record(
    path: Path,
    *,
    record_id: str,
    sample_set_id: str,
    image_uri: str,
    image_sha256: str | None = None,
    perceptual_hash: str | None = None,
    board_signature: str | None = None,
    width: int | None = None,
    height: int | None = None,
    captured_at: str | None = None,
    metadata: dict[str, object] | None = None,
) -> None:
    initialize_catalog(path)
    with sqlite3.connect(path) as connection:
        connection.execute("PRAGMA foreign_keys = ON")
        connection.execute(
            """
            INSERT INTO sample_records (
                id,
                sample_set_id,
                image_uri,
                image_sha256,
                perceptual_hash,
                board_signature,
                width,
                height,
                captured_at,
                metadata_json,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(id) DO UPDATE SET
                sample_set_id = excluded.sample_set_id,
                image_uri = excluded.image_uri,
                image_sha256 = excluded.image_sha256,
                perceptual_hash = excluded.perceptual_hash,
                board_signature = excluded.board_signature,
                width = excluded.width,
                height = excluded.height,
                captured_at = excluded.captured_at,
                metadata_json = excluded.metadata_json,
                updated_at = CURRENT_TIMESTAMP
            """,
            (
                record_id,
                sample_set_id,
                image_uri,
                image_sha256,
                perceptual_hash,
                board_signature,
                width,
                height,
                captured_at,
                _dump_metadata(metadata),
            ),
        )
        connection.commit()


def _open_readonly(path: Path) -> sqlite3.Connection:
    connection = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    connection.row_factory = sqlite3.Row
    return connection


def _split_overlay_paths(raw_paths: str) -> list[str]:
    return [path for path in (part.strip() for part in raw_paths.split(os.pathsep)) if path]


def _metadata(raw_metadata: str) -> dict[str, object]:
    metadata = json.loads(raw_metadata)
    if not isinstance(metadata, dict):
        raise ValueError("sample catalog metadata_json must contain a JSON object")
    return metadata


def _dump_metadata(metadata: dict[str, object] | None) -> str:
    return json.dumps(metadata or {}, sort_keys=True, separators=(",", ":"))
