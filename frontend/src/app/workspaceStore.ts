import { create } from "zustand";
import type { ImportSummary } from "../features/step-import/contracts";

interface WorkspaceState {
  activePlane: "XY" | "XZ" | "YZ";
  sectionOffsetMm: number | null;
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
  imported: null,
  selectedComponentId: null,
  setActivePlane: (activePlane) => {
    if (get().activePlane === activePlane) return;
    set({ activePlane, sectionOffsetMm: null });
  },
  setSectionOffsetMm: (sectionOffsetMm) => { set({ sectionOffsetMm }); },
  setImported: (imported) => { set({ imported, selectedComponentId: null, sectionOffsetMm: null }); },
  selectComponent: (selectedComponentId) => { set({ selectedComponentId }); },
}));
