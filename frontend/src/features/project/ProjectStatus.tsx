import { useEffect } from "react";

import { useWorkspaceStore } from "../../app/workspaceStore";
import { useGlobalShortcuts } from "../step-import/useGlobalShortcuts";
import { saveProject, useProjectStore } from "./projectActions";

/** How long a plain notice ("Project saved…") stays; warnings and errors stay until replaced. */
export const NOTICE_MS = 6000;

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
  useEffect(() => {
    if (!notice || error || warning) return;
    const timer = setTimeout(() => { useProjectStore.setState({ notice: null }); }, NOTICE_MS);
    return () => { clearTimeout(timer); };
  }, [notice, error, warning]);
  const text = error ?? warning ?? notice;
  if (!text) return null;
  const level = error ? "error" : warning ? "warning" : "info";
  return <div role={level === "info" ? "status" : "alert"} className={`project-status ${level}`}>
    {level === "error" ? "✕ " : level === "warning" ? "⚠ " : ""}{text}
    {level !== "info" && <button type="button" aria-label="Dismiss"
      onClick={() => { useProjectStore.setState({ error: null, warning: null }); }}>×</button>}
  </div>;
}
