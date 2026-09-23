from typing import Literal

from pydantic import BaseModel


class HealthResponse(BaseModel):
    status: Literal["ok"] = "ok"
    service: str = "cad2maxwell-backend"
    api_version: str = "v1"
