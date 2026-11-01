/** Wire format: buffers are base64 little-endian typed arrays (display-only mesh). */
export interface PreviewComponentMeshWire {
  component_id: string;
  color: [number, number, number] | null;
  positions: string;
  normals: string;
  indices: string;
  edge_indices: string;
}

export interface PreviewMeshWire {
  import_id: string;
  components: PreviewComponentMeshWire[];
  deflection_mm: number;
  coarsened: boolean;
}

export interface PreviewComponentMesh {
  component_id: string;
  color: [number, number, number] | null;
  positions: Float32Array;
  normals: Float32Array;
  indices: Uint32Array;
  edgeIndices: Uint32Array;
}

export interface PreviewMeshResponse {
  import_id: string;
  components: PreviewComponentMesh[];
  deflection_mm: number;
  coarsened: boolean;
}

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function rgb(value: unknown): value is [number, number, number] {
  return Array.isArray(value) && value.length === 3 &&
    value.every((channel) => Number.isInteger(channel) && channel >= 0 && channel <= 255);
}

function meshWire(value: unknown): value is PreviewComponentMeshWire {
  return record(value) && typeof value.component_id === "string" &&
    (value.color === null || rgb(value.color)) &&
    ["positions", "normals", "indices", "edge_indices"].every((key) => typeof value[key] === "string");
}

export function isPreviewMeshResponse(value: unknown): value is PreviewMeshWire {
  return record(value) && typeof value.import_id === "string" &&
    typeof value.deflection_mm === "number" && Number.isFinite(value.deflection_mm) &&
    typeof value.coarsened === "boolean" &&
    Array.isArray(value.components) && value.components.every(meshWire);
}

function bytes(base64: string): ArrayBuffer {
  const text = atob(base64);
  if (text.length % 4 !== 0) throw new Error("Preview buffer is not 32-bit aligned.");
  const buffer = new Uint8Array(text.length);
  for (let index = 0; index < text.length; index += 1) buffer[index] = text.charCodeAt(index);
  return buffer.buffer;
}

// shortcut: typed arrays use platform byte order, which is little-endian on every supported WebView.
function decodeMesh(wire: PreviewComponentMeshWire): PreviewComponentMesh {
  const positions = new Float32Array(bytes(wire.positions));
  const normals = new Float32Array(bytes(wire.normals));
  const indices = new Uint32Array(bytes(wire.indices));
  const edgeIndices = new Uint32Array(bytes(wire.edge_indices));
  const nodes = positions.length / 3;
  if (positions.length % 3 !== 0 || normals.length !== positions.length ||
      indices.length % 3 !== 0 || edgeIndices.length % 2 !== 0) {
    throw new Error("Preview buffers have inconsistent lengths.");
  }
  if (!positions.every(Number.isFinite) || !normals.every(Number.isFinite)) {
    throw new Error("Preview buffers contain non-finite values.");
  }
  if (indices.some((index) => index >= nodes) || edgeIndices.some((index) => index >= nodes)) {
    throw new Error("Preview indices are out of range.");
  }
  return { component_id: wire.component_id, color: wire.color, positions, normals, indices, edgeIndices };
}

/** Decodes and validates the wire buffers; throws on malformed data. */
export function decodePreviewMesh(wire: PreviewMeshWire): PreviewMeshResponse {
  return { ...wire, components: wire.components.map(decodeMesh) };
}
