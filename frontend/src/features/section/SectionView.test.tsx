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
  useWorkspaceStore.setState({ imported, activePlane: "XY", sectionOffsetMm: null, selectedComponentId: null });
  vi.resetAllMocks();
});

describe("SectionView", () => {
  it("shares the edited section offset with the 3D plane guide", () => {
    render(<SectionView imported={imported} plane="XY" />);
    fireEvent.change(screen.getByRole("spinbutton", { name: /offset/i }), { target: { value: "12.5" } });
    expect(useWorkspaceStore.getState().sectionOffsetMm).toBe(12.5);
  });

  it("fills a classified region while leaving its hole empty", async () => {
    vi.mocked(computeSection).mockResolvedValue({
      section_id: "section-hole", import_id: "session-1",
      plane: { kind: "XY", offset_mm: 15 }, diagnostics: [],
      components: [{ component_id: "body-1", wires: [
        { closed: true, role: "outer", curves: [
          { type: "line", start: [0, 0], end: [10, 0] },
          { type: "line", start: [10, 0], end: [10, 10] },
          { type: "line", start: [10, 10], end: [0, 10] },
          { type: "line", start: [0, 10], end: [0, 0] },
        ] },
        { closed: true, role: "hole", curves: [
          { type: "line", start: [2, 2], end: [2, 4] },
          { type: "line", start: [2, 4], end: [4, 4] },
          { type: "line", start: [4, 4], end: [4, 2] },
          { type: "line", start: [4, 2], end: [2, 2] },
        ] },
      ] }],
    });
    render(<SectionView imported={imported} plane="XY" />);
    fireEvent.click(screen.getByRole("button", { name: /compute section/i }));
    const filled = await screen.findByLabelText("Rotor filled region");
    expect(filled).toHaveAttribute("fill-rule", "evenodd");
    expect(filled.getAttribute("d")).toContain("M 0 0 L 10 0 L 10 10 L 0 10 L 0 0 Z");
    expect(filled.getAttribute("d")).toContain("M 2 2 L 2 4 L 4 4 L 4 2 L 2 2 Z");
  });

  it("removes the previous section and export when the plane moves", async () => {
    vi.mocked(computeSection).mockResolvedValue({
      section_id: "section-1", import_id: "session-1",
      plane: { kind: "XY", offset_mm: 15 }, diagnostics: [],
      components: [{ component_id: "body-1", wires: [{ closed: true, role: "outer", curves: [
        { type: "circle", center: [5, 5], radius: 2, x_axis: [1, 0], y_axis: [0, 1],
          start_parameter: 0, end_parameter: Math.PI * 2 },
      ] }] }],
    });
    render(<SectionView imported={imported} plane="XY" />);
    fireEvent.click(screen.getByRole("button", { name: /compute section/i }));
    expect(await screen.findByRole("button", { name: /export draft dxf/i })).toBeInTheDocument();
    fireEvent.change(screen.getByRole("spinbutton", { name: /offset/i }), { target: { value: "12" } });
    expect(screen.queryByRole("button", { name: /export draft dxf/i })).not.toBeInTheDocument();
    expect(screen.queryByLabelText("2D section geometry")).not.toBeInTheDocument();
  });

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
    expect(screen.getByLabelText("2D section geometry").querySelector("path:not([aria-label])"))
      .toHaveAttribute("d", "M 0 0 L 10 0");
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
    expect(screen.queryByLabelText("Rotor filled region")).not.toBeInTheDocument();
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
