from __future__ import annotations

import argparse
import json
from pathlib import Path

from fruitbox_api.sample_catalog import (
    SampleCatalog,
    SampleCatalogSource,
    initialize_catalog,
    upsert_sample_record,
    upsert_sample_set,
)


def main() -> None:
    parser = argparse.ArgumentParser(description="Manage Fruitbox sample catalog SQLite files.")
    subparsers = parser.add_subparsers(dest="command", required=True)

    init_parser = subparsers.add_parser("init", help="Create or migrate a catalog database.")
    init_parser.add_argument("--db", type=Path, required=True)

    set_parser = subparsers.add_parser("upsert-set", help="Create or update a sample set.")
    set_parser.add_argument("--db", type=Path, required=True)
    set_parser.add_argument("--id", required=True)
    set_parser.add_argument("--label", required=True)
    set_parser.add_argument("--description", default="")
    set_parser.add_argument(
        "--purpose",
        default="test",
        choices=("test", "validation", "training", "reference"),
    )
    set_parser.add_argument("--allow-training", action="store_true")
    set_parser.add_argument("--metadata-json", default="{}")

    record_parser = subparsers.add_parser("upsert-record", help="Create or update a sample record.")
    record_parser.add_argument("--db", type=Path, required=True)
    record_parser.add_argument("--id", required=True)
    record_parser.add_argument("--sample-set-id", required=True)
    record_parser.add_argument("--image-uri", required=True)
    record_parser.add_argument("--image-sha256")
    record_parser.add_argument("--perceptual-hash")
    record_parser.add_argument("--board-signature")
    record_parser.add_argument("--width", type=int)
    record_parser.add_argument("--height", type=int)
    record_parser.add_argument("--captured-at")
    record_parser.add_argument("--metadata-json", default="{}")

    exclusions_parser = subparsers.add_parser(
        "training-exclusions",
        help="Emit JSON rows that training code must ignore exactly or by similarity.",
    )
    exclusions_parser.add_argument("--db", type=Path, required=True)
    exclusions_parser.add_argument("--source-name")
    exclusions_parser.add_argument(
        "--source-kind",
        choices=("provisioned", "environment"),
        default="provisioned",
    )

    args = parser.parse_args()

    if args.command == "init":
        initialize_catalog(args.db)
        return

    if args.command == "upsert-set":
        metadata = _parse_metadata(parser, args.metadata_json)
        upsert_sample_set(
            args.db,
            sample_set_id=args.id,
            label=args.label,
            description=args.description,
            purpose=args.purpose,
            exclude_from_training=not args.allow_training,
            metadata=metadata,
        )
        return

    if args.command == "upsert-record":
        metadata = _parse_metadata(parser, args.metadata_json)
        upsert_sample_record(
            args.db,
            record_id=args.id,
            sample_set_id=args.sample_set_id,
            image_uri=args.image_uri,
            image_sha256=args.image_sha256,
            perceptual_hash=args.perceptual_hash,
            board_signature=args.board_signature,
            width=args.width,
            height=args.height,
            captured_at=args.captured_at,
            metadata=metadata,
        )
        return

    if args.command == "training-exclusions":
        catalog = SampleCatalog(
            [
                SampleCatalogSource(
                    name=args.source_name or args.db.stem,
                    kind=args.source_kind,
                    path=args.db,
                    required=True,
                )
            ]
        )
        print(
            json.dumps(
                [exclusion.model_dump() for exclusion in catalog.list_training_exclusions()],
                indent=2,
                sort_keys=True,
            )
        )


def _parse_metadata(parser: argparse.ArgumentParser, raw_metadata: str) -> dict[str, object]:
    try:
        metadata = json.loads(raw_metadata)
    except json.JSONDecodeError as error:
        parser.error(f"--metadata-json must be valid JSON: {error}")
    if not isinstance(metadata, dict):
        parser.error("--metadata-json must be a JSON object")
    return metadata


if __name__ == "__main__":
    main()
