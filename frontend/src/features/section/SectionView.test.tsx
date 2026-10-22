import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useFormatStore } from "../../app/formatStore";
import { useWorkspaceStore } from "../../app/workspaceStore";
import type { ImportSummary } from "../step-import/contracts";
import { computeSection, fetchComponentPlane } from "./client";
import { downloadSectionArchive } from "./exportClient";
import type { SectionPlaneKind } from "./plane";
import { SectionMessages, SectionSettings } from "./SectionSettings";
import { SectionView } from "./SectionView";
import { useSectionRun } from "./useSectionRun";

vi.mock("./client", () => ({ computeSection: vi.fn(), fetchComponentPlane: vi.fn() }));
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
  useWorkspaceStore.setState({ imported, activePlane: "XY", sectionOffsetMm: null, selectedComponentId: null, customNormal: [0, 0, 1] });
  useFormatStore.getState().resetFormat();
  vi.resetAllMocks();
});

afterEach(() => { vi.unstubAllGlobals(); });

/** The Settings, Graphics and Messages parts of one section, as the workspace mounts them. */
function SectionPanes({ imported, plane }: { imported: ImportSummary; plane: SectionPlaneKind }) {
  const run = useSectionRun(imported, plane);
  if (!run) return null;
  return <><SectionSettings run={run} /><SectionView run={run} /><SectionMessages run={run} /></>;
}

describe("SectionView", () => {
  it("does not open a stale export dialog after leaving the section", async () => {
    vi.mocked(computeSection).mockResolvedValue({
      section_id: "section-1", import_id: "session-1", plane: { kind: "XY", offset_mm: 15 },
      diagnostics: [], components: [{ component_id: "body-1", wires: [
        { closed: true, role: "outer", curves: [{ type: "line", start: [0, 0], end: [10, 0] }] },
      ] }],
    });
    let complete: (blob: Blob) => void = () => { throw new Error("Download not started"); };
    vi.mocked(downloadSectionArchive).mockImplementation(() => new Promise((resolve) => { complete = resolve; }));
    const invoke = vi.fn();
    vi.stubGlobal("__TAURI__", { core: { invoke } });
    const view = render(<SectionPanes imported={imported} plane="XY" />);
    fireEvent.click(screen.getByRole("button", { name: /compute section/i }));
    fireEvent.click(await screen.findByRole("button", { name: /export draft dxf/i }));
    view.unmount();
    complete(new Blob(["PK"]));
    await waitFor(() => { expect(downloadSectionArchive).toHaveBeenCalledOnce(); });
    expect(invoke).not.toHaveBeenCalled();
  });
  it("explains how to generate a section instead of leaving a blank viewport", () => {
    render(<SectionPanes imported={imported} plane="XY" />);
    expect(screen.getByText("No section computed")).toBeInTheDocument();
    expect(screen.getByText(/choose a plane and offset/i)).toBeInTheDocument();
  });

  it("offers view controls and reports actual closed region counts", async () => {
    vi.mocked(computeSection).mockResolvedValue({
      section_id: "section-1", import_id: "session-1", plane: { kind: "XY", offset_mm: 15 },
      diagnostics: [], components: [{ component_id: "body-1", wires: [
        { closed: true, role: "outer", curves: [{ type: "line", start: [0, 0], end: [10, 0] }] },
      ] }],
    });
    render(<SectionPanes imported={imported} plane="XY" />);
    fireEvent.click(screen.getByRole("button", { name: /compute section/i }));
    expect(await screen.findByRole("button", { name: /fit section/i })).toBeInTheDocument();
    const canvas = screen.getByLabelText("2D section geometry");
    const original = canvas.getAttribute("viewBox");
    fireEvent.click(screen.getByRole("button", { name: /zoom in/i }));
    expect(canvas.getAttribute("viewBox")).not.toBe(original);
    fireEvent.click(screen.getByRole("button", { name: /fit section/i }));
    expect(canvas.getAttribute("viewBox")).toBe(original);
    expect(screen.getByText(/1 of 1 bodies intersected/)).toBeInTheDocument();
    expect(useWorkspaceStore.getState().sectionStats?.regions).toBe(1);
    fireEvent.change(screen.getByRole("spinbutton", { name: /offset/i }), { target: { value: "12" } });
    expect(useWorkspaceStore.getState().sectionStats).toBeNull();
  });

  it("shares the edited section offset with the 3D plane guide", () => {
    render(<SectionPanes imported={imported} plane="XY" />);
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
    render(<SectionPanes imported={imported} plane="XY" />);
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
    render(<SectionPanes imported={imported} plane="XY" />);
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
    render(<SectionPanes imported={imported} plane="XY" />);
    expect(screen.getByRole("spinbutton", { name: /offset/i })).toHaveValue("15.000");
    fireEvent.click(screen.getByRole("button", { name: /compute section/i }));
    await waitFor(() => expect(screen.getByLabelText("2D section geometry")).toBeInTheDocument());
    expect(screen.getByLabelText("2D section geometry").querySelector("path:not([aria-label])"))
      .toHaveAttribute("d", "M 0 0 L 10 0");
    expect(vi.mocked(computeSection)).toHaveBeenCalledWith("session-1", { kind: "XY", offset_mm: 15 });
  });

  it("shows a section error without losing the imported model", async () => {
    vi.mocked(computeSection).mockRejectedValue(new Error("The section could not be completed."));
    render(<SectionPanes imported={imported} plane="XY" />);
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
    render(<SectionPanes imported={imported} plane="XY" />);
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
    render(<SectionPanes imported={imported} plane="XY" />);
    fireEvent.click(screen.getByRole("button", { name: /compute section/i }));

    expect(await screen.findByText("Open wire")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /export draft dxf/i })).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Rotor filled region")).not.toBeInTheDocument();
  });

  it("reports the native saved path and prevents recomputation while saving", async () => {
    vi.mocked(computeSection).mockResolvedValue({
      section_id: "section-1", import_id: "session-1", plane: { kind: "XY", offset_mm: 15 },
      diagnostics: [], components: [{ component_id: "body-1", wires: [
        { closed: true, role: "outer", curves: [{ type: "line", start: [0, 0], end: [10, 0] }] },
      ] }],
    });
    vi.mocked(downloadSectionArchive).mockResolvedValue(new Blob(["PK"]));
    let finishSave: (value: string | null) => void = () => { throw new Error("Save was not started"); };
    const invoke = vi.fn(() => new Promise<string | null>((resolve) => { finishSave = resolve; }));
    vi.stubGlobal("__TAURI__", { core: { invoke } });
    render(<SectionPanes imported={imported} plane="XY" />);
    fireEvent.click(screen.getByRole("button", { name: /compute section/i }));
    fireEvent.click(await screen.findByRole("button", { name: /export draft dxf/i }));
    await waitFor(() => { expect(invoke).toHaveBeenCalledOnce(); });
    expect(screen.getByRole("button", { name: /compute section/i })).toBeDisabled();
    expect(screen.getByRole("spinbutton", { name: /offset/i })).toBeDisabled();
    finishSave("C:\\Exports\\section.zip");
    expect(await screen.findByRole("status")).toHaveTextContent("C:\\Exports\\section.zip");
    expect(screen.getByRole("status")).toHaveTextContent("Extract section.dxf and section.json");
    expect(screen.getByRole("button", { name: /compute section/i })).toBeEnabled();
    fireEvent.change(screen.getByRole("spinbutton", { name: /offset/i }), { target: { value: "12" } });
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
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
    render(<SectionPanes imported={withStator} plane="XY" />);
    fireEvent.click(screen.getByRole("button", { name: /compute section/i }));
    expect(await screen.findByText("Rotor and Stator overlap by 5 mm².")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Select Stator" }));
    expect(useWorkspaceStore.getState().selectedComponentId).toBe("body-2");
  });

  it("aligns a custom plane to the selected body and cuts it there", async () => {
    useWorkspaceStore.setState({ activePlane: "custom", selectedComponentId: "body-1" });
    vi.mocked(fetchComponentPlane).mockResolvedValue({
      component_id: "body-1", normal_xyz: [0, 0, 1], offset_mm: 7.5, planar_area_mm2: 400,
    });
    vi.mocked(computeSection).mockResolvedValue({
      section_id: "section-1", import_id: "session-1",
      plane: { kind: "custom", origin_xyz: [0, 0, 7.5], normal_xyz: [0, 0, 1], x_dir_xyz: [1, 0, 0] },
      diagnostics: [], components: [{ component_id: "body-1", wires: [
        { closed: true, role: "outer", curves: [{ type: "line", start: [0, 0], end: [10, 0] }] },
      ] }],
    });
    render(<SectionPanes imported={imported} plane="custom" />);
    fireEvent.click(screen.getByRole("button", { name: /align to selected body/i }));
    await waitFor(() => { expect(useWorkspaceStore.getState().sectionOffsetMm).toBe(7.5); });
    expect(screen.getByRole("spinbutton", { name: /offset along normal/i })).toHaveValue("7.500");
    fireEvent.click(screen.getByRole("button", { name: /compute section/i }));
    expect(await screen.findByText(/1 of 1 bodies intersected/)).toBeInTheDocument();
    expect(computeSection).toHaveBeenCalledWith("session-1", {
      kind: "custom", origin_xyz: [0, 0, 7.5], normal_xyz: [0, 0, 1], x_dir_xyz: [1, 0, 0],
    });
  });

  it("requires a selected body before aligning", () => {
    useWorkspaceStore.setState({ activePlane: "custom" });
    render(<SectionPanes imported={imported} plane="custom" />);
    expect(screen.getByRole("button", { name: /align to selected body/i })).toBeDisabled();
  });

  it("refuses a zero normal instead of computing a guessed plane", () => {
    useWorkspaceStore.setState({ activePlane: "custom", customNormal: [0, 0, 0] });
    render(<SectionPanes imported={imported} plane="custom" />);
    fireEvent.click(screen.getByRole("button", { name: /compute section/i }));
    expect(screen.getByRole("alert")).toHaveTextContent(/normal must not be zero/i);
    expect(computeSection).not.toHaveBeenCalled();
  });

  it("discards the computed section when the plane kind changes", async () => {
    vi.mocked(computeSection).mockResolvedValue({
      section_id: "section-1", import_id: "session-1", plane: { kind: "XY", offset_mm: 15 },
      diagnostics: [], components: [{ component_id: "body-1", wires: [
        { closed: true, role: "outer", curves: [{ type: "line", start: [0, 0], end: [10, 0] }] },
      ] }],
    });
    const view = render(<SectionPanes imported={imported} plane="XY" />);
    fireEvent.click(screen.getByRole("button", { name: /compute section/i }));
    expect(await screen.findByLabelText("2D section geometry")).toBeInTheDocument();

    view.rerender(<SectionPanes imported={imported} plane="XZ" />);

    expect(screen.queryByLabelText("2D section geometry")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /export draft dxf/i })).not.toBeInTheDocument();
    expect(screen.getByText("No section computed")).toBeInTheDocument();
  });

  it("rounds the default midpoint offset to a micrometre but keeps typed offsets exact", () => {
    const odd: ImportSummary = { ...imported, bounds_mm: { min_xyz: [0, 0, 0.3], max_xyz: [10, 20, 220.147016] } };
    render(<SectionPanes imported={odd} plane="XY" />);
    expect(screen.getByRole("spinbutton", { name: /offset/i })).toHaveValue("110.224");
    fireEvent.change(screen.getByRole("spinbutton", { name: /offset/i }), { target: { value: "110.2235081" } });
    expect(useWorkspaceStore.getState().sectionOffsetMm).toBe(110.2235081);
  });

  it("accepts the user's decimal comma and keeps invalid text out of the plane", () => {
    useFormatStore.getState().setFormat({ decimalMarker: "," });
    render(<SectionPanes imported={imported} plane="XY" />);
    const offset = screen.getByRole("spinbutton", { name: /offset z/i });
    expect(offset).toHaveValue("15,000");
    fireEvent.change(offset, { target: { value: "12,5" } });
    expect(useWorkspaceStore.getState().sectionOffsetMm).toBe(12.5);
    fireEvent.change(offset, { target: { value: "12,5 mm" } });
    expect(offset).toHaveAttribute("aria-invalid", "true");
    expect(useWorkspaceStore.getState().sectionOffsetMm).toBe(12.5);
    fireEvent.blur(offset);
    expect(offset).toHaveValue("12,500");
  });

  it("shows the offset in the display unit but stores millimetres", () => {
    useFormatStore.getState().setFormat({ lengthUnit: "cm" });
    render(<SectionPanes imported={imported} plane="XY" />);
    const offset = screen.getByRole("spinbutton", { name: /offset/i });
    expect(offset).toHaveValue("1.500");
    expect(screen.getByText("cm")).toBeInTheDocument();
    fireEvent.change(offset, { target: { value: "2" } });
    expect(useWorkspaceStore.getState().sectionOffsetMm).toBe(20);
  });
});
