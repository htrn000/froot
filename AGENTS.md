# AGENTS.md

## Cursor Cloud specific instructions

Fruitbox is a FastAPI (Python 3.12) backend with a Rust/PyO3 solver core, managed
by [`uv`](https://docs.astral.sh/uv/) and built with [`maturin`](https://www.maturin.rs/).
Standard dev commands (sync, run, test, lint) live in `README.md`; the notes below
are only the non-obvious caveats.

- `uv` is installed by the startup update script to `~/.local/bin`. If `uv` is not
  found in a shell, add it to `PATH` with `export PATH="$HOME/.local/bin:$PATH"`.
- The Rust extension (`fruitbox_core._native`) is compiled into the uv environment
  by `uv sync` via maturin. After editing `src/lib.rs`, re-run `uv sync` (or
  `uv run maturin develop`) to rebuild it — uvicorn `--reload` only watches the
  Python sources and will NOT pick up Rust changes on its own.
- MySQL is NOT required to run or test the currently implemented endpoints
  (`GET /health`, `GET /api/v1/modes`, `POST /api/v1/solver/static-move`). The
  SQLAlchemy async engine in `python/fruitbox_api/db.py` is created lazily and is
  not exercised by these routes. Only start MySQL (`docker compose up`) when working
  on DB-backed features.
- Run the dev server with `uv run uvicorn fruitbox_api.app:create_app --factory --reload`.
  Smoke-test the Rust path end to end with:
  `curl -X POST http://localhost:8000/api/v1/solver/static-move -H 'content-type: application/json' -d '{"width":3,"height":2,"cells":[1,2,4,3,4,6]}'`
