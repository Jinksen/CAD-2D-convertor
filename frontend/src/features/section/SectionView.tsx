import { useState, type SyntheticEvent } from "react";

import { useWorkspaceStore } from "../../app/workspaceStore";
import type { ImportSummary } from "../step-import/contracts";
import { computeSection } from "./client";
import type { SectionResponse } from "./contracts";
import { curvePath } from "./rendering";

interface SectionViewProps {
  imported: ImportSummary;
  plane: "XY" | "XZ" | "YZ";
}

export function SectionView({ imported, plane }: SectionViewProps) {
  const axes = plane === "XY" ? [0, 1, 2] : plane === "XZ" ? [0, 2, 1] : [1, 2, 0];
  const [xAxis, yAxis, offsetAxis] = axes;
  const defaultOffset = (imported.bounds_mm.min_xyz[offsetAxis] + imported.bounds_mm.max_xyz[offsetAxis]) / 2;
  const [offset, setOffset] = useState(defaultOffset);
  const [section, setSection] = useState<SectionResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const selectComponent = useWorkspaceStore((state) => state.selectComponent);

  async function submit(event: SyntheticEvent<HTMLFormElement>) {
    event.preventDefault();
    setLoading(true);
    setError(null);
    try {
      setSection(await computeSection(imported.import_id, plane, offset));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The exact section could not be completed.");
    } finally {
      setLoading(false);
    }
  }

  const xMin = imported.bounds_mm.min_xyz[xAxis];
  const xMax = imported.bounds_mm.max_xyz[xAxis];
  const yMin = imported.bounds_mm.min_xyz[yAxis];
  const yMax = imported.bounds_mm.max_xyz[yAxis];
  const span = Math.max(xMax - xMin, yMax - yMin, 1);
  const margin = span * 0.05;
  const viewBox = [xMin - margin, -yMax - margin, xMax - xMin + 2 * margin,
    yMax - yMin + 2 * margin].join(" ");
  const width = Math.max(span / 500, 0.15);

  return <div className="section-view">
    <form className="section-controls" onSubmit={(event) => { void submit(event); }}>
      <label htmlFor="section-offset">{plane} offset (mm)</label>
      <input id="section-offset" type="number" step="any" required value={offset}
        onChange={(event) => { setOffset(Number(event.target.value)); }} disabled={loading} />
      <button type="submit" disabled={loading}>{loading ? "Computing…" : "Compute Section"}</button>
    </form>
    {error && <p role="alert" className="import-error">{error}</p>}
    {section && <>
      <svg className="section-canvas" aria-label="2D section geometry" viewBox={viewBox}>
        <g transform="scale(1,-1)" fill="none" strokeWidth={width}>
          {section.components.flatMap((component) => component.wires.flatMap((wire, wireIndex) =>
            wire.curves.map((curve, curveIndex) => {
              const source = imported.components.find((item) => item.id === component.component_id);
              return <path key={`${component.component_id}-${String(wireIndex)}-${String(curveIndex)}`}
                d={curvePath(curve)} stroke={source?.source_color ? `rgb(${source.source_color.map(String).join(",")})` : "#79c3f5"}
                onClick={() => { selectComponent(component.component_id); }} />;
            }))) }
        </g>
      </svg>
      {section.components.length === 0 && <p className="section-empty">The plane does not intersect a body.</p>}
      {section.diagnostics.map((item, index) => <p key={`${item.code}-${String(index)}`}
        className="section-diagnostic">{item.message}</p>)}
    </>}
  </div>;
}
