# Provisioned Fruitbox sample catalog

`fruitbox_samples.sqlite3` is the in-repo SQLite catalog for curated Fruitbox
game captures. It starts with the schema and a provisioned sample set, then grows
with real captures as they are approved for repeatable evaluation.

Store image files under `data/provisioned/images/` and reference them by relative
path from `sample_records.image_uri`. Do not store deployment-only captures here;
put those in an environment SQLite overlay configured with
`FRUITBOX_SAMPLE_CATALOG_OVERLAYS`.

Rows in provisioned test sets default to `exclude_from_training = 1`. RL and
supervised training jobs must query `/api/v1/samples/training-exclusions` or run
`python -m fruitbox_api.sample_catalog_cli training-exclusions --db <catalog>` and
drop exact `image_sha256` matches plus near matches keyed by `perceptual_hash` or
`board_signature`.
