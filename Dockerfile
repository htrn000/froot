FROM ghcr.io/astral-sh/uv:python3.12-bookworm

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential cargo pkg-config \
    && rm -rf /var/lib/apt/lists/*

ENV UV_COMPILE_BYTECODE=1
ENV UV_LINK_MODE=copy

COPY pyproject.toml uv.lock Cargo.toml Cargo.lock README.md ./
COPY data ./data
COPY python ./python
COPY src ./src

RUN uv sync --frozen

EXPOSE 8000

CMD ["uv", "run", "uvicorn", "fruitbox_api.app:create_app", "--factory", "--host", "0.0.0.0", "--port", "8000"]
