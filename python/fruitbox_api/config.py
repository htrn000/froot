from functools import lru_cache

from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    app_name: str = "Fruitbox API"
    database_url: str = "mysql+asyncmy://fruitbox:fruitbox@localhost:3306/fruitbox"
    sample_catalog_path: str = "data/provisioned/fruitbox_samples.sqlite3"
    sample_catalog_overlays: str = ""

    model_config = SettingsConfigDict(
        env_file=".env",
        env_prefix="FRUITBOX_",
        extra="ignore",
    )


@lru_cache
def get_settings() -> Settings:
    return Settings()
