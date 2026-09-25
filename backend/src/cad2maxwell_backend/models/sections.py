import math
from typing import Annotated, Literal

from pydantic import BaseModel, ConfigDict, Field, model_validator

Point2 = tuple[float, float]
Point3 = tuple[float, float, float]


class FiniteModel(BaseModel):
    model_config = ConfigDict(allow_inf_nan=False)


class SectionPlane(FiniteModel):
    kind: Literal["XY", "XZ", "YZ", "custom"]
    offset_mm: float | None = None
    origin_xyz: Point3 | None = None
    normal_xyz: Point3 | None = None
    x_dir_xyz: Point3 | None = None

    @model_validator(mode="after")
    def valid_basis(self) -> "SectionPlane":
        if self.kind != "custom":
            if self.offset_mm is None or any(
                value is not None for value in (self.origin_xyz, self.normal_xyz, self.x_dir_xyz)
            ):
                raise ValueError("standard planes need only an offset")
            return self
        if self.offset_mm is not None or any(
            value is None for value in (self.origin_xyz, self.normal_xyz, self.x_dir_xyz)
        ):
            raise ValueError("custom planes need origin, normal, and x direction")
        assert self.normal_xyz is not None and self.x_dir_xyz is not None
        normal_length = math.hypot(*self.normal_xyz)
        x_length = math.hypot(*self.x_dir_xyz)
        if normal_length == 0 or x_length == 0:
            raise ValueError("plane directions must be nonzero")
        dot = sum(
            (a / normal_length) * (b / x_length)
            for a, b in zip(self.normal_xyz, self.x_dir_xyz, strict=True)
        )
        if abs(dot) > 1e-8:
            raise ValueError("plane x direction must be perpendicular to normal")
        return self


class SectionRequest(FiniteModel):
    import_id: str = Field(min_length=1)
    plane: SectionPlane


class Line2D(FiniteModel):
    type: Literal["line"] = "line"
    start: Point2
    end: Point2


class Circle2D(FiniteModel):
    type: Literal["circle"] = "circle"
    center: Point2
    radius: float = Field(gt=0)
    x_axis: Point2
    y_axis: Point2
    start_parameter: float
    end_parameter: float


class Ellipse2D(FiniteModel):
    type: Literal["ellipse"] = "ellipse"
    center: Point2
    major_radius: float = Field(gt=0)
    minor_radius: float = Field(gt=0)
    x_axis: Point2
    y_axis: Point2
    start_parameter: float
    end_parameter: float


Curve2D = Annotated[Line2D | Circle2D | Ellipse2D, Field(discriminator="type")]


class SectionWire(FiniteModel):
    closed: bool
    role: Literal["outer", "hole"] | None = None
    curves: list[Curve2D]


class SectionComponent(FiniteModel):
    component_id: str
    wires: list[SectionWire]


class SectionDiagnostic(FiniteModel):
    code: str
    severity: Literal["warning", "error"]
    message: str
    component_id: str | None = None
    related_component_id: str | None = None


class SectionResponse(FiniteModel):
    section_id: str
    import_id: str
    plane: SectionPlane
    components: list[SectionComponent]
    diagnostics: list[SectionDiagnostic]
