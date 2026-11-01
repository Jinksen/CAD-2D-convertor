from pydantic import BaseModel, ConfigDict

from cad2maxwell_backend.models.imports import Rgb


class PreviewComponentMesh(BaseModel):
    """Display-only mesh; buffers are base64 little-endian typed arrays."""

    component_id: str
    color: Rgb | None
    positions: str  # Float32 xyz per node
    normals: str  # Float32 xyz per node
    indices: str  # Uint32 node triples, one per triangle
    edge_indices: str  # Uint32 node pairs, one per B-Rep edge segment


class PreviewMeshResponse(BaseModel):
    model_config = ConfigDict(allow_inf_nan=False)

    import_id: str
    components: list[PreviewComponentMesh]
    deflection_mm: float
    coarsened: bool
