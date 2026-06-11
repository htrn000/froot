FROM node:22-bookworm AS web-builder

WORKDIR /web

COPY web/package.json web/package-lock.json ./
RUN npm ci

COPY web ./
RUN npm run build

FROM ghcr.io/astral-sh/uv:python3.12-bookworm

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential curl pkg-config \
    && rm -rf /var/lib/apt/lists/*

ENV UV_COMPILE_BYTECODE=1
ENV UV_LINK_MODE=copy
ENV PATH="/root/.cargo/bin:$PATH"

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain stable

COPY pyproject.toml uv.lock Cargo.toml Cargo.lock README.md ./
COPY python ./python
COPY src ./src
COPY --from=web-builder /web/dist ./web/dist

RUN uv sync --frozen --no-dev

EXPOSE 8000

CMD ["uv", "run", "uvicorn", "fruitbox_api.app:create_app", "--factory", "--host", "0.0.0.0", "--port", "8000"]
