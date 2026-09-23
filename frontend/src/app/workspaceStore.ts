import { create } from "zustand";

interface WorkspaceState {
  activePlane: "XY" | "XZ" | "YZ";
  setActivePlane: (plane: WorkspaceState["activePlane"]) => void;
}

export const useWorkspaceStore = create<WorkspaceState>((set) => ({
  activePlane: "XY",
  setActivePlane: (activePlane) => {
    set({ activePlane });
  },
}));
