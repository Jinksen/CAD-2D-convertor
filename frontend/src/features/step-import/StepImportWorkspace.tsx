import { lazy, Suspense, useRef, useState } from "react";
import { useShallow } from "zustand/react/shallow";

import { Ribbon } from "../../app/Ribbon";
import { useWorkspaceStore } from "../../app/workspaceStore";
import { FormatSettings } from "../format/FormatSettings";
import { SectionView } from "../section/SectionView";
import { SectionSettings } from "../section/SectionSettings";
import { useSectionRun } from "../section/useSectionRun";
import { importStep, importStepFile } from "./client";
import type { ImportSummary } from "./contracts";
import { MessagesPane } from "./MessagesPane";
import { BodySettings, ModelSettings } from "./ModelSettings";
import { ModelTree, type SettingsNode } from "./ModelTree";
import { openProjectText } from "../project/projectActions";
import { ProjectStatus } from "../project/ProjectStatus";
import { useGlobalShortcuts } from "./useGlobalShortcuts";

// three.js is only needed once the 3D tab is opened, so it lives in its own chunk.
const PreviewView = lazy(() => import("../preview/PreviewView").then((module) => ({ default: module.PreviewView })));

export function StepImportWorkspace() {
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  // Start of the running import (epoch ms); null while idle.
  const [importStartedAt, setImportStartedAt] = useState<number | null>(null);
  const loading = importStartedAt !== null;
  const importing = useRef<AbortController | null>(null);
  const viewportMode = useWorkspaceStore((state) => state.viewportMode);
  const setViewportMode = useWorkspaceStore((state) => state.setViewportMode);
  const [previewOpened, setPreviewOpened] = useState(false);
  const [node, setNode] = useState<SettingsNode>(null);
  const { activePlane, setActivePlane, imported, selectedComponentId, setImported, selectComponent } = useWorkspaceStore(
    useShallow((state) => ({ activePlane: state.activePlane, setActivePlane: state.setActivePlane,
      imported: state.imported, selectedComponentId: state.selectedComponentId,
      setImported: state.setImported, selectComponent: state.selectComponent })));
  const selected = imported?.components.find((item) => item.id === selectedComponentId);
  const chooser = useRef<HTMLInputElement>(null);
  const run = useSectionRun(imported, activePlane);

  async function load(request: (signal: AbortSignal) => Promise<ImportSummary>) {
    importing.current?.abort();
    const controller = new AbortController();
    importing.current = controller;
    setError(null);
    setNotice(null);
    setImportStartedAt(Date.now());
    try {
      const result = await request(controller.signal);
      if (importing.current === controller) setImported(result);
    } catch (cause) {
      if (importing.current === controller) {
        setError(cause instanceof Error ? cause.message : "The STEP file could not be imported.");
      }
    } finally {
      if (importing.current === controller) {
        importing.current = null;
        setImportStartedAt(null);
      }
    }
  }

  function cancelImport() {
    importing.current?.abort();
    importing.current = null;
    setImportStartedAt(null);
    setNotice("Cancelled.");
  }

  function compute() {
    setNode("section");
    setViewportMode("2d");
    run?.compute();
  }

  function exportSection() {
    setNode("section");
    run?.exportSection();
  }

  function show3d() {
    setPreviewOpened(true);
    setViewportMode("3d");
  }

  useGlobalShortcuts((key, ctrl) => {
    if (ctrl && key === "o") {
      if (!loading) chooser.current?.click();
    } else if (ctrl && key === "enter") {
      if (run && !run.busy) compute();
    } else if (ctrl && key === "e") {
      if (run?.exportable && !run.exporting) exportSection();
    } else if (ctrl && key === "1" && imported) {
      setViewportMode("2d");
    } else if (ctrl && key === "2" && imported) {
      show3d();
    } else if (!ctrl && (key === "1" || key === "2" || key === "3") && imported) {
      setActivePlane((["XY", "XZ", "YZ"] as const)[Number(key) - 1]);
    } else if (!ctrl && key === "escape") {
      selectComponent(null);
    } else {
      return false;
    }
    return true;
  });

  return (
    <>
      <ProjectStatus />
      <Ribbon hasModel={Boolean(imported)} activePlane={activePlane} run={run}
        onOpen={() => { chooser.current?.click(); }} onPlane={setActivePlane}
        onCompute={compute} onExport={exportSection} />
      <input ref={chooser} className="visually-hidden" type="file" accept=".step,.stp,.c2mproj" aria-label="Choose STEP file"
        tabIndex={-1} disabled={loading}
        onChange={(event) => {
          const file = event.target.files?.[0];
          event.target.value = "";
          if (file?.name.toLowerCase().endsWith(".c2mproj")) void file.text().then((text) => openProjectText(text, null));
          else if (file) void load((signal) => importStepFile(file, signal));
        }} />
      <div className="workspace">
        <ModelTree imported={imported} node={node} selectedComponentId={selectedComponentId} activePlane={activePlane}
          onNode={(next) => { setNode(next); if (next === "section") setViewportMode("2d"); }}
          onSelectBody={(id) => { setNode(null); selectComponent(id); }} />
        <section className="pane settings" aria-label="Settings">
          <div className="pane-title">Settings</div>
          <div className="pane-body">
            {node === "format" ? <FormatSettings /> : node === "section" && run ? <SectionSettings run={run} /> :
              selected ? <BodySettings body={selected} /> :
              <ModelSettings imported={imported} loading={loading} error={error}
                onImportPath={(path) => { void load((signal) => importStep(path, signal)); }}
                onBrowse={() => { chooser.current?.click(); }} />}
          </div>
        </section>
        <main className="pane graphics" aria-label="Graphics">
          <div className="pane-title graphics-title">
            <span>Graphics</span>
            <div className="view-tabs" role="tablist" aria-label="Graphics mode">
              <button type="button" role="tab" aria-selected={Boolean(imported) && viewportMode === "2d"}
                disabled={!imported} title="2D section (Ctrl+1)" onClick={() => { setViewportMode("2d"); }}>2D Section</button>
              <button type="button" role="tab" aria-selected={Boolean(imported) && viewportMode === "3d"}
                disabled={!imported} title="3D preview (Ctrl+2)" onClick={show3d}>3D Preview</button>
            </div>
          </div>
          {imported && run ? <>
            <div className="graphics-panel" hidden={viewportMode !== "2d"}>
              <SectionView run={run} />
            </div>
            {previewOpened && <div className="graphics-panel" hidden={viewportMode !== "3d"}>
              <Suspense fallback={<div className="preview-message">Generating 3D preview…</div>}>
                <PreviewView key={imported.import_id} imported={imported} />
              </Suspense>
            </div>}
          </> :
            <div className="graphics-empty">
              <p>Exact CAD geometry will appear here after import.</p>
            </div>}
        </main>
      </div>
      <MessagesPane imported={imported} run={run} importStartedAt={importStartedAt} notice={notice}
        onCancelImport={cancelImport} />
    </>
  );
}
