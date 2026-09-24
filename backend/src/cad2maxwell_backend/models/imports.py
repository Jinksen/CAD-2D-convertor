from typing import Annotated, Literal

from pydantic import BaseModel, ConfigDict, Field, model_validator

RgbChannel = Annotated[int, Field(ge=0, le=255)]
Rgb = tuple[RgbChannel, RgbChannel, RgbChannel]


class BoundingBox(BaseModel):
    model_config = ConfigDict(allow_inf_nan=False)

    min_xyz: tuple[float, float, float]
    max_xyz: tuple[float, float, float]

    @model_validator(mode="after")
    def ordered(self) -> "BoundingBox":
        if any(low > high for low, high in zip(self.min_xyz, self.max_xyz, strict=True)):
            raise ValueError("bounding box minimum exceeds maximum")
        return self


class ImportRequest(BaseModel):
    path: str


class ImportDiagnostic(BaseModel):
    code: str
    severity: Literal["info", "warning", "error"]
    message: str
    component_id: str | None = None


class ImportComponent(BaseModel):
    id: str
    source_id: str | None = None
    source_name: str | None
    display_name: str
    export_name: str
    hierarchy_path: list[str]
    source_color: Rgb | None
    name_provenance: Literal["occurrence", "product"] | None = None
    color_provenance: Literal["instance", "shape"] | None = None
    body_ordinal: int = Field(ge=0)
    bounds_mm: BoundingBox


class ImportResponse(BaseModel):
    import_id: str
    path: str
    sha256: str
    source_unit: str | None
    to_mm_scale: float | None
    bounds_mm: BoundingBox
    component_count: int = Field(ge=0)
    components: list[ImportComponent]
    diagnostics: list[ImportDiagnostic]
