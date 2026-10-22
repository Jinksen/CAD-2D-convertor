import { describe, expect, it } from "vitest";

import { computeSection, fetchComponentPlane } from "./client";

const tilted = {
  kind: "custom" as const, origin_xyz: [0, 0, 10] as [number, number, number],
  normal_xyz: [0, 0, 1] as [number, number, number], x_dir_xyz: [1, 0, 0] as [number, number, number],
};

describe("computeSection", () => {
  it("rejects a section returned for a different plane", async () => {
    const fetcher: typeof fetch = () => Promise.resolve(new Response(JSON.stringify({
      section_id: "section-1", import_id: "import-1",
      plane: { kind: "XZ", offset_mm: 10 }, components: [], diagnostics: [],
    })));
    await expect(computeSection("import-1", { kind: "XY", offset_mm: 10 }, fetcher))
      .rejects.toThrow("invalid section response");
  });

  it("reports an expired import session from the backend", async () => {
    const fetcher: typeof fetch = () => Promise.resolve(new Response(JSON.stringify({
      error: { code: "unknown_import", message: "Import the file again." },
    }), { status: 404 }));
    await expect(computeSection("expired", { kind: "XY", offset_mm: 10 }, fetcher))
      .rejects.toThrow("Import the file again.");
  });

  it("sends a custom plane and accepts the echoed basis", async () => {
    let body: unknown = null;
    const fetcher: typeof fetch = (_url, init) => {
      body = JSON.parse(init?.body as string);
      return Promise.resolve(new Response(JSON.stringify({
        section_id: "section-1", import_id: "import-1", components: [], diagnostics: [],
        plane: { ...tilted, offset_mm: null },
      })));
    };
    const section = await computeSection("import-1", tilted, fetcher);
    expect(body).toEqual({ import_id: "import-1", plane: tilted });
    expect(section.plane.kind).toBe("custom");
  });

  it("rejects a custom section computed on another plane", async () => {
    const fetcher: typeof fetch = () => Promise.resolve(new Response(JSON.stringify({
      section_id: "section-1", import_id: "import-1", components: [], diagnostics: [],
      plane: { ...tilted, origin_xyz: [0, 0, 11] },
    })));
    await expect(computeSection("import-1", tilted, fetcher)).rejects.toThrow("invalid section response");
  });
});

describe("fetchComponentPlane", () => {
  it("returns the sheet plane of the selected body", async () => {
    let url = "";
    const fetcher: typeof fetch = (input) => {
      url = input as string;
      return Promise.resolve(new Response(JSON.stringify({
        component_id: "body 1", normal_xyz: [0.331, 0.013, 0.944], offset_mm: 131.1, planar_area_mm2: 11704,
      })));
    };
    const plane = await fetchComponentPlane("import-1", "body 1", fetcher);
    expect(url).toMatch(/\/api\/v1\/imports\/import-1\/components\/body%201\/plane$/);
    expect(plane.offset_mm).toBe(131.1);
  });

  it("reports a body without planar faces", async () => {
    const fetcher: typeof fetch = () => Promise.resolve(new Response(JSON.stringify({
      error: { code: "no_planar_face", message: "The selected body has no planar face." },
    }), { status: 422 }));
    await expect(fetchComponentPlane("import-1", "body-1", fetcher)).rejects.toThrow("no planar face");
  });
});
