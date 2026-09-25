import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useWorkspaceStore } from "../../app/workspaceStore";
import type { ImportSummary } from "../step-import/contracts";
import { computeSection } from "./client";
import { downloadSectionArchive } from "./exportClient";
import { SectionView } from "./SectionView";

vi.mock("./client", () => ({ computeSection: vi.fn() }));
vi.mock("./exportClient", () => ({ downloadSectionArchive: vi.fn() }));

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

  it("downloads an exportable section as a draft DXF bundle", async () => {
    vi.mocked(computeSection).mockResolvedValue({
      section_id: "section-1", import_id: "session-1",
      plane: { kind: "XY", offset_mm: 15 }, diagnostics: [],
      components: [{ component_id: "body-1", wires: [{ closed: true, role: "outer", curves: [
        { type: "line", start: [0, 0], end: [10, 0] },
      ] }] }],
    });
    vi.mocked(downloadSectionArchive).mockResolvedValue(new Blob(["zip"], { type: "application/zip" }));
    const createObjectURL = vi.fn().mockReturnValue("blob:section");
    const revokeObjectURL = vi.fn();
    Object.defineProperty(URL, "createObjectURL", { value: createObjectURL, configurable: true });
    Object.defineProperty(URL, "revokeObjectURL", { value: revokeObjectURL, configurable: true });
    const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => { /* download */ });
    render(<SectionView imported={imported} plane="XY" />);
    fireEvent.click(screen.getByRole("button", { name: /compute section/i }));
    const exportButton = await screen.findByRole("button", { name: /export draft dxf/i });
    fireEvent.click(exportButton);

    await waitFor(() => { expect(click).toHaveBeenCalledOnce(); });
    expect(vi.mocked(downloadSectionArchive)).toHaveBeenCalledWith("session-1", "section-1");
    expect(createObjectURL).toHaveBeenCalledOnce();
    expect(revokeObjectURL).toHaveBeenCalledWith("blob:section");
    click.mockRestore();
  });

  it("does not offer export for a section with an open wire", async () => {
    vi.mocked(computeSection).mockResolvedValue({
      section_id: "section-1", import_id: "session-1",
      plane: { kind: "XY", offset_mm: 15 }, diagnostics: [{ code: "open_section_wire", severity: "error",
        message: "Open wire", component_id: "body-1" }],
      components: [{ component_id: "body-1", wires: [{ closed: false, role: null, curves: [
        { type: "line", start: [0, 0], end: [10, 0] },
      ] }] }],
    });
    render(<SectionView imported={imported} plane="XY" />);
    fireEvent.click(screen.getByRole("button", { name: /compute section/i }));

    expect(await screen.findByText("Open wire")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /export draft dxf/i })).not.toBeInTheDocument();
  });

  it("lets the engineer select both components named in an overlap diagnostic", async () => {
    vi.mocked(computeSection).mockResolvedValue({
      section_id: "section-1", import_id: "session-1",
      plane: { kind: "XY", offset_mm: 15 }, components: [],
      diagnostics: [{ code: "overlapping_regions", severity: "error",
        message: "Rotor and Stator overlap by 5 mm².", component_id: "body-1",
        related_component_id: "body-2" }],
    });
    const withStator: ImportSummary = { ...imported, component_count: 2, components: [
      ...imported.components,
      { ...imported.components[0], id: "body-2", display_name: "Stator" },
    ] };
    render(<SectionView imported={withStator} plane="XY" />);
    fireEvent.click(screen.getByRole("button", { name: /compute section/i }));
    expect(await screen.findByText("Rotor and Stator overlap by 5 mm².")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Select Stator" }));
    expect(useWorkspaceStore.getState().selectedComponentId).toBe("body-2");
  });
});
