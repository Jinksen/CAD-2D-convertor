import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { StepImportWorkspace } from "./StepImportWorkspace";
import { useWorkspaceStore } from "../../app/workspaceStore";
import { importStep, importStepFile } from "./client";
import type { ImportSummary } from "./contracts";
import { fetchPreview } from "../preview/client";
import { computeSection } from "../section/client";
import { downloadSectionArchive } from "../section/exportClient";
import { saveSectionArchive } from "../desktop/runtime";

vi.mock("./client", () => ({ importStep: vi.fn(), importStepFile: vi.fn() }));
vi.mock("../preview/client", () => ({ fetchPreview: vi.fn() }));
vi.mock("../section/client", () => ({ computeSection: vi.fn() }));
vi.mock("../section/exportClient", () => ({ downloadSectionArchive: vi.fn() }));
vi.mock("../desktop/runtime", () => ({ saveSectionArchive: vi.fn() }));

const summary = {
  import_id: "session-1", path: "C:/motor.step", sha256: "abc", source_unit: "mm",
  to_mm_scale: 1, bounds_mm: { min_xyz: [0, 0, 0], max_xyz: [10, 20, 30] },
  component_count: 1,
  components: [{ id: "body-1", source_id: null, source_name: "Rotor", display_name: "Rotor",
    export_name: "Rotor", hierarchy_path: ["Motor", "Rotor"], source_color: [255, 0, 0],
    name_provenance: "product", color_provenance: "shape", body_ordinal: 0,
    bounds_mm: { min_xyz: [0, 0, 0], max_xyz: [10, 20, 30] } }],
  diagnostics: [{ code: "reader_warning", severity: "warning", message: "Partial transfer", component_id: null }],
} satisfies ImportSummary;

beforeEach(() => {
  useWorkspaceStore.setState({ imported: null, selectedComponentId: null });
  vi.resetAllMocks();
});

describe("STEP workspace", () => {
  it("shows imported components and selected source details", async () => {
    vi.mocked(importStep).mockResolvedValue(summary);
    render(<StepImportWorkspace />);
    fireEvent.change(screen.getByRole("textbox", { name: /step path/i }), { target: { value: "C:/motor.step" } });
    fireEvent.click(screen.getByRole("button", { name: /import step/i }));
    await waitFor(() => expect(screen.getByRole("button", { name: /rotor/i })).toBeInTheDocument());
    fireEvent.click(screen.getByRole("button", { name: /rotor/i }));
    expect(screen.getByText("Partial transfer")).toBeInTheDocument();
    expect(screen.getByText("Body: Rotor")).toBeInTheDocument();
    expect(screen.getAllByText("Rotor", { selector: "dd" })).toHaveLength(2);
    expect(screen.getByText(/^\(10\.000 × 20\.000 × 30\.000\)\smm$/)).toBeInTheDocument();
  });

  it("keeps the current model when a later import fails", async () => {
    useWorkspaceStore.setState({ imported: summary, selectedComponentId: "body-1" });
    vi.mocked(importStep).mockRejectedValue(new Error("The STEP file was not found."));
    render(<StepImportWorkspace />);
    fireEvent.click(screen.getByRole("button", { name: "motor.step" }));
    fireEvent.change(screen.getByRole("textbox", { name: /step path/i }), { target: { value: "C:/missing.step" } });
    fireEvent.click(screen.getByRole("button", { name: /import step/i }));
    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("The STEP file was not found."));
    expect(screen.getByRole("button", { name: /rotor/i })).toBeInTheDocument();
  });

  it("imports a file selected through the system chooser", async () => {
    vi.mocked(importStepFile).mockResolvedValue(summary);
    render(<StepImportWorkspace />);
    const chooser = screen.getByLabelText(/choose step file/i);
    fireEvent.change(chooser, { target: { files: [new File(["step"], "motor.step")] } });

    await waitFor(() => expect(screen.getByRole("button", { name: /rotor/i })).toBeInTheDocument());
  });

  it("opens the 3D tab and keeps the 2D section tab available", () => {
    useWorkspaceStore.setState({ imported: summary });
    vi.mocked(fetchPreview).mockReturnValue(new Promise(() => {}));
    render(<StepImportWorkspace />);

    fireEvent.click(screen.getByRole("tab", { name: "3D Preview" }));

    expect(screen.getByText("Generating 3D preview…")).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "3D Preview" })).toHaveAttribute("aria-selected", "true");
    fireEvent.click(screen.getByRole("tab", { name: "2D Section" }));
    expect(screen.getByRole("tab", { name: "2D Section" })).toHaveAttribute("aria-selected", "true");
  });

  it("preserves a computed section while inspecting 3D", async () => {
    useWorkspaceStore.setState({ imported: summary });
    vi.mocked(fetchPreview).mockReturnValue(new Promise(() => {}));
    vi.mocked(computeSection).mockResolvedValue({
      section_id: "section-1", import_id: "session-1",
      plane: { kind: "XY", offset_mm: 15 }, diagnostics: [],
      components: [{ component_id: "body-1", wires: [{ closed: true, role: "outer", curves: [
        { type: "line", start: [0, 0], end: [10, 0] },
      ] }] }],
    });
    render(<StepImportWorkspace />);
    fireEvent.click(screen.getByRole("button", { name: "Section XY" }));
    fireEvent.click(screen.getByRole("button", { name: /compute section/i }));
    await screen.findByRole("button", { name: /export draft dxf/i });

    fireEvent.click(screen.getByRole("tab", { name: "3D Preview" }));
    fireEvent.click(screen.getByRole("tab", { name: "2D Section" }));

    expect(screen.getByRole("button", { name: /export draft dxf/i })).toBeInTheDocument();
    expect(computeSection).toHaveBeenCalledTimes(1);
  });

  it("groups bodies by parent assembly and returns to model settings from the file node", () => {
    const stator = { ...summary.components[0], id: "body-2", display_name: "Stator", export_name: "Stator",
      source_name: "Stator", hierarchy_path: ["Motor", "Housing", "Stator"] };
    useWorkspaceStore.setState({ imported: { ...summary, component_count: 2, components: [summary.components[0], stator] } });
    render(<StepImportWorkspace />);

    expect(screen.getByText("Motor (1)")).toBeInTheDocument();
    expect(screen.getByText("Housing (1)")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /stator/i }));
    expect(screen.getByText("Body: Stator")).toBeInTheDocument();
    expect(screen.getByText("Motor / Housing / Stator")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "motor.step" }));
    expect(useWorkspaceStore.getState().selectedComponentId).toBeNull();
    expect(screen.getByRole("textbox", { name: /step path/i })).toBeInTheDocument();
  });

  it("lists import diagnostics in the messages pane", () => {
    useWorkspaceStore.setState({ imported: summary });
    render(<StepImportWorkspace />);
    expect(screen.getByRole("region", { name: /messages/i })).toHaveTextContent(/warning\s*Partial transfer/i);
  });

  it("shows section controls in Settings and keeps the body selection for alignment", () => {
    useWorkspaceStore.setState({ imported: summary, selectedComponentId: "body-1" });
    render(<StepImportWorkspace />);
    expect(screen.queryByRole("button", { name: /compute section/i })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Section XY" }));

    const settings = screen.getByRole("region", { name: /settings/i });
    expect(settings).toHaveTextContent("Exact planar cut of all bodies");
    expect(screen.getByRole("button", { name: /compute section/i })).toBeInTheDocument();
    expect(screen.getByRole("main", { name: /graphics/i })).toHaveTextContent("No section computed");
    expect(useWorkspaceStore.getState().selectedComponentId).toBe("body-1");
  });

  it("computes and exports the section from the ribbon", async () => {
    useWorkspaceStore.setState({ imported: summary, selectedComponentId: "body-1" });
    vi.mocked(computeSection).mockResolvedValue({
      section_id: "section-1", import_id: "session-1",
      plane: { kind: "XY", offset_mm: 15 }, diagnostics: [],
      components: [{ component_id: "body-1", wires: [{ closed: true, role: "outer", curves: [
        { type: "line", start: [0, 0], end: [10, 0] },
      ] }] }],
    });
    vi.mocked(downloadSectionArchive).mockResolvedValue(new Blob(["PK"]));
    vi.mocked(saveSectionArchive).mockResolvedValue("C:\\Exports\\section.zip");
    render(<StepImportWorkspace />);
    const exportButton = screen.getByRole("button", { name: "DXF" });
    expect(exportButton).toBeDisabled();

    fireEvent.click(screen.getByRole("button", { name: "Compute" }));

    expect(await screen.findByText(/1 of 1 bodies intersected/)).toBeInTheDocument();
    expect(screen.getByRole("region", { name: /settings/i })).toHaveTextContent("Exact planar cut of all bodies");
    expect(computeSection).toHaveBeenCalledWith("session-1", { kind: "XY", offset_mm: 15 });
    expect(exportButton).toBeEnabled();
    fireEvent.click(exportButton);
    expect(await screen.findByRole("status")).toHaveTextContent("C:\\Exports\\section.zip");
    expect(downloadSectionArchive).toHaveBeenCalledWith("session-1", "section-1");
  });

  it("keeps ribbon export disabled for a section with an open wire", async () => {
    useWorkspaceStore.setState({ imported: summary });
    vi.mocked(computeSection).mockResolvedValue({
      section_id: "section-1", import_id: "session-1",
      plane: { kind: "XY", offset_mm: 15 }, diagnostics: [{ code: "open_section_wire", severity: "error",
        message: "Open wire", component_id: "body-1" }],
      components: [{ component_id: "body-1", wires: [{ closed: false, role: null, curves: [
        { type: "line", start: [0, 0], end: [10, 0] },
      ] }] }],
    });
    render(<StepImportWorkspace />);
    fireEvent.click(screen.getByRole("button", { name: "Compute" }));
    expect(await screen.findByText("Open wire")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "DXF" })).toBeDisabled();
  });

  it("shows export names next to duplicate body names", () => {
    const sheet = { ...summary.components[0], display_name: "Plech:1", source_name: "Plech:1" };
    useWorkspaceStore.setState({ imported: { ...summary, component_count: 3, components: [
      { ...sheet, id: "body-1", export_name: "Plech_1" },
      { ...sheet, id: "body-2", export_name: "Plech_1_02" },
      { ...summary.components[0], id: "body-3" },
    ] } });
    render(<StepImportWorkspace />);
    const tree = screen.getByRole("navigation", { name: /model builder/i });
    expect(tree).toHaveTextContent("Plech_1_02");
    expect(screen.getByRole("button", { name: "Plech:1 Plech_1_02" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Rotor" })).toBeInTheDocument();
  });
});
