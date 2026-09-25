import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { StepImportWorkspace } from "./StepImportWorkspace";
import { useWorkspaceStore } from "../../app/workspaceStore";
import { importStep, importStepFile } from "./client";
import type { ImportSummary } from "./contracts";
import { fetchPreview } from "../preview/client";
import { computeSection } from "../section/client";

vi.mock("./client", () => ({ importStep: vi.fn(), importStepFile: vi.fn() }));
vi.mock("../preview/client", () => ({ fetchPreview: vi.fn() }));
vi.mock("../section/client", () => ({ computeSection: vi.fn() }));

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
    expect(screen.getAllByText("Rotor", { selector: "dd" })).toHaveLength(3);
    expect(screen.getByText("10 × 20 × 30 mm")).toBeInTheDocument();
  });

  it("keeps the current model when a later import fails", async () => {
    useWorkspaceStore.setState({ imported: summary, selectedComponentId: "body-1" });
    vi.mocked(importStep).mockRejectedValue(new Error("The STEP file was not found."));
    render(<StepImportWorkspace />);
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

    fireEvent.click(screen.getByRole("tab", { name: "3D VIEW" }));

    expect(screen.getByText("Generating 3D preview…")).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "3D VIEW" })).toHaveAttribute("aria-selected", "true");
    fireEvent.click(screen.getByRole("tab", { name: "2D SECTION" }));
    expect(screen.getByRole("tab", { name: "2D SECTION" })).toHaveAttribute("aria-selected", "true");
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
    fireEvent.click(screen.getByRole("button", { name: /compute section/i }));
    await screen.findByRole("button", { name: /export draft dxf/i });

    fireEvent.click(screen.getByRole("tab", { name: "3D VIEW" }));
    fireEvent.click(screen.getByRole("tab", { name: "2D SECTION" }));

    expect(screen.getByRole("button", { name: /export draft dxf/i })).toBeInTheDocument();
    expect(computeSection).toHaveBeenCalledTimes(1);
  });
});
