# Fruitbox

Fruitbox game backend scaffold using:

- [uv](https://docs.astral.sh/uv/) for Python dependency and environment management;
- [maturin](https://www.maturin.rs/) for Rust/PyO3 bindings;
- FastAPI for the web API shell;
- Vite, TypeScript, and vite-plugin-pwa for the browser PWA;
- wasm-bindgen/wasm-pack for Rust game logic in the browser;
- Gymnasium/NumPy wrappers for Rust-backed RL simulation;
- MySQL for persistent backend state;
- Docker Compose for local stack orchestration.

The intended product shape is singleplayer, multiplayer, and selectable bot
modes. Singleplayer and deterministic/static bots should be designed to work
offline in the PWA. Heavier RL/NN bots can become offline-capable later if their
runtime and model size are suitable for browser delivery.

## Local development

Install the Python/Rust dependencies and build the Rust extension into the uv
environment:

```bash
uv sync
```

Run the API:

```bash
uv run uvicorn fruitbox_api.app:create_app --factory --reload
```

Run the website in development mode:

```bash
cd web
npm install
npm run dev
```

Vite proxies `/api` and `/health` to `localhost:8000`.

Build the PWA:

```bash
cd web
npm run build
```

Rebuild the Rust Wasm bindings after editing `wasm/fruitbox_wasm`:

```bash
cd web
npm run build:wasm
```

When `web/dist` exists, FastAPI serves the built website from `/` and keeps API
routes under `/api/v1`.

Run tests and linting:

```bash
uv run pytest
uv run ruff check .
cargo test
cd web && npm run build
```

## RL simulation

The Rust core exposes a batched simulator through maturin. Python wraps it as:

- `fruitbox_rl.FruitboxBatch` for NumPy-shaped batched simulation;
- `fruitbox_rl.FruitboxEnv` for a Gymnasium env backed by Rust `batch_size=1`.

For Stable-Baselines3 training, start with `sb3-contrib` `MaskablePPO` so the
agent can consume `env.action_masks()` and avoid invalid rectangle actions. See
`docs/rl.md` for the recommended SB3 subset and example training snippet.

Install the optional RL training stack and run the provided commands:

```bash
uv sync --group rl
uv run --group rl fruitbox-train-maskable-ppo --total-timesteps 100000
uv run --group rl fruitbox-infer-maskable-ppo models/fruitbox-maskable-ppo.zip
```

## Docker Compose

Start the website/API and MySQL:

```bash
docker compose up --build
```

The website and API listen on <http://localhost:8000>. MySQL listens on localhost
port 3306 with the development credentials from `docker-compose.yml`.

## API endpoints

- `GET /health`
- `GET /api/v1/modes`
- `POST /api/v1/solver/static-move`

Example solver request:

```bash
curl -X POST http://localhost:8000/api/v1/solver/static-move \
  -H 'content-type: application/json' \
  -d '{"width":3,"height":2,"cells":[1,2,4,3,4,6]}'
```

## Architecture guidance

The current scaffold intentionally keeps Vite/TypeScript for the PWA shell,
FastAPI as the HTTP/async orchestration layer, and Rust as the deterministic
game/solver core. Browser singleplayer now imports Rust-compiled Wasm for board
generation, scoring, applying moves, and offline static solver hints.

No frontend framework is required yet. Plain TypeScript keeps the current
singleplayer game small; introduce React/Svelte/etc. later only if UI state,
routing, or reusable components become painful.

If real-time multiplayer, authoritative game ticks, matchmaking, or long-running
bot jobs become the backend's dominant concern, consider promoting the Rust core
into a dedicated Axum/Tokio service. See `docs/architecture.md` for details.
