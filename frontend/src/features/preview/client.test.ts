import { describe, expect, it, vi } from "vitest";

import { fetchPreview } from "./client";

function base64(array: Float32Array | Uint32Array): string {
  return btoa(String.fromCharCode(...new Uint8Array(array.buffer)));
}

const positions = new Float32Array([0, 0, 0, 1.5, 0, 0, 0, 1, 0]);
const normals = new Float32Array([0, 0, 1, 0, 0, 1, 0, 0, 1]);
const preview = {
  import_id: "import-1", deflection_mm: 0.05, coarsened: false,
  components: [{
    component_id: "component-1", color: [204, 51, 25],
    positions: base64(positions), normals: base64(normals),
    indices: base64(new Uint32Array([0, 1, 2])), edge_indices: base64(new Uint32Array([0, 1, 1, 2])),
  }],
};

function respond(body: unknown) {
  return vi.fn<typeof fetch>().mockResolvedValue(new Response(JSON.stringify(body)));
}

describe("preview client", () => {
  it("decodes base64 little-endian buffers into typed arrays", async () => {
    const fetcher = respond(preview);

    const result = await fetchPreview("import-1", fetcher);

    expect(fetcher).toHaveBeenCalledWith("http://127.0.0.1:8000/api/v1/imports/import-1/preview", {
      headers: { Accept: "application/json" },
    });
    expect(result.coarsened).toBe(false);
    const mesh = result.components[0];
    expect(mesh.component_id).toBe("component-1");
    expect(mesh.color).toEqual([204, 51, 25]);
    expect(Array.from(mesh.positions)).toEqual([0, 0, 0, 1.5, 0, 0, 0, 1, 0]);
    expect(Array.from(mesh.normals)).toEqual(Array.from(normals));
    expect(Array.from(mesh.indices)).toEqual([0, 1, 2]);
    expect(Array.from(mesh.edgeIndices)).toEqual([0, 1, 1, 2]);
  });

  it("rejects out-of-range triangle indices", async () => {
    const malformed = { ...preview, components: [{
      ...preview.components[0], indices: base64(new Uint32Array([0, 1, 3])),
    }] };

    await expect(fetchPreview("import-1", respond(malformed))).rejects.toThrow("invalid preview response");
  });

  it("rejects buffers with mismatched lengths", async () => {
    const malformed = { ...preview, components: [{
      ...preview.components[0], normals: base64(new Float32Array([0, 0, 1])),
    }] };

    await expect(fetchPreview("import-1", respond(malformed))).rejects.toThrow("invalid preview response");
  });

  it("rejects the legacy nested-array payload", async () => {
    const legacy = { ...preview, components: [{ ...preview.components[0], positions: [[0, 0, 0]] }] };

    await expect(fetchPreview("import-1", respond(legacy))).rejects.toThrow("invalid preview response");
  });
});
