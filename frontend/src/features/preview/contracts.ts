export interface PreviewComponentMesh {
  component_id: string;
  color: [number, number, number] | null;
  positions: [number, number, number][];
  normals: [number, number, number][];
  indices: number[];
}

export interface PreviewMeshResponse {
  import_id: string;
  components: PreviewComponentMesh[];
}

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function vector(value: unknown): value is [number, number, number] {
  return Array.isArray(value) && value.length === 3 &&
    value.every((number) => typeof number === "number" && Number.isFinite(number));
}

function mesh(value: unknown): value is PreviewComponentMesh {
  if (!record(value) || typeof value.component_id !== "string") return false;
  const { positions, normals, indices, color } = value;
  if (!Array.isArray(positions) || !positions.every(vector) ||
      !Array.isArray(normals) || !normals.every(vector) ||
      !Array.isArray(indices) || indices.length % 3 !== 0 ||
      positions.length !== normals.length) return false;
  if (color !== null && (!vector(color) || !color.every(
    (channel) => Number.isInteger(channel) && channel >= 0 && channel <= 255,
  ))) return false;
  return indices.every((index) => Number.isInteger(index) &&
    index >= 0 && index < positions.length);
}

export function isPreviewMeshResponse(value: unknown): value is PreviewMeshResponse {
  return record(value) && typeof value.import_id === "string" &&
    Array.isArray(value.components) && value.components.every(mesh);
}
