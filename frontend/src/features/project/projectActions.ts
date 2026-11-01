import { create } from "zustand";

import { useFormatStore } from "../../app/formatStore";
import { useWorkspaceStore } from "../../app/workspaceStore";
import { desktopInvoke, isDesktop } from "../desktop/runtime";
import { importStep } from "../step-import/client";
import { parseProject, PROJECT_EXTENSION, PROJECT_VERSION, serializeProject, type ProjectFile } from "./projectFile";

/** Outcome of the last project action, shown by ProjectStatus. */
export const useProjectStore = create<{ path: string | null; notice: string | null; error: string | null; warning: string | null }>(() => (
  { path: null, notice: null, error: null, warning: null }));

function report(patch: { notice?: string; error?: string; warning?: string }) {
  useProjectStore.setState({ notice: null, error: null, warning: null, ...patch });
}

export function currentProject(): ProjectFile | null {
  const { imported, activePlane, customNormal, sectionOffsetMm } = useWorkspaceStore.getState();
  if (!imported) return null;
  return {
    app: "CAD2Maxwell", version: PROJECT_VERSION,
    step: { path: imported.path, sha256: imported.sha256 },
    plane: { kind: activePlane, normal_xyz: customNormal, offset_mm: sectionOffsetMm },
    number_format: useFormatStore.getState().format,
  };
}

function download(text: string, name: string) {
  const url = URL.createObjectURL(new Blob([text], { type: "application/json" }));
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  document.body.append(link);
  try { link.click(); } finally { link.remove(); URL.revokeObjectURL(url); }
}

export async function saveProject(saveAs = false): Promise<void> {
  const project = currentProject();
  if (!project) { report({ error: "Open a STEP file before saving a project." }); return; }
  const text = serializeProject(project);
  try {
    if (!isDesktop()) {
      download(text, `project${PROJECT_EXTENSION}`);
      report({ notice: `project${PROJECT_EXTENSION} (browser download requested)` });
      return;
    }
    const path = await desktopInvoke<string | null>("save_project", {
      contents: text, path: saveAs ? null : useProjectStore.getState().path });
    if (path) { useProjectStore.setState({ path }); report({ notice: `Project saved to ${path}` }); }
  } catch (cause) {
    report({ error: cause instanceof Error ? cause.message : "The project could not be saved." });
  }
}

/** Applies a parsed project: re-imports the STEP, then restores plane and number format. */
export async function applyProject(project: ProjectFile, path: string | null): Promise<void> {
  const imported = await importStep(project.step.path);
  useWorkspaceStore.getState().setImported(imported);
  useWorkspaceStore.getState().restorePlane({
    activePlane: project.plane.kind, customNormal: project.plane.normal_xyz, sectionOffsetMm: project.plane.offset_mm });
  useFormatStore.getState().setFormat(project.number_format);
  useProjectStore.setState({ path });
  if (imported.sha256 !== project.step.sha256) {
    report({ warning: `The STEP file changed since the project was saved (SHA-256 mismatch for ${project.step.path}). Results may differ from the saved session.` });
  } else report({ notice: "Project opened." });
}

function pickBrowserFile(): Promise<string | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = PROJECT_EXTENSION;
    input.onchange = () => { void (input.files?.[0]?.text() ?? Promise.resolve(null)).then(resolve); };
    input.oncancel = () => { resolve(null); };
    input.click();
  });
}

export async function openProject(): Promise<void> {
  try {
    let path: string | null = null;
    let text: string | null;
    if (isDesktop()) {
      const picked = await desktopInvoke<[string, string] | null>("open_project");
      [path, text] = picked ?? [null, null];
    } else text = await pickBrowserFile();
    if (text === null) return;
    await applyProject(parseProject(text), path);
  } catch (cause) {
    report({ error: cause instanceof Error ? cause.message : "The project could not be opened." });
  }
}
