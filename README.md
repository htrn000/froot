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

Run the API:

```bash
uv run uvicorn fruitbox_api.app:create_app --factory --reload
```

Run tests and linting:

```bash
uv run pytest
uv run ruff check .
```

Run the Rust static-solver benchmark binary:

```bash
cargo run --release --bin fruitbox_bench -- --generator fungster --width 17 --height 10
```

The benchmark prints CSV rows for DFS single-solution candidates and the
memoized exhaustive DP summary. Use `--generator random` for positive 17x10
boards whose total sum is divisible by 10, or `--generator rejection` to keep
sampling until the DFS candidate finds an empty-board solution. Prefer official
17x10 grids for benchmark evidence; smaller grids are only for fast debugging
or solver research iteration.

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

## Architecture guidance

The current scaffold intentionally keeps FastAPI as the HTTP/async orchestration
layer and Rust as the deterministic game/solver core. That is a good starting
pattern for product APIs, MySQL access, and PWA-facing routes while preserving a
path to reuse the Rust crate from Wasm.

If real-time multiplayer, authoritative game ticks, matchmaking, or long-running
bot jobs become the backend's dominant concern, consider promoting the Rust core
into a dedicated Axum/Tokio service. See `docs/architecture.md` for details.
