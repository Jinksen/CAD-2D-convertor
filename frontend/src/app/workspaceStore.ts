import { create } from "zustand";
import type { ImportSummary } from "../features/step-import/contracts";
import type { SectionResponse } from "../features/section/contracts";

export interface SectionStats { regions: number; errors: number; warnings: number }

interface WorkspaceState {
  activePlane: "XY" | "XZ" | "YZ";
  sectionOffsetMm: number | null;
  sectionStats: SectionStats | null;
  setSectionStats: (section: SectionResponse | null) => void;
  setActivePlane: (plane: WorkspaceState["activePlane"]) => void;
  setSectionOffsetMm: (offset: number) => void;
  imported: ImportSummary | null;
  selectedComponentId: string | null;
  setImported: (imported: ImportSummary) => void;
  selectComponent: (id: string) => void;
}

export const useWorkspaceStore = create<WorkspaceState>((set, get) => ({
  activePlane: "XY",
  sectionOffsetMm: null,
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
    set({ activePlane, sectionOffsetMm: null, sectionStats: null });
  },
  setSectionOffsetMm: (sectionOffsetMm) => { set({ sectionOffsetMm, sectionStats: null }); },
  setImported: (imported) => { set({ imported, selectedComponentId: null, sectionOffsetMm: null, sectionStats: null }); },
  selectComponent: (selectedComponentId) => { set({ selectedComponentId }); },
}));
