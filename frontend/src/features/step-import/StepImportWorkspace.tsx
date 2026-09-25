import { useState, type SyntheticEvent } from "react";

import { useWorkspaceStore } from "../../app/workspaceStore";
import { SectionView } from "../section/SectionView";
import { PreviewView } from "../preview/PreviewView";
import { importStep, importStepFile } from "./client";

function dimensions(bounds: { min_xyz: [number, number, number]; max_xyz: [number, number, number] }): string {
  return bounds.max_xyz.map((high, index) => (high - bounds.min_xyz[index]).toLocaleString(undefined, {
    maximumFractionDigits: 3,
  })).join(" × ") + " mm";
}

export function StepImportWorkspace() {
  const [path, setPath] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [viewportMode, setViewportMode] = useState<"2d" | "3d">("2d");
  const [previewOpened, setPreviewOpened] = useState(false);
  const { activePlane, imported, selectedComponentId, setImported, selectComponent } = useWorkspaceStore();
  const selected = imported?.components.find((item) => item.id === selectedComponentId);

  async function submit(event: SyntheticEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setLoading(true);
    try {
      setImported(await importStep(path.trim()));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The STEP file could not be imported.");
    } finally {
      setLoading(false);
    }
  }

  async function chooseFile(file: File | undefined) {
    if (!file) return;
    setError(null);
    setLoading(true);
    try {
      setImported(await importStepFile(file));
      setPath("");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The STEP file could not be imported.");
    } finally {
      setLoading(false);
    }
  }

  return (
    <>
      <form className="import-form" onSubmit={(event) => { void submit(event); }}>
        <label htmlFor="step-path">STEP path</label>
        <div className="import-controls">
          <input id="step-path" value={path} onChange={(event) => { setPath(event.target.value); }}
            placeholder="C:\\Projects\\motor.step" required disabled={loading} />
          <label className="file-chooser">Choose STEP file
            <input type="file" accept=".step,.stp" disabled={loading}
              onChange={(event) => { void chooseFile(event.target.files?.[0]); event.target.value = ""; }} />
          </label>
          <button type="submit" disabled={loading}>{loading ? "Importing…" : "Import STEP"}</button>
        </div>
        {error && <p role="alert" className="import-error">{error}</p>}
      </form>
      <div className="workspace">
        <nav className="panel model-tree" aria-label="Model tree">
          <div className="panel-title">MODEL TREE {imported && `· ${String(imported.component_count)} ${imported.component_count === 1 ? "body" : "bodies"}`}</div>
          {imported ? <ul className="component-list">{imported.components.map((item) => (
            <li key={item.id}><button type="button" aria-pressed={selectedComponentId === item.id}
              onClick={() => { selectComponent(item.id); }}>
              <span className="color-swatch" style={{ backgroundColor: item.source_color ? `rgb(${item.source_color.map(String).join(",")})` : undefined }} />
              {item.display_name}
            </button></li>
          ))}</ul> : <div className="empty-panel"><strong>No model loaded</strong><span>Enter an absolute STEP path to inspect components.</span></div>}
        </nav>
        <main className="viewport" aria-label="Viewport">
          <div className="viewport-tabs" role="tablist" aria-label="Viewport mode">
            <button type="button" role="tab" aria-selected={!imported || viewportMode === "3d"}
              disabled={!imported} onClick={() => { setPreviewOpened(true); setViewportMode("3d"); }}>3D VIEW</button>
            <button type="button" role="tab" aria-selected={Boolean(imported) && viewportMode === "2d"}
              disabled={!imported} onClick={() => { setViewportMode("2d"); }}>2D SECTION</button>
          </div>
          {imported ? <>
            <div className="viewport-panel" hidden={viewportMode !== "2d"}>
              <SectionView key={`${imported.import_id}-${activePlane}`} imported={imported} plane={activePlane} />
            </div>
            {previewOpened && <div className="viewport-panel" hidden={viewportMode !== "3d"}>
              <PreviewView key={imported.import_id} imported={imported} />
            </div>}
          </> :
            <div className="viewport-empty"><h1>Engineering viewport</h1>
              <p>Exact CAD geometry will appear here after import.</p>
            </div>}
        </main>
        <aside className="panel properties" aria-label="Properties">
          <div className="panel-title">PROPERTIES</div>
          <dl>
            <dt>Selection</dt><dd>{selected?.display_name ?? "None"}</dd>
            <dt>Source name</dt><dd>{selected?.source_name ?? "—"}</dd>
            <dt>Export name</dt><dd>{selected?.export_name ?? "—"}</dd>
            <dt>Hierarchy</dt><dd>{selected?.hierarchy_path.join(" / ") ?? "—"}</dd>
            <dt>Bounds</dt><dd>{selected ? dimensions(selected.bounds_mm) : "—"}</dd>
            <dt>Source units</dt><dd>{imported?.source_unit ?? "Unknown"}</dd>
            <dt>Section plane</dt><dd>{activePlane}</dd>
          </dl>
        </aside>
      </div>
      {imported && imported.diagnostics.length > 0 && <section className="import-diagnostics" aria-label="Import diagnostics">
        {imported.diagnostics.map((item, index) => <div key={`${item.code}-${String(index)}`} className={item.severity}>{item.message}</div>)}
      </section>}
    </>
  );
}
