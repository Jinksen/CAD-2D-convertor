import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";

import { useWorkspaceStore } from "../../app/workspaceStore";
import type { ImportSummary } from "../step-import/contracts";
import type { SectionResponse } from "./contracts";
import { SectionViewport } from "./SectionViewport";

const imported: ImportSummary = {
  import_id: "session-1", path: "C:/motor.step", sha256: "abc", source_unit: "mm",
  to_mm_scale: 1, bounds_mm: { min_xyz: [0, 0, 0], max_xyz: [10, 10, 10] }, component_count: 1,
  components: [{ id: "body-1", source_id: null, source_name: "Rotor", display_name: "Rotor",
    export_name: "Rotor", hierarchy_path: ["Rotor"], source_color: [255, 0, 0],
    name_provenance: "product", color_provenance: "shape", body_ordinal: 0,
    bounds_mm: { min_xyz: [0, 0, 0], max_xyz: [10, 10, 10] } }],
  diagnostics: [],
};
const section: SectionResponse = {
  section_id: "section-1", import_id: "session-1", plane: { kind: "XY", offset_mm: 5 }, diagnostics: [],
  components: [{ component_id: "body-1", wires: [{ closed: true, role: "outer", curves: [
    { type: "line", start: [0, 0], end: [10, 0] }, { type: "line", start: [10, 0], end: [10, 10] },
    { type: "line", start: [10, 10], end: [0, 10] }, { type: "line", start: [0, 10], end: [0, 0] },
  ] }] }],
};

function viewBox(canvas: Element) { return (canvas.getAttribute("viewBox") ?? "").split(" ").map(Number); }

beforeEach(() => { useWorkspaceStore.setState({ imported, selectedComponentId: null }); });

describe("SectionViewport", () => {
  it("navigates with focusable keyboard shortcuts", () => {
    render(<SectionViewport section={section} imported={imported} bounds={{ x: 0, y: -10, width: 10, height: 10 }} />);
    const canvas = screen.getByLabelText("2D section geometry");
    expect(canvas).toHaveAttribute("tabindex", "0");
    const fitted = viewBox(canvas);
    const [x, y, width, height] = fitted;

    fireEvent.keyDown(canvas, { key: "ArrowRight" });
    expect(viewBox(canvas)).toEqual([x + width * 0.1, y, width, height]);
    fireEvent.keyDown(canvas, { key: "ArrowUp" });
    expect(viewBox(canvas)[1]).toBeCloseTo(y - height * 0.1);

    fireEvent.keyDown(canvas, { key: "f" });
    expect(viewBox(canvas)).toEqual(fitted);

    fireEvent.keyDown(canvas, { key: "+" });
    expect(viewBox(canvas)[2]).toBeCloseTo(width * 0.8);
    fireEvent.keyDown(canvas, { key: "-" });
    expect(viewBox(canvas)[2]).toBeCloseTo(width);
  });

  it("zooms on wheel without letting the page scroll", () => {
    render(<SectionViewport section={section} imported={imported} bounds={{ x: 0, y: -10, width: 10, height: 10 }} />);
    const canvas = screen.getByLabelText("2D section geometry");
    const width = viewBox(canvas)[2];
    const notCancelled = fireEvent.wheel(canvas, { deltaY: -100 });
    expect(notCancelled).toBe(false);
    expect(viewBox(canvas)[2]).toBeCloseTo(width * 0.9);
  });

  it("still selects a region on a plain click", () => {
    render(<SectionViewport section={section} imported={imported} bounds={{ x: 0, y: -10, width: 10, height: 10 }} />);
    fireEvent.click(screen.getByLabelText("Rotor filled region"));
    expect(useWorkspaceStore.getState().selectedComponentId).toBe("body-1");
  });
});
