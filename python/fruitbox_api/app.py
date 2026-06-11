from pathlib import Path

from fastapi import FastAPI, HTTPException
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles

from fruitbox_api.config import get_settings
from fruitbox_api.routes import router


def create_app() -> FastAPI:
    settings = get_settings()
    app = FastAPI(title=settings.app_name)

    @app.get("/health")
    async def health() -> dict[str, str]:
        return {"status": "ok"}

    app.include_router(router, prefix="/api/v1")
    _mount_frontend(app, settings.frontend_dist_path)
    return app


def _mount_frontend(app: FastAPI, frontend_dist_path: Path) -> None:
    frontend_dist = _resolve_frontend_dist(frontend_dist_path)
    index_file = frontend_dist / "index.html"
    assets_dir = frontend_dist / "assets"

    if not index_file.exists():
        return

    if assets_dir.exists():
        app.mount("/assets", StaticFiles(directory=assets_dir), name="assets")

    @app.get("/", include_in_schema=False)
    async def serve_index() -> FileResponse:
        return FileResponse(index_file)

    @app.get("/{path:path}", include_in_schema=False)
    async def serve_frontend_path(path: str) -> FileResponse:
        if path.startswith(("api/", "health")):
            raise HTTPException(status_code=404)

        candidate = frontend_dist / path
        if candidate.is_file():
            return FileResponse(candidate)

        return FileResponse(index_file)


def _resolve_frontend_dist(frontend_dist_path: Path) -> Path:
    if frontend_dist_path.is_absolute():
        return frontend_dist_path

    return Path.cwd() / frontend_dist_path


app = create_app()
