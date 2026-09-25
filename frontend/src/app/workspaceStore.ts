import { create } from "zustand";
import type { ImportSummary } from "../features/step-import/contracts";

interface WorkspaceState {
  activePlane: "XY" | "XZ" | "YZ";
  setActivePlane: (plane: WorkspaceState["activePlane"]) => void;
  imported: ImportSummary | null;
  selectedComponentId: string | null;
  setImported: (imported: ImportSummary) => void;
  selectComponent: (id: string) => void;
}

export const useWorkspaceStore = create<WorkspaceState>((set) => ({
  activePlane: "XY",
  imported: null,
  selectedComponentId: null,
  setActivePlane: (activePlane) => {
    set({ activePlane });
  },
  setImported: (imported) => { set({ imported, selectedComponentId: null }); },
  selectComponent: (selectedComponentId) => { set({ selectedComponentId }); },
}));
