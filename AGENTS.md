# Agent setup notes

This repo uses Python 3.12, uv, maturin/PyO3, Rust/Cargo, FastAPI, and Docker
Compose.

## Cursor Cloud specific instructions

- The per-session update script already runs `pip install --user uv maturin`
  and `uv sync`, so dependencies and the Rust extension are prebuilt on startup.
- `uv`/`maturin` live in `~/.local/bin`. That directory is added to `PATH` via
  `~/.bashrc`, so it is available in fresh interactive shells without re-export.
- The API runs in dev mode without MySQL: `uv run uvicorn
  fruitbox_api.app:create_app --factory --reload`. The SQLAlchemy engine in
  `python/fruitbox_api/db.py` is created lazily and never connected by the
  `/health`, `/api/v1/modes`, or `/api/v1/solver/static-move` routes, so the
  solver/core flow works fully offline. MySQL (via Docker Compose) is only
  needed for future persistent-state features.
- Docker is not preinstalled here; `docker compose` validation is optional and
  not required to run or test the app in dev mode (see "Docker in cloud VMs").

## Bootstrap tools

If uv or maturin are missing in a fresh cloud VM, install them for the current
user:

```bash
python3 -m pip install --user --upgrade uv maturin
export PATH="$HOME/.local/bin:$PATH"
```

Install project dependencies and build the Rust extension:

```bash
uv sync
```

## Docker in cloud VMs

If Docker is missing and the VM allows apt installs, install it impurely:

```bash
sudo apt-get update
sudo apt-get install -y docker.io docker-compose-v2
```

If there is no init system running Docker, start the daemon manually:

```bash
sudo dockerd --host=unix:///var/run/docker.sock
```

Some restricted cloud VMs do not support Docker's default iptables bridge setup.
For validation commands such as `docker compose config`, this fallback is enough:

```bash
sudo dockerd \
  --host=unix:///var/run/docker.sock \
  --iptables=false \
  --ip-forward=false \
  --bridge=none
```

Use `sudo docker ...` if the current user is not in the Docker group.

## Commits

This repo strictly uses [Conventional Commits](https://www.conventionalcommits.org/).
Every commit message must follow the format:

```
<type>[optional scope]: <description>
```

Common types include `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, and
`ci`. Use the imperative mood in the description (for example, `fix: handle
empty input` rather than `fixed empty input`).

## Validation

Run these checks before handing off changes:

```bash
uv run pytest
uv run ruff check .
cargo test
sudo docker compose config
```
