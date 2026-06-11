# Fruitbox architecture notes

## Recommendation

Start with a Vite/TypeScript PWA served by a FastAPI service that calls a Rust
core through maturin/PyO3.

That pattern is a good fit while the backend is mostly product/API orchestration:

- account/session APIs, persistence, and deployment configuration in Python;
- async MySQL access and web concerns in FastAPI;
- deterministic game rules, scoring, batched simulation, and static solver code in Rust;
- one Rust crate that can later expose both PyO3 bindings and a Wasm/browser build.
- a minimal browser stack without a framework until the UI complexity needs one.

Keep the Rust functions mostly pure and CPU-bound. Python should own the HTTP
async loop, database sessions, validation, and background task orchestration.
For expensive bot searches, call Rust from a worker thread/process instead of
blocking the event loop directly.

Move the API loop into Rust when the multiplayer/runtime layer becomes the
dominant complexity:

- authoritative real-time game ticks or low-latency WebSockets;
- matchmaking and rooms with many concurrent sessions;
- long-running bot simulations or inference jobs;
- a need to share Tokio-native code directly with the network server.

A practical path is to keep `fruitbox-core` independent from FastAPI. Today it
is loaded through maturin as `fruitbox_core._native`; later the same crate can
grow a Wasm target for offline bots or be reused by an Axum/Tokio service if the
multiplayer workload justifies it.

## Offline/PWA implications

Singleplayer should be designed as browser-first:

- cache the app shell and static assets with the PWA service worker;
- keep deterministic game rules in shared Rust so they can compile to Wasm;
- store local progress in IndexedDB and sync when online.

Static solver bots are the easiest offline target because they are deterministic
and compact. RL/NN bots can be offline if the model is small enough and runs in
the browser through Wasm, WebGPU, or ONNX Runtime Web. Larger models are better
kept server-side at first and exposed as an online bot mode.

## Current scaffold

- `web`: Vite/TypeScript PWA with a playable singleplayer board and solver hints.
- `python/fruitbox_api`: FastAPI app, config, request/response models, routes.
- `python/fruitbox_core`: Python import wrapper for the maturin extension.
- `python/fruitbox_rl`: Gymnasium/NumPy adapters for Rust-backed simulation.
- `src/lib.rs`: Rust/PyO3 game-core primitive for finding target-sum rectangles.
- `docs/rl.md`: Stable-Baselines3 guidance for the Gymnasium env.
- `docker-compose.yml`: API plus MySQL 8.4.
- `pyproject.toml` and `uv.lock`: uv-managed Python dependencies.
- `Cargo.toml` and `Cargo.lock`: Rust dependency lockfile.
