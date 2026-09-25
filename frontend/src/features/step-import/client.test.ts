import { describe, expect, it } from "vitest";

import { importStep } from "./client";

const imported = {
  import_id: "session-1", path: "C:/motor.step", sha256: "abc", source_unit: "mm",
  to_mm_scale: 1, bounds_mm: { min_xyz: [0, 0, 0], max_xyz: [10, 20, 30] },
  component_count: 1,
  components: [{ id: "body-1", source_id: null, source_name: "Rotor", display_name: "Rotor",
    export_name: "Rotor", hierarchy_path: ["Motor", "Rotor"], source_color: [255, 0, 0],
    name_provenance: "product", color_provenance: "shape", body_ordinal: 0,
    bounds_mm: { min_xyz: [0, 0, 0], max_xyz: [10, 20, 30] } }],
  diagnostics: [],
};

describe("importStep", () => {
  it("sends the absolute path and accepts a typed import summary", async () => {
    const fetcher: typeof fetch = (_url, init) => {
      expect(init?.method).toBe("POST");
      expect(JSON.parse(init?.body as string)).toEqual({ path: "C:/motor.step" });
      return Promise.resolve(new Response(JSON.stringify(imported), { status: 200 }));
    };
    await expect(importStep("C:/motor.step", fetcher)).resolves.toEqual(imported);
  });

  it("uses the backend's actionable error message", async () => {
    const fetcher: typeof fetch = () => Promise.resolve(new Response(JSON.stringify({
      error: { code: "missing_import_file", message: "The STEP file was not found." },
    }), { status: 404 }));
    await expect(importStep("C:/missing.step", fetcher)).rejects.toThrow("The STEP file was not found.");
  });

  it("rejects malformed success data", async () => {
    const fetcher: typeof fetch = () => Promise.resolve(new Response(JSON.stringify({ ...imported, components: [] })));
    await expect(importStep("C:/motor.step", fetcher)).rejects.toThrow("invalid import response");
  });
});
