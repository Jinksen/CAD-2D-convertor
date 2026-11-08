import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useWorkspaceStore } from "../../app/workspaceStore";
import type { ImportSummary } from "./contracts";
import { ModelTree } from "./ModelTree";

const imported = {
  import_id: "session-1", path: "C:/motor.step", sha256: "abc", source_unit: "mm",
  to_mm_scale: 1, bounds_mm: { min_xyz: [0, 0, 0], max_xyz: [10, 20, 30] },
  component_count: 2,
  components: [
    { id: "body-1", source_id: null, source_name: "Rotor", display_name: "Rotor",
      export_name: "Rotor", hierarchy_path: ["Motor", "Rotor"], source_color: [255, 0, 0],
      name_provenance: "product", color_provenance: "shape", body_ordinal: 0,
      bounds_mm: { min_xyz: [0, 0, 0], max_xyz: [10, 20, 30] } },
    { id: "body-2", source_id: null, source_name: "Stator", display_name: "Stator",
      export_name: "Stator", hierarchy_path: ["Motor", "Housing", "Stator"], source_color: [0, 0, 255],
      name_provenance: "product", color_provenance: "shape", body_ordinal: 1,
      bounds_mm: { min_xyz: [0, 0, 0], max_xyz: [10, 20, 30] } },
  ],
  diagnostics: [],
} satisfies ImportSummary;

beforeEach(() => {
  useWorkspaceStore.setState({ imported, selectedComponentId: null, hiddenIds: [] });
});

function tree() {
  return render(<ModelTree imported={imported} node={null} selectedComponentId={null}
    activePlane="XY" onNode={vi.fn()} onSelectBody={vi.fn()} />);
}

describe("ModelTree filter", () => {
  it("hides bodies that do not match and reports how many remain", () => {
    tree();
    expect(screen.getByText("Bodies (2)")).toBeInTheDocument();
    fireEvent.change(screen.getByRole("searchbox", { name: /filter bodies/i }), { target: { value: "stat" } });
    expect(screen.getByRole("button", { name: /stator/i })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /rotor/i })).not.toBeInTheDocument();
    expect(screen.getByText("Bodies (1 of 2)")).toBeInTheDocument();
    expect(screen.queryByText("Motor (1)")).not.toBeInTheDocument();
    expect(screen.getByText("Housing (1)")).toBeInTheDocument();
  });

  it("says when nothing matches, and Ctrl+F focuses the field", () => {
    tree();
    fireEvent.keyDown(window, { key: "f", ctrlKey: true });
    const field = screen.getByRole("searchbox", { name: /filter bodies/i });
    expect(field).toHaveFocus();
    fireEvent.change(field, { target: { value: "coil" } });
    expect(screen.getByRole("status")).toHaveTextContent("No bodies match this filter.");
  });
});
