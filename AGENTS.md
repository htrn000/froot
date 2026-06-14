# Agent setup notes

This repo uses Python 3.12, uv, maturin/PyO3, Rust/Cargo, FastAPI, and Docker
Compose.

## Repository direction

`froot` is a standalone repo today, but it is expected to be interned into the
`depot` monorepo eventually. Keep documentation, CI, and environment setup easy
to transplant into `depot` instead of assuming `froot` will stay isolated.

## Cursor Cloud specific instructions

### Shared environment with depot

Cursor Cloud environments for this workspace should support both repos:

- `froot`: Python 3.12, `uv`, `maturin`/PyO3, Rust stable, FastAPI, and Docker
  Compose.
- `depot`: Nix, `nix-daemon` when systemd is unavailable, flake commands with
  `--accept-flake-config`, and the depot default dev shell.

When changing startup scripts, base images, or environment docs, preserve this
dual-repo setup so agents can validate `froot` in place today and later move it
into `depot` without reworking the Cloud toolchain.

### Toolchain is preinstalled in the VM snapshot

The heavy tools are baked into the VM snapshot, so fresh agents do **not**
reinstall them each run:

- Python 3.12, `uv` + `maturin` (in `~/.local/bin`).
- `rustup` with the `stable` toolchain as default (`CARGO_HOME=/usr/local/cargo`,
  `RUSTUP_HOME=/usr/local/rustup`, both on `PATH` system-wide). `stable`
  (currently `rustc`/`cargo` 1.96) covers `froot`'s crate: Rust **edition 2021**,
  `Cargo.lock` format **v4** (needs cargo ≥ 1.78), and `pyo3 0.28`. There is no
  `rust-toolchain.toml` pin, so keep `stable` recent enough for these.
- `docker.io` (Docker Engine) + the `docker compose` v2 plugin (`docker-compose-v2`).

`~/.bashrc` exports `~/.local/bin` and `$CARGO_HOME/bin` on `PATH` and sets
`CARGO_HOME`/`RUSTUP_HOME`, so interactive shells resolve `uv`, `rustc`, and
`cargo` without manual setup.

### Per-session update script (fast + idempotent)

The startup script verifies tools and refreshes deps without reinstalling the
preinstalled toolchain on every run:

1. Installs `uv`/`maturin` **only if missing** (guarded fallback).
2. Ensures `docker.io` + `docker-compose-v2` — `apt`-installs them **only if
   `docker` is missing** (guarded; a no-op on snapshotted VMs that already have
   it).
3. Prints startup checks for `uv`, `rustc`, and `docker compose`.
4. Runs `uv sync` (which rebuilds the `fruitbox_core` Rust extension only when
   sources change).

The guarded `docker` install is the only `apt` step and self-heals a VM that
lost Docker; the daemon is still not started here (see below).

### Docker daemon for restricted VMs

`docker compose config` does **not** need a running daemon, but `docker compose
up`/`build` do. Cloud VMs often lack systemd, so start the daemon manually with
the restricted-VM fallback flags (no iptables/bridge):

```bash
sudo dockerd --host=unix:///var/run/docker.sock \
  --iptables=false --ip-forward=false --bridge=none >/var/log/dockerd.log 2>&1 &
```

Use `sudo docker ...` (the current user is not in the `docker` group).

### Running the app without MySQL

The API runs in dev mode without MySQL: `uv run uvicorn
fruitbox_api.app:create_app --factory --reload`. The SQLAlchemy engine in
`python/fruitbox_api/db.py` is created lazily and never connected by the
`/health`, `/api/v1/modes`, or `/api/v1/solver/static-move` routes, so the
solver/core flow works fully offline. MySQL (via Docker Compose) is only needed
for future persistent-state features.

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

## CI workflow parity

The authoritative premerge checks live in:

- `.github/workflows/premerge-validation.yml`

`preapproved` is guarded by the workflow's `PREAPPROVED_MAINTAINERS` list, which
includes the repository creator `htrn000`. Pull requests targeting
`preapproved` are only auto-merge candidates when the PR author is in that list
or a maintainer from that list has approved the current PR head commit.

Before requesting merge, review that workflow and run the same commands locally
to verify your changes against CI expectations.
