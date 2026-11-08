import { describe, expect, it } from "vitest";

import type { ImportComponent } from "./contracts";
import { bodyMatchesFilter } from "./treeFilter";

function body(overrides: Partial<ImportComponent>): ImportComponent {
  return {
    id: "body-1", source_id: null, source_name: "Rotor", display_name: "Rotor",
    export_name: "Rotor_1", hierarchy_path: ["Motor", "Rotor"], source_color: [255, 0, 0],
    name_provenance: "product", color_provenance: "shape", body_ordinal: 0,
    bounds_mm: { min_xyz: [0, 0, 0], max_xyz: [10, 20, 30] },
    ...overrides,
  };
}

describe("bodyMatchesFilter", () => {
  it("treats blank and whitespace-only queries as a match", () => {
    expect(bodyMatchesFilter(body({}), "")).toBe(true);
    expect(bodyMatchesFilter(body({}), "   ")).toBe(true);
  });

  it("matches display name, export name, or a parent in the path", () => {
    const rotor = body({});
    expect(bodyMatchesFilter(rotor, "ROT")).toBe(true);
    expect(bodyMatchesFilter(rotor, "rotor_1")).toBe(true);
    expect(bodyMatchesFilter(rotor, "motor")).toBe(true);
    expect(bodyMatchesFilter(rotor, "stator")).toBe(false);
  });
});
