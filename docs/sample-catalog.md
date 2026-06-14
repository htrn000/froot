# Fruitbox sample catalog

The sample catalog is the first datasource for captured Fruitbox games as
images. It supports two tiers:

1. **Provisioned**: reviewed rows committed in `data/provisioned/`.
2. **Environment**: deployment or experiment rows stored outside git and enabled
   with `FRUITBOX_SAMPLE_CATALOG_OVERLAYS`.

FastAPI reads both tiers and returns a merged view.

## Runtime configuration

```bash
FRUITBOX_SAMPLE_CATALOG_PATH=data/provisioned/fruitbox_samples.sqlite3
FRUITBOX_SAMPLE_CATALOG_OVERLAYS=/var/lib/fruitbox/samples.sqlite3
```

Use the platform path separator to provide multiple overlays. Provisioned data is
read first; overlays are read after it and marked as `source_kind=environment`.

## Provisioning hooks

Create or migrate a catalog:

```bash
uv run python -m fruitbox_api.sample_catalog_cli init \
  --db data/provisioned/fruitbox_samples.sqlite3
```

Add a curated test set:

```bash
uv run python -m fruitbox_api.sample_catalog_cli upsert-set \
  --db data/provisioned/fruitbox_samples.sqlite3 \
  --id fruitbox-provisioned-v1 \
  --label "Provisioned Fruitbox captures" \
  --purpose test
```

Add a captured image row:

```bash
uv run python -m fruitbox_api.sample_catalog_cli upsert-record \
  --db data/provisioned/fruitbox_samples.sqlite3 \
  --sample-set-id fruitbox-provisioned-v1 \
  --id capture-0001 \
  --image-uri data/provisioned/images/capture-0001.png \
  --image-sha256 <sha256> \
  --perceptual-hash <phash> \
  --board-signature <canonical-board-signature>
```

Every record must include at least one of `image_sha256`, `perceptual_hash`, or
`board_signature`. Prefer all three for test-set protection.

## Query hooks

- `GET /api/v1/samples/sets`: list sample sets from provisioned and environment
  catalogs.
- `GET /api/v1/samples/sets/{sample_set_id}/records`: list records in one set.
- `GET /api/v1/samples/training-exclusions`: list exact and similarity keys that
  training jobs must ignore.

Agents can use the API for live deployments or use the CLI directly against a
SQLite file during local data-prep work.

## RL and training policy

Rows in evaluation/test sets must keep `exclude_from_training = 1`. Training
code must remove:

- exact image duplicates by `image_sha256`;
- visually similar captures by `perceptual_hash`;
- equivalent board states by `board_signature`.

Environment overlays may contain private or impure deployment captures. They are
never committed to git, but the same exclusion endpoint includes them so deployed
training and evaluation jobs can apply the same policy.
