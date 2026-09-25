import { describe, expect, it, vi } from "vitest";

import { fetchPreview } from "./client";

const preview = {
  import_id: "import-1",
  components: [{
    component_id: "component-1", color: [204, 51, 25],
    positions: [[0, 0, 0], [1, 0, 0], [0, 1, 0]],
    normals: [[0, 0, 1], [0, 0, 1], [0, 0, 1]], indices: [0, 1, 2],
  }],
};

describe("preview client", () => {
  it("reads a typed component mesh", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(new Response(JSON.stringify(preview)));

    expect(await fetchPreview("import-1", fetcher)).toEqual(preview);
    expect(fetcher).toHaveBeenCalledWith("http://127.0.0.1:8000/api/v1/imports/import-1/preview");
  });

  it("rejects malformed triangle indices", async () => {
    const malformed = { ...preview, components: [{ ...preview.components[0], indices: [0, 1, 3] }] };
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(new Response(JSON.stringify(malformed)));

    await expect(fetchPreview("import-1", fetcher)).rejects.toThrow("invalid preview response");
  });
});
