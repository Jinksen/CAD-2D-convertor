from typing import Literal

from pydantic import SecretStr
from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    """Runtime settings restricted to the local desktop boundary."""

    model_config = SettingsConfigDict(env_prefix="CAD2MAXWELL_")

    host: Literal["127.0.0.1"] = "127.0.0.1"
    port: int = 8000
    session_token: SecretStr | None = None
