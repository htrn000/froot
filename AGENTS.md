# Agent setup notes

This repo uses Python 3.12, uv, maturin/PyO3, Rust/Cargo, FastAPI, Vite,
TypeScript, Gymnasium, optional Stable-Baselines3/SB3-contrib, and Docker
Compose.

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

Install frontend dependencies:

```bash
cd web
npm install
```

Install the optional RL training stack only when needed; it pulls large PyTorch
artifacts:

```bash
uv sync --group rl
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

If `docker compose build` fails with `network bridge not found` in a restricted
VM, validate the Dockerfile directly with host networking:

```bash
sudo docker build --network=host -t fruitbox-api-test .
```

## Validation

Run these checks before handing off changes:

```bash
uv run pytest
uv run ruff check .
cargo test
cd web && npm run build
sudo docker compose config
sudo docker build --network=host -t fruitbox-api-test .
```
