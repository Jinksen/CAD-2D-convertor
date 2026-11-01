import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useWorkspaceStore } from "../../app/workspaceStore";
import { boundsSphere } from "../preview/cameraViews";
import type { SectionResponse } from "../section/contracts";
import { componentBounds } from "../section/rendering";
import { SectionMessages } from "../section/SectionSettings";
import { SectionViewport } from "../section/SectionViewport";
import type { SectionRun } from "../section/useSectionRun";
import type { ImportComponent, ImportSummary } from "./contracts";
import { MessagesPane } from "./MessagesPane";
import { ModelTree } from "./ModelTree";

const body = (id: string, name: string): ImportComponent => ({ id, source_id: null, source_name: name,
  display_name: name, export_name: name, hierarchy_path: [name], source_color: [255, 0, 0],
  name_provenance: "product", color_provenance: "shape", body_ordinal: 0,
  bounds_mm: { min_xyz: [0, 0, 0], max_xyz: [10, 10, 10] } });
const imported: ImportSummary = {
  import_id: "s1", path: "C:/m.step", sha256: "a", source_unit: "mm", to_mm_scale: 1,
  bounds_mm: { min_xyz: [0, 0, 0], max_xyz: [100, 10, 10] }, component_count: 2,
  components: [body("a", "Rotor"), body("b", "Stator")], diagnostics: [],
};
const square = (x: number, size: number) => ({ closed: true, role: "outer" as const, curves: [
  { type: "line" as const, start: [x, 0] as [number, number], end: [x + size, 0] as [number, number] },
  { type: "line" as const, start: [x + size, 0] as [number, number], end: [x + size, size] as [number, number] },
  { type: "line" as const, start: [x + size, size] as [number, number], end: [x, size] as [number, number] },
  { type: "line" as const, start: [x, size] as [number, number], end: [x, 0] as [number, number] },
] });
const section: SectionResponse = {
  section_id: "x", import_id: "s1", plane: { kind: "XY", offset_mm: 0 },
  diagnostics: [{ code: "overlapping_regions", severity: "error", message: "Rotor and Stator overlap",
    component_id: "a", related_component_id: "b" }],
  components: [{ component_id: "a", wires: [square(0, 10)] }, { component_id: "b", wires: [square(90, 10)] }],
};
const viewBox = (el: Element) => (el.getAttribute("viewBox") ?? "").split(" ").map(Number);

beforeEach(() => {
  useWorkspaceStore.setState({ imported, selectedComponentId: null, relatedComponentId: null, hiddenIds: [],
    viewportMode: "3d", focusRequest: { componentId: null, seq: 0 } });
});

describe("visibility store", () => {
  it("toggles, isolates and restores without touching the model", () => {
    const store = useWorkspaceStore.getState();
    store.toggleHidden("a");
    expect(useWorkspaceStore.getState().hiddenIds).toEqual(["a"]);
    store.toggleHidden("a");
    expect(useWorkspaceStore.getState().hiddenIds).toEqual([]);
    store.toggleIsolate("a");
    expect(useWorkspaceStore.getState().hiddenIds).toEqual(["b"]);
    store.toggleIsolate("a");
    expect(useWorkspaceStore.getState().hiddenIds).toEqual([]);
    expect(useWorkspaceStore.getState().imported?.components).toHaveLength(2);
  });

  it("is reset by a new import", () => {
    useWorkspaceStore.getState().toggleHidden("a");
    useWorkspaceStore.getState().setImported(imported);
    expect(useWorkspaceStore.getState().hiddenIds).toEqual([]);
  });
});

describe("Model Builder visibility", () => {
  it("eye button and V / I keys change visibility", () => {
    render(<ModelTree imported={imported} node={null} selectedComponentId="a" activePlane="XY"
      onNode={vi.fn()} onSelectBody={vi.fn()} />);
    fireEvent.click(screen.getAllByRole("button", { name: "Hide body" })[1]);
    expect(useWorkspaceStore.getState().hiddenIds).toEqual(["b"]);
    expect(screen.getByRole("button", { name: "Show body" })).toBeInTheDocument();
    fireEvent.keyDown(window, { key: "v" });
    expect(useWorkspaceStore.getState().hiddenIds).toEqual(["b", "a"]);
    fireEvent.keyDown(window, { key: "i" });
    expect(useWorkspaceStore.getState().hiddenIds).toEqual(["b"]);
  });
});

describe("2D viewport", () => {
  it("draws hidden bodies as faint outlines only", () => {
    useWorkspaceStore.setState({ hiddenIds: ["b"] });
    const { container } = render(<SectionViewport section={{ ...section, diagnostics: [] }} imported={imported}
      bounds={{ x: 0, y: -10, width: 100, height: 10 }} />);
    expect(screen.queryByLabelText("Stator filled region")).toBeNull();
    expect(screen.getByLabelText("Rotor filled region")).toBeInTheDocument();
    expect(container.querySelectorAll("path[data-hidden]")).toHaveLength(4);
  });

  it("fits the selection with F and everything with Shift+F, from the window", () => {
    render(<SectionViewport section={section} imported={imported} bounds={{ x: 0, y: -10, width: 100, height: 10 }} />);
    const canvas = screen.getByLabelText("2D section geometry");
    const all = viewBox(canvas);
    act(() => { useWorkspaceStore.getState().selectComponent("a"); });
    fireEvent.keyDown(window, { key: "f" });
    expect(viewBox(canvas)[2]).toBeLessThan(all[2] / 4);
    fireEvent.keyDown(window, { key: "F", shiftKey: true });
    expect(viewBox(canvas)).toEqual(all);
  });
});

describe("diagnostics focus (T21)", () => {
  it("selects the body, switches to 2D, highlights the related one and fits the view", () => {
    const run = { section, imported } as unknown as SectionRun;
    const view = render(<SectionViewport section={section} imported={imported}
      bounds={{ x: 0, y: -10, width: 100, height: 10 }} />);
    const canvas = screen.getByLabelText("2D section geometry");
    const all = viewBox(canvas);
    view.rerender(<><SectionViewport section={section} imported={imported}
      bounds={{ x: 0, y: -10, width: 100, height: 10 }} /><SectionMessages run={run} /></>);
    expect(screen.getByText(/✕ ERROR/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Select Rotor" }));
    const state = useWorkspaceStore.getState();
    expect(state.selectedComponentId).toBe("a");
    expect(state.relatedComponentId).toBe("b");
    expect(state.viewportMode).toBe("2d");
    expect(viewBox(screen.getByLabelText("2D section geometry"))[2]).toBeLessThan(all[2] / 4);
  });
});

describe("Messages pane", () => {
  const props = { imported: null, run: null, notice: null, onCancelImport: vi.fn() };

  it("shows No messages only when idle", () => {
    const { rerender } = render(<MessagesPane {...props} importStartedAt={null} />);
    expect(screen.getByText("No messages.")).toBeInTheDocument();
    rerender(<MessagesPane {...props} importStartedAt={Date.now()} />);
    expect(screen.queryByText("No messages.")).toBeNull();
  });

  it("labels severities with icon and text, and Ctrl+L focuses the pane", () => {
    render(<MessagesPane {...props} importStartedAt={null} imported={{ ...imported, diagnostics: [
      { code: "w", severity: "warning", message: "Partial", component_id: null }] }} />);
    expect(screen.getByText("⚠ WARNING")).toBeInTheDocument();
    fireEvent.keyDown(window, { key: "l", ctrlKey: true });
    expect(screen.getByLabelText("Messages")).toHaveFocus();
  });
});

describe("fit maths", () => {
  it("bounds a body, including arcs", () => {
    expect(componentBounds({ component_id: "a", wires: [square(5, 10)] })).toEqual({ x: 5, y: 0, width: 10, height: 10 });
    const bounds = componentBounds({ component_id: "c", wires: [{ closed: true, role: "outer", curves: [{
      type: "circle", center: [0, 0], radius: 2, x_axis: [1, 0], y_axis: [0, 1], start_parameter: 0, end_parameter: 2 * Math.PI }] }] });
    expect(bounds?.width).toBeCloseTo(4);
    expect(bounds?.x).toBeCloseTo(-2);
  });

  it("gives the sphere around a box", () => {
    expect(boundsSphere([0, 0, 0], [2, 4, 4])).toEqual({ center: [1, 2, 2], radius: 3 });
  });
});
