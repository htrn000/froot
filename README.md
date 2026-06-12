# Fruitbox

Fruitbox game backend scaffold using:

- [uv](https://docs.astral.sh/uv/) for Python dependency and environment management;
- [maturin](https://www.maturin.rs/) for Rust/PyO3 bindings;
- FastAPI for the web API shell;
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

Install and run the frontend dev server:

```bash
cd frontend
npm install
npm run dev
```

In another terminal, run the API:

```bash
uv run uvicorn fruitbox_api.app:create_app --factory --reload
```

Open <http://localhost:5173> for the game UI. Vite proxies `/api` and `/health`
to the FastAPI process on port 8000.

Build the frontend for production serving from FastAPI:

```bash
cd frontend
npm run build
uv run uvicorn fruitbox_api.app:create_app --factory --reload
```

Open <http://localhost:8000> after building the frontend.

Run tests and linting:

```bash
uv run pytest
uv run ruff check .
cd frontend && npm test
```

## Docker Compose

Start the API and MySQL:

```bash
docker compose up --build
```

The API listens on <http://localhost:8000>. MySQL listens on localhost port
3306 with the development credentials from `docker-compose.yml`.

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

## Frontend

The browser UI lives in `frontend/` and uses:

- Vite + TypeScript for the app shell;
- HTML/CSS grid cells for the board, not canvas;
- a pure TypeScript game engine under `frontend/src/game/` for headless play,
  bot turns, and offline validation;
- optional API calls for health checks and server-backed solver hints.

DOM cells are the default because rectangle drag-selection, accessibility, and
headless testing are much simpler than canvas hit-testing. The engine is kept
separate from rendering so the same rules can later move into the Rust/Wasm
core without rewriting the UI.

## Architecture guidance

The current scaffold intentionally keeps FastAPI as the HTTP/async orchestration
layer and Rust as the deterministic game/solver core. That is a good starting
pattern for product APIs, MySQL access, and PWA-facing routes while preserving a
path to reuse the Rust crate from Wasm.

If real-time multiplayer, authoritative game ticks, matchmaking, or long-running
bot jobs become the backend's dominant concern, consider promoting the Rust core
into a dedicated Axum/Tokio service. See `docs/architecture.md` for details.
