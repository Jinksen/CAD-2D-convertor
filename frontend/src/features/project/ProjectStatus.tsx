import { useWorkspaceStore } from "../../app/workspaceStore";
import { useGlobalShortcuts } from "../step-import/useGlobalShortcuts";
import { saveProject, useProjectStore } from "./projectActions";

/** Registers project and undo/redo shortcuts and shows the last project message. Mount once. */
export function ProjectStatus() {
  const { notice, error, warning } = useProjectStore();
  useGlobalShortcuts((key, ctrl, event) => {
    if (!ctrl) return false;
    const { undoPlane, redoPlane } = useWorkspaceStore.getState();
    if (key === "s") void saveProject(event.shiftKey);
    else if (key === "z" && !event.shiftKey) undoPlane();
    else if (key === "y" || (key === "z" && event.shiftKey)) redoPlane();
    else return false;
    return true;
  });
  const text = error ?? warning ?? notice;
  if (!text) return null;
  return <div role={error ?? warning ? "alert" : "status"}
    style={{ position: "fixed", right: 8, bottom: 28, maxWidth: 480, padding: "6px 10px", zIndex: 20,
      border: "1px solid currentColor", background: "Canvas", color: "CanvasText", fontSize: 12 }}>
    {error ? "✕ " : warning ? "⚠ " : ""}{text}
  </div>;
}
