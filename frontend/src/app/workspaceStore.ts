import { create } from "zustand";
import type { ImportSummary } from "../features/step-import/contracts";
import type { SectionResponse } from "../features/section/contracts";
import type { SectionPlaneKind, Vec3 } from "../features/section/plane";

export interface SectionStats { regions: number; errors: number; warnings: number }

// --- undo/redo history of plane edits (spec 42) ---
export interface PlaneSnapshot { activePlane: SectionPlaneKind; sectionOffsetMm: number | null; customNormal: Vec3 }
/** Edits closer together than this form one undo step, so typing a value is one step. */
export const HISTORY_COALESCE_MS = 600;
const HISTORY_LIMIT = 100;
let lastEditAt = 0;

interface WorkspaceState {
  planeHistory: { past: PlaneSnapshot[]; future: PlaneSnapshot[] };
  undoPlane: () => void;
  redoPlane: () => void;
  /** Replaces the plane without a history step (project open); history is cleared. */
  restorePlane: (plane: PlaneSnapshot) => void;
  activePlane: SectionPlaneKind;
  sectionOffsetMm: number | null;
  customNormal: Vec3;
  sectionStats: SectionStats | null;
  setSectionStats: (section: SectionResponse | null) => void;
  setActivePlane: (plane: WorkspaceState["activePlane"]) => void;
  setSectionOffsetMm: (offset: number) => void;
  setCustomPlane: (normal: Vec3, offset: number | null) => void;
  imported: ImportSummary | null;
  selectedComponentId: string | null;
  setImported: (imported: ImportSummary) => void;
  selectComponent: (id: string | null) => void;
  // T21 diagnostics focus. StepImportWorkspace should read viewportMode from here.
  viewportMode: "2d" | "3d";
  setViewportMode: (mode: "2d" | "3d") => void;
  relatedComponentId: string | null;
  focusRequest: { componentId: string | null; seq: number };
  focusComponent: (id: string, relatedId?: string | null) => void;
  // T22 visibility: display only, never reaches sectioning or export.
  hiddenIds: string[];
  toggleHidden: (id: string) => void;
  toggleIsolate: (id: string) => void;
}

const snapshot = (state: PlaneSnapshot): PlaneSnapshot => ({
  activePlane: state.activePlane, sectionOffsetMm: state.sectionOffsetMm, customNormal: state.customNormal,
});

export const useWorkspaceStore = create<WorkspaceState>((set, get) => {
  /** Call before a plane edit: pushes the current plane unless this edit continues the previous one. */
  function record() {
    const now = Date.now();
    const continuing = now - lastEditAt < HISTORY_COALESCE_MS;
    lastEditAt = now;
    if (continuing) return;
    const { past } = get().planeHistory;
    set({ planeHistory: { past: [...past, snapshot(get())].slice(-HISTORY_LIMIT), future: [] } });
  }
  function travel(from: "past" | "future") {
    const history = get().planeHistory;
    const target = history[from].at(-1);
    if (!target) return;
    const to = from === "past" ? "future" : "past";
    lastEditAt = 0;
    set({ ...target, sectionStats: null, planeHistory: {
      ...history, [from]: history[from].slice(0, -1), [to]: [...history[to], snapshot(get())] } });
  }
  return {
  planeHistory: { past: [], future: [] },
  undoPlane: () => { travel("past"); },
  redoPlane: () => { travel("future"); },
  restorePlane: (plane) => { lastEditAt = 0; set({ ...plane, sectionStats: null, planeHistory: { past: [], future: [] } }); },
  activePlane: "XY",
  sectionOffsetMm: null,
  customNormal: [0, 0, 1],
  sectionStats: null,
  setSectionStats: (section) => {
    if (section && (section.import_id !== get().imported?.import_id || section.plane.kind !== get().activePlane)) return;
    set({ sectionStats: section ? {
      regions: section.components.reduce((sum, component) => sum + component.wires.filter((wire) => wire.closed && wire.role === "outer").length, 0),
      errors: section.diagnostics.filter((item) => item.severity === "error").length,
      warnings: section.diagnostics.filter((item) => item.severity === "warning").length,
    } : null });
  },
  imported: null,
  selectedComponentId: null,
  setActivePlane: (activePlane) => {
    if (get().activePlane === activePlane) return;
    record();
    set({ activePlane, sectionOffsetMm: null, sectionStats: null });
  },
  setSectionOffsetMm: (sectionOffsetMm) => { record(); set({ sectionOffsetMm, sectionStats: null }); },
  setCustomPlane: (customNormal, sectionOffsetMm) => { record(); set({ customNormal, sectionOffsetMm, sectionStats: null }); },
  setImported: (imported) => {
    lastEditAt = 0;
    set({ planeHistory: { past: [], future: [] }, imported, selectedComponentId: null, relatedComponentId: null,
      hiddenIds: [], sectionOffsetMm: null, sectionStats: null });
  },
  selectComponent: (selectedComponentId) => { set({ selectedComponentId, relatedComponentId: null }); },
  viewportMode: "2d",
  setViewportMode: (viewportMode) => { set({ viewportMode }); },
  relatedComponentId: null,
  focusRequest: { componentId: null, seq: 0 },
  focusComponent: (id, relatedId = null) => {
    set((state) => ({ selectedComponentId: id, relatedComponentId: relatedId, viewportMode: "2d",
      focusRequest: { componentId: id, seq: state.focusRequest.seq + 1 } }));
  },
  hiddenIds: [],
  toggleHidden: (id) => {
    set((state) => ({ hiddenIds: state.hiddenIds.includes(id)
      ? state.hiddenIds.filter((item) => item !== id) : [...state.hiddenIds, id] }));
  },
  // Isolate shows only `id`; isolating again restores all bodies.
  toggleIsolate: (id) => {
    const { imported, hiddenIds } = get();
    const others = (imported?.components ?? []).map((item) => item.id).filter((item) => item !== id);
    const isolated = others.length > 0 && others.length === hiddenIds.length && others.every((item) => hiddenIds.includes(item));
    set({ hiddenIds: isolated ? [] : others });
  },
};
});
