import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useWorkspaceStore } from "../../app/workspaceStore";
import type { ImportSummary } from "../step-import/contracts";
import { computeSection } from "./client";
import { SectionView } from "./SectionView";

vi.mock("./client", () => ({ computeSection: vi.fn() }));

const imported: ImportSummary = {
  import_id: "session-1", path: "C:/motor.step", sha256: "abc", source_unit: "mm",
  to_mm_scale: 1, bounds_mm: { min_xyz: [0, 0, 0], max_xyz: [10, 20, 30] },
  component_count: 1,
  components: [{ id: "body-1", source_id: null, source_name: "Rotor", display_name: "Rotor",
    export_name: "Rotor", hierarchy_path: ["Motor", "Rotor"], source_color: [255, 0, 0],
    name_provenance: "product", color_provenance: "shape", body_ordinal: 0,
    bounds_mm: { min_xyz: [0, 0, 0], max_xyz: [10, 20, 30] } }],
  diagnostics: [],
};

beforeEach(() => {
  useWorkspaceStore.setState({ imported, activePlane: "XY", selectedComponentId: null });
  vi.resetAllMocks();
});

describe("SectionView", () => {
  it("requests an exact middle-plane section and draws its analytic line", async () => {
    vi.mocked(computeSection).mockResolvedValue({
      section_id: "section-1", import_id: "session-1",
      plane: { kind: "XY", offset_mm: 15 }, diagnostics: [],
      components: [{ component_id: "body-1", wires: [{ closed: true, role: "outer", curves: [
        { type: "line", start: [0, 0], end: [10, 0] },
      ] }] }],
    });
    render(<SectionView imported={imported} plane="XY" />);
    expect(screen.getByRole("spinbutton", { name: /offset/i })).toHaveValue(15);
    fireEvent.click(screen.getByRole("button", { name: /compute section/i }));
    await waitFor(() => expect(screen.getByLabelText("2D section geometry")).toBeInTheDocument());
    expect(screen.getByLabelText("2D section geometry").querySelector("path")).toHaveAttribute("d", "M 0 0 L 10 0");
    expect(vi.mocked(computeSection)).toHaveBeenCalledWith("session-1", "XY", 15);
  });

  it("shows a section error without losing the imported model", async () => {
    vi.mocked(computeSection).mockRejectedValue(new Error("The section could not be completed."));
    render(<SectionView imported={imported} plane="XY" />);
    fireEvent.click(screen.getByRole("button", { name: /compute section/i }));
    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("The section could not be completed."));
    expect(useWorkspaceStore.getState().imported?.import_id).toBe("session-1");
  });
});
