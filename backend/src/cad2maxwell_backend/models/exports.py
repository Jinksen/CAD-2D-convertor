from typing import Literal

from pydantic import BaseModel, Field, field_validator, model_validator

LayerMode = Literal["component", "group", "hybrid"]
ColorMode = Literal["original", "group", "material", "monochrome"]
ExportFormat = Literal["dxf", "svg", "csv", "femm", "gmsh", "pyaedt"]
# User-assigned labels only; never inferred from names or colours (spec §16).
MaterialLabel = Literal[
    "air", "copper", "aluminum", "electrical_steel", "structural_steel",
    "permanent_magnet", "insulation", "custom",
]


class ExportGroup(BaseModel):
    name: str = Field(min_length=1, max_length=80)
    members: list[str] = Field(default_factory=list, max_length=4096)
    material: MaterialLabel | None = None
    material_note: str | None = Field(default=None, max_length=80)


class ExportOptions(BaseModel):
    layer_mode: LayerMode = "component"
    color_mode: ColorMode = "original"
    groups: list[ExportGroup] = Field(default_factory=list, max_length=256)
    formats: list[ExportFormat] = Field(default=["dxf"], min_length=1)

    @field_validator("formats")
    @classmethod
    def unique_formats(cls, value: list[ExportFormat]) -> list[ExportFormat]:
        if len(set(value)) != len(value):
            raise ValueError("formats must be unique")
        return value

    @model_validator(mode="after")
    def unique_groups(self) -> "ExportOptions":
        names = [group.name.casefold() for group in self.groups]
        if len(set(names)) != len(names):
            raise ValueError("group names must be unique")
        return self


class ExportRequest(ExportOptions):
    import_id: str = Field(min_length=1)
    section_id: str = Field(min_length=1)
