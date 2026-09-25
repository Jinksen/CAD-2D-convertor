from pydantic import BaseModel, ConfigDict, Field

from cad2maxwell_backend.models.imports import Rgb


class PreviewComponentMesh(BaseModel):
    model_config = ConfigDict(allow_inf_nan=False)

    component_id: str
    color: Rgb | None
    positions: list[tuple[float, float, float]]
    normals: list[tuple[float, float, float]]
    indices: list[int] = Field(default_factory=list)


class PreviewMeshResponse(BaseModel):
    import_id: str
    components: list[PreviewComponentMesh]
