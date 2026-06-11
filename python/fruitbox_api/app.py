from fastapi import FastAPI

from fruitbox_api.config import get_settings
from fruitbox_api.routes import router


def create_app() -> FastAPI:
    settings = get_settings()
    app = FastAPI(title=settings.app_name)

    @app.get("/health")
    async def health() -> dict[str, str]:
        return {"status": "ok"}

    app.include_router(router, prefix="/api/v1")
    return app


app = create_app()
