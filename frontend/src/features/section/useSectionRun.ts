import { useEffect, useRef, useState } from "react";

import { useWorkspaceStore } from "../../app/workspaceStore";
import { saveSectionArchive } from "../desktop/runtime";
import type { ImportSummary } from "../step-import/contracts";
import { computeSection, fetchComponentPlane } from "./client";
import type { SectionResponse } from "./contracts";
import { downloadSectionArchive } from "./exportClient";
import { defaultOffset, planeBasis, sectionPlaneRequest, type PlaneBasis, type SectionPlaneKind, type Vec3 } from "./plane";

interface RunState {
  key: string;
  section: SectionResponse | null;
  error: string | null;
  loading: boolean;
  exporting: boolean;
  exportLocation: string | null;
  aligning: boolean;
}

function initialState(key: string): RunState {
  return { key, section: null, error: null, loading: false, exporting: false, exportLocation: null, aligning: false };
}

export interface SectionRun extends Omit<RunState, "key"> {
  imported: ImportSummary;
  plane: SectionPlaneKind;
  offset: number;
  customNormal: Vec3;
  basis: PlaneBasis | null;
  busy: boolean;
  exportable: boolean;
  closedRegions: number;
  compute: () => void;
  setOffset: (offset: number) => void;
  setNormalComponent: (index: number, value: number) => void;
  alignToSelectedBody: () => void;
  exportSection: () => void;
}

/**
 * One exact section of the imported model, shared by the Settings, Graphics and Messages panes.
 * The result is discarded whenever the model or the plane kind changes.
 */
export function useSectionRun(imported: ImportSummary | null, plane: SectionPlaneKind): SectionRun | null {
  const customNormal = useWorkspaceStore((state) => state.customNormal);
  const setCustomPlane = useWorkspaceStore((state) => state.setCustomPlane);
  const selectedComponentId = useWorkspaceStore((state) => state.selectedComponentId);
  const offsetOverride = useWorkspaceStore((state) => state.sectionOffsetMm);
  const setSectionOffsetMm = useWorkspaceStore((state) => state.setSectionOffsetMm);
  const setSectionStats = useWorkspaceStore((state) => state.setSectionStats);
  const active = useRef(true);
  useEffect(() => {
    active.current = true;
    return () => { active.current = false; };
  }, []);

  const key = `${imported?.import_id ?? ""}-${plane}`;
  const [stored, setStored] = useState(() => initialState(key));
  let state = stored;
  if (stored.key !== key) {
    state = initialState(key);
    setStored(state);
  }
  // Updates from a request started for another model or plane are dropped.
  function update(patch: Partial<RunState>) {
    setStored((current) => current.key === key ? { ...current, ...patch } : current);
  }

  if (!imported) return null;
  const model = imported;
  // The midpoint is an arbitrary start, so it is shown at 1 µm; typed and aligned offsets stay exact.
  const offset = offsetOverride ?? Math.round(defaultOffset(model.bounds_mm, plane, customNormal) * 1000) / 1000;

  function clear() {
    update({ section: null, error: null, exportLocation: null });
    setSectionStats(null);
  }

  async function compute() {
    const request = sectionPlaneRequest(plane, offset, customNormal);
    if (!request) {
      update({ error: "The plane normal must not be zero." });
      return;
    }
    update({ loading: true, error: null, section: null, exportLocation: null });
    setSectionStats(null);
    try {
      const result = await computeSection(model.import_id, request);
      if (!active.current) return;
      update({ section: result });
      setSectionStats(result);
    } catch (cause) {
      update({ error: cause instanceof Error ? cause.message : "The exact section could not be completed." });
    } finally {
      update({ loading: false });
    }
  }

  async function alignToSelectedBody() {
    if (!selectedComponentId) return;
    clear();
    update({ aligning: true });
    try {
      const aligned = await fetchComponentPlane(model.import_id, selectedComponentId);
      if (!active.current) return;
      setCustomPlane(aligned.normal_xyz, aligned.offset_mm);
    } catch (cause) {
      update({ error: cause instanceof Error ? cause.message : "The body plane could not be determined." });
    } finally {
      update({ aligning: false });
    }
  }

  async function exportSection() {
    const section = state.section;
    if (!section) return;
    update({ error: null, exporting: true, exportLocation: null });
    try {
      const bundle = await downloadSectionArchive(model.import_id, section.section_id);
      if (!active.current) return;
      update({ exportLocation: await saveSectionArchive(bundle) });
    } catch (cause) {
      update({ error: cause instanceof Error ? cause.message : "The DXF could not be exported." });
    } finally {
      update({ exporting: false });
    }
  }

  const { section } = state;
  const exportable = section !== null && section.components.length > 0 &&
    !section.diagnostics.some((item) => item.severity === "error") &&
    section.components.every((component) => component.wires.length > 0 &&
      component.wires.every((wire) => wire.closed && wire.role !== null));

  return {
    section, error: state.error, loading: state.loading, exporting: state.exporting,
    exportLocation: state.exportLocation, aligning: state.aligning,
    imported: model, plane, offset, customNormal,
    basis: planeBasis(plane, customNormal),
    busy: state.loading || state.exporting || state.aligning,
    exportable,
    closedRegions: section?.components.reduce((sum, component) =>
      sum + component.wires.filter((wire) => wire.closed && wire.role === "outer").length, 0) ?? 0,
    compute: () => { void compute(); },
    setOffset: (value) => {
      setSectionOffsetMm(value);
      clear();
    },
    setNormalComponent: (index, value) => {
      const normal = [...customNormal] as Vec3;
      normal[index] = value;
      setCustomPlane(normal, offsetOverride);
      clear();
    },
    alignToSelectedBody: () => { void alignToSelectedBody(); },
    exportSection: () => { void exportSection(); },
  };
}
