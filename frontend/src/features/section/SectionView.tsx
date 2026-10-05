import { useEffect, useRef, useState, type SyntheticEvent } from "react";

import { useWorkspaceStore } from "../../app/workspaceStore";
import { saveSectionArchive } from "../desktop/runtime";
import type { ImportSummary } from "../step-import/contracts";
import { computeSection } from "./client";
import { downloadSectionArchive } from "./exportClient";
import type { SectionResponse } from "./contracts";
import { SectionViewport } from "./SectionViewport";

interface SectionViewProps {
  imported: ImportSummary;
  plane: "XY" | "XZ" | "YZ";
}

export function SectionView({ imported, plane }: SectionViewProps) {
  const axes = plane === "XY" ? [0, 1, 2] : plane === "XZ" ? [0, 2, 1] : [1, 2, 0];
  const [xAxis, yAxis, offsetAxis] = axes;
  const defaultOffset = (imported.bounds_mm.min_xyz[offsetAxis] + imported.bounds_mm.max_xyz[offsetAxis]) / 2;
  const offsetOverride = useWorkspaceStore((state) => state.sectionOffsetMm);
  const setSectionOffsetMm = useWorkspaceStore((state) => state.setSectionOffsetMm);
  const offset = offsetOverride ?? defaultOffset;
  const setSectionStats = useWorkspaceStore((state) => state.setSectionStats);
  const active = useRef(true);
  useEffect(() => {
    active.current = true;
    return () => { active.current = false; };
  }, []);
  const [section, setSection] = useState<SectionResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [exportLocation, setExportLocation] = useState<string | null>(null);
  const selectComponent = useWorkspaceStore((state) => state.selectComponent);

  async function submit(event: SyntheticEvent<HTMLFormElement>) {
    event.preventDefault();
    setLoading(true);
    setError(null);
    setSection(null);
    setSectionStats(null);
    setExportLocation(null);
    try {
      const result = await computeSection(imported.import_id, plane, offset);
      if (!active.current) return;
      setSection(result);
      setSectionStats(result);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The exact section could not be completed.");
    } finally {
      setLoading(false);
    }
  }

  async function exportSection() {
    if (!section) return;
    setError(null);
    setExporting(true);
    setExportLocation(null);
    try {
      const bundle = await downloadSectionArchive(imported.import_id, section.section_id);
      if (!active.current) return;
      setExportLocation(await saveSectionArchive(bundle));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The DXF could not be exported.");
    } finally {
      setExporting(false);
    }
  }

  const exportable = section !== null && section.components.length > 0 &&
    !section.diagnostics.some((item) => item.severity === "error") &&
    section.components.every((component) => component.wires.length > 0 &&
      component.wires.every((wire) => wire.closed && wire.role !== null));

  const xMin = imported.bounds_mm.min_xyz[xAxis];
  const xMax = imported.bounds_mm.max_xyz[xAxis];
  const yMin = imported.bounds_mm.min_xyz[yAxis];
  const yMax = imported.bounds_mm.max_xyz[yAxis];
  return <div className="section-view">
    <form className="section-controls" onSubmit={(event) => { void submit(event); }}>
      <label htmlFor="section-offset">{plane} offset (mm)</label>
      <input id="section-offset" type="number" step="any" required value={offset}
        onChange={(event) => {
          setSectionOffsetMm(Number(event.target.value));
          setSection(null);
          setSectionStats(null);
          setError(null);
          setExportLocation(null);
        }} disabled={loading || exporting} />
      <button type="submit" disabled={loading || exporting}>{loading ? "Computing…" : "Compute Section"}</button>
      {exportable && <button type="button" disabled={exporting}
        onClick={() => { void exportSection(); }}>{exporting ? "Exporting..." : "Export draft DXF"}</button>}
    </form>
    {exportable && <p className="section-draft">Draft DXF bundle: complex Maxwell import still needs verification.</p>}
    {exportLocation && <p role="status" className="section-draft">
      DXF bundle: {exportLocation}. Extract section.dxf and section.json before importing into Maxwell.
    </p>}
    {error && <p role="alert" className="import-error">{error}</p>}
    {!section && <div className="empty-panel section-placeholder"><strong>{loading ? "Computing exact section…" : "No section computed"}</strong><span>Choose a plane and offset, then click Compute Section.</span><span>{plane} moves along {plane === "XY" ? "Z" : plane === "XZ" ? "Y" : "X"}; the initial offset is the model midpoint.</span></div>}
    {section && <>
      <p className="section-draft">{section.components.length} of {imported.component_count} bodies intersected · {section.components.reduce((sum, component) => sum + component.wires.filter((wire) => wire.closed && wire.role === "outer").length, 0)} closed regions</p>
      <SectionViewport key={section.section_id} section={section} imported={imported}
        bounds={{ x: xMin, y: -yMax, width: xMax - xMin, height: yMax - yMin }} />
      {section.components.length === 0 && <p className="section-empty">The plane does not intersect a body.</p>}
      <div className="section-diagnostics">{section.diagnostics.map((item, index) => <p key={`${item.code}-${String(index)}`}
        className={`section-diagnostic ${item.severity}`}>
        {item.message}
        {[item.component_id, item.related_component_id].filter((id): id is string => Boolean(id))
          .map((id) => <button key={id} type="button" onClick={() => { selectComponent(id); }}>
            Select {imported.components.find((component) => component.id === id)?.display_name ?? id}
          </button>)}
      </p>)}</div>
    </>}
  </div>;
}
