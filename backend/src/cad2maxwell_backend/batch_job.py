import json
import re
from pathlib import Path
from typing import Literal

from pydantic import BaseModel, ConfigDict, Field, field_validator, model_validator

from cad2maxwell_backend.models.exports import ExportFormat

_NAME = re.compile(r"^[A-Za-z0-9_-]{1,64}$")


class JobSection(BaseModel):
    model_config = ConfigDict(extra="forbid", allow_inf_nan=False)

    name: str
    plane: Literal["XY", "XZ", "YZ"]
    offset_mm: float
    formats: list[ExportFormat] = Field(default=["dxf"], min_length=1)

    @field_validator("name")
    @classmethod
    def ascii_name(cls, value: str) -> str:
        if not _NAME.match(value):
            raise ValueError("name must be 1-64 ASCII letters, digits, '_' or '-'")
        return value


class BatchJob(BaseModel):
    model_config = ConfigDict(extra="forbid")

    input: str
    output_dir: str
    sections: list[JobSection] = Field(min_length=1, max_length=256)

    @model_validator(mode="after")
    def unique_names(self) -> "BatchJob":
        names = [section.name.casefold() for section in self.sections]
        if len(set(names)) != len(names):
            raise ValueError("section names must be unique")
        return self


def load_job(path: Path) -> BatchJob:
    return BatchJob.model_validate(json.loads(path.read_text(encoding="utf-8")))
