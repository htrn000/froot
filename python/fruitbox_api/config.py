from functools import lru_cache
from pathlib import Path

from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    app_name: str = "Fruitbox API"
    database_url: str = "mysql+asyncmy://fruitbox:fruitbox@localhost:3306/fruitbox"
    frontend_dist_path: Path = Path("web/dist")

    model_config = SettingsConfigDict(
        env_file=".env",
        env_prefix="FRUITBOX_",
        extra="ignore",
    )


@lru_cache
def get_settings() -> Settings:
    return Settings()
