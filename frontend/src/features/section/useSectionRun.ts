import { useEffect, useEffectEvent, useRef, useState } from "react";

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
  /** When the running computation started (epoch ms), for the elapsed-time display. */
  startedAt: number | null;
  /** Non-error outcome, such as a cancelled computation. */
  notice: string | null;
  /** A section was computed for this model and plane, so edits recompute it automatically. */
  auto: boolean;
  /** The shown section no longer matches the plane and offset. */
  stale: boolean;
}

function initialState(key: string): RunState {
  return { key, section: null, error: null, loading: false, exporting: false, exportLocation: null, aligning: false,
    startedAt: null, notice: null, auto: false, stale: false };
}

export const RECOMPUTE_DELAY_MS = 300;

export interface SectionRun extends Omit<RunState, "key" | "auto" | "stale"> {
  imported: ImportSummary;
  plane: SectionPlaneKind;
  offset: number;
  customNormal: Vec3;
  basis: PlaneBasis | null;
  busy: boolean;
  exportable: boolean;
  closedRegions: number;
  compute: () => void;
  cancel: () => void;
  setOffset: (offset: number) => void;
  setNormalComponent: (index: number, value: number) => void;
  alignToSelectedBody: () => void;
  exportSection: () => void;
}

/**
 * One exact section of the imported model, shared by the Settings, Graphics and Messages panes.
 * The result is discarded whenever the model or the plane kind changes. Once a section exists,
 * offset and normal edits recompute it after a short debounce; only the newest request is applied.
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
  // Sequence number of the newest section request; responses of older requests are dropped.
  const sequence = useRef(0);
  const controller = useRef<AbortController | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  const key = `${imported?.import_id ?? ""}-${plane}`;
  useEffect(() => () => {
    clearTimeout(timer.current);
    controller.current?.abort();
    sequence.current += 1;
  }, [key]);
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

  const model = imported;
  // The midpoint is an arbitrary start, so it is shown at 1 µm; typed and aligned offsets stay exact.
  const midpoint = (normal: Vec3) => model ? Math.round(defaultOffset(model.bounds_mm, plane, normal) * 1000) / 1000 : 0;
  const offset = offsetOverride ?? midpoint(customNormal);

  /** Stops the pending and the running computation; a late response is ignored. */
  function invalidate() {
    clearTimeout(timer.current);
    controller.current?.abort();
    sequence.current += 1;
  }

  function clear() {
    update({ section: null, error: null, exportLocation: null });
    setSectionStats(null);
  }

  // A debounced edit passes the new values because this render's closure still holds the old ones.
  async function compute(at = offset, normal = customNormal) {
    if (!model) return;
    invalidate();
    const request = sectionPlaneRequest(plane, at, normal);
    if (!request) {
      update({ error: "The plane normal must not be zero.", loading: false, startedAt: null });
      return;
    }
    const abort = new AbortController();
    controller.current = abort;
    const id = sequence.current;
    // The previous section stays visible, marked stale, until the new one arrives.
    update({ loading: true, startedAt: Date.now(), error: null, notice: null, exportLocation: null, stale: true });
    setSectionStats(null);
    try {
      const result = await computeSection(model.import_id, request, abort.signal);
      if (id !== sequence.current) return;
      update({ section: result, stale: false, auto: true, loading: false, startedAt: null });
      setSectionStats(result);
    } catch (cause) {
      if (id !== sequence.current) return;
      update({ section: null, loading: false, startedAt: null,
        error: cause instanceof Error ? cause.message : "The exact section could not be completed." });
    }
  }

  function edited(at: number, normal: Vec3) {
    invalidate();
    if (!state.auto) {
      clear();
      update({ loading: false, startedAt: null });
      return;
    }
    update({ loading: false, startedAt: null, exportLocation: null, notice: null, stale: true });
    timer.current = setTimeout(() => { void compute(at, normal); }, RECOMPUTE_DELAY_MS);
  }

  async function alignToSelectedBody() {
    if (!selectedComponentId || !model) return;
    invalidate();
    clear();
    update({ aligning: true, loading: false, startedAt: null });
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
    if (!section || !model) return;
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

  // Any offset/normal change in the store (typing, undo/redo, project restore) is one edit.
  const onPlaneEdited = useEffectEvent(() => { edited(offset, customNormal); });
  const planeSignature = `${String(offsetOverride)}|${customNormal.join(",")}`;
  const handledSignature = useRef(planeSignature);
  useEffect(() => {
    if (handledSignature.current === planeSignature) return;
    handledSignature.current = planeSignature;
    // Reacting to an external store change (undo, project restore) is what this effect is for.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    if (imported) onPlaneEdited();
  }, [planeSignature, imported]);

  if (!model) return null;
  const { section } = state;
  const exportable = section !== null && !state.stale && section.components.length > 0 &&
    !section.diagnostics.some((item) => item.severity === "error") &&
    section.components.every((component) => component.wires.length > 0 &&
      component.wires.every((wire) => wire.closed && wire.role !== null));

  return {
    section, error: state.error, loading: state.loading, exporting: state.exporting,
    exportLocation: state.exportLocation, aligning: state.aligning, startedAt: state.startedAt, notice: state.notice,
    imported: model, plane, offset, customNormal,
    basis: planeBasis(plane, customNormal),
    busy: state.loading || state.exporting || state.aligning,
    exportable,
    closedRegions: section?.components.reduce((sum, component) =>
      sum + component.wires.filter((wire) => wire.closed && wire.role === "outer").length, 0) ?? 0,
    compute: () => { void compute(); },
    cancel: () => {
      invalidate();
      // Back to idle: the stale section is dropped and edits wait for an explicit Compute again.
      update({ loading: false, startedAt: null, section: null, stale: false, auto: false, notice: "Cancelled." });
      setSectionStats(null);
    },
    setOffset: (value) => { setSectionOffsetMm(value); },
    setNormalComponent: (index, value) => {
      const normal = [...customNormal] as Vec3;
      normal[index] = value;
      setCustomPlane(normal, offsetOverride);
    },
    alignToSelectedBody: () => { void alignToSelectedBody(); },
    exportSection: () => { void exportSection(); },
  };
}
