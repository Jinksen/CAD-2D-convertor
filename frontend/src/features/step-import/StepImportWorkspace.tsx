import { useRef, useState, type SyntheticEvent } from "react";

import { Ribbon } from "../../app/Ribbon";
import { useWorkspaceStore } from "../../app/workspaceStore";
import { SectionView } from "../section/SectionView";
import { SectionMessages, SectionSettings } from "../section/SectionSettings";
import { useSectionRun } from "../section/useSectionRun";
import { PreviewView } from "../preview/PreviewView";
import { importStep, importStepFile } from "./client";
import type { ImportComponent, ImportSummary } from "./contracts";

function dimensions(bounds: { min_xyz: [number, number, number]; max_xyz: [number, number, number] }): string {
  return bounds.max_xyz.map((high, index) => (high - bounds.min_xyz[index]).toLocaleString(undefined, {
    maximumFractionDigits: 3,
  })).join(" × ") + " mm";
}

function rgb(color: [number, number, number] | null): string | undefined {
  return color ? `rgb(${color.map(String).join(",")})` : undefined;
}

/** Bodies grouped by their parent assembly path; one group means no extra tree level. */
function groupByParent(components: ImportComponent[]): [string, ImportComponent[]][] {
  const groups = new Map<string, ImportComponent[]>();
  for (const component of components) {
    const parent = component.hierarchy_path.slice(0, -1).join(" / ");
    groups.set(parent, [...(groups.get(parent) ?? []), component]);
  }
  return [...groups.entries()];
}

function BodySettings({ body }: { body: ImportComponent }) {
  return <>
    <div className="settings-heading">
      <strong>Body: {body.display_name}</strong>
    </div>
    <h3 className="settings-section">Identification</h3>
    <dl className="field-list">
      <dt>Export name</dt><dd>{body.export_name}</dd>
      <dt>Source name</dt><dd>{body.source_name ?? "—"}</dd>
      <dt>Assembly path</dt><dd title={body.hierarchy_path.join(" / ")}>{body.hierarchy_path.join(" / ")}</dd>
      <dt>Name origin</dt><dd>{body.name_provenance ?? "Generated"}</dd>
      <dt>Color</dt><dd>{body.source_color ? <>
        <span className="color-swatch" style={{ backgroundColor: rgb(body.source_color) }} />
        {body.source_color.join(" ")} ({body.color_provenance ?? "unknown"})
      </> : "None in source"}</dd>
    </dl>
    <h3 className="settings-section">3D body</h3>
    <dl className="field-list">
      <dt>Size</dt><dd>{dimensions(body.bounds_mm)}</dd>
      <dt>Body ordinal</dt><dd>{body.body_ordinal}</dd>
    </dl>
  </>;
}

interface ModelSettingsProps {
  imported: ImportSummary | null;
  loading: boolean;
  error: string | null;
  onImportPath: (path: string) => void;
  onBrowse: () => void;
}

function ModelSettings({ imported, loading, error, onImportPath, onBrowse }: ModelSettingsProps) {
  const [path, setPath] = useState("");
  function submit(event: SyntheticEvent<HTMLFormElement>) {
    event.preventDefault();
    onImportPath(path.trim());
  }
  return <>
    <div className="settings-heading"><strong>Model</strong></div>
    <h3 className="settings-section">Source file</h3>
    <form className="import-form" onSubmit={submit}>
      <label htmlFor="step-path">STEP path</label>
      <input id="step-path" value={path} onChange={(event) => { setPath(event.target.value); }}
        placeholder="C:\\Projects\\motor.step" required disabled={loading} />
      <div className="form-buttons">
        <button type="submit" className="primary" disabled={loading}>{loading ? "Importing…" : "Import STEP"}</button>
        <button type="button" disabled={loading} onClick={onBrowse}>Browse…</button>
      </div>
      {error && <p role="alert" className="form-error">{error}</p>}
    </form>
    {imported && <>
      <h3 className="settings-section">Imported model</h3>
      <dl className="field-list">
        <dt>Path</dt><dd title={imported.path}>{imported.path}</dd>
        <dt>SHA-256</dt><dd title={imported.sha256}>{imported.sha256.slice(0, 12)}…</dd>
        <dt>Source units</dt><dd>{imported.source_unit ?? "Unknown"}</dd>
        <dt>Bodies</dt><dd>{imported.component_count}</dd>
        <dt>Size</dt><dd>{dimensions(imported.bounds_mm)}</dd>
      </dl>
    </>}
  </>;
}

export function StepImportWorkspace() {
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [viewportMode, setViewportMode] = useState<"2d" | "3d">("2d");
  const [previewOpened, setPreviewOpened] = useState(false);
  const [sectionSelected, setSectionSelected] = useState(false);
  const { activePlane, setActivePlane, imported, selectedComponentId, setImported, selectComponent } = useWorkspaceStore();
  const selected = imported?.components.find((item) => item.id === selectedComponentId);
  const chooser = useRef<HTMLInputElement>(null);
  const run = useSectionRun(imported, activePlane);

  async function load(request: () => Promise<ImportSummary>) {
    setError(null);
    setLoading(true);
    try {
      setImported(await request());
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The STEP file could not be imported.");
    } finally {
      setLoading(false);
    }
  }

  const displayNames = imported?.components.map((item) => item.display_name) ?? [];
  const duplicateNames = new Set(displayNames.filter((name, index) => displayNames.indexOf(name) !== index));

  function bodyButton(item: ImportComponent) {
    const duplicate = duplicateNames.has(item.display_name);
    return <li key={item.id}><button type="button" className="tree-node" aria-pressed={!sectionSelected && selectedComponentId === item.id}
      aria-label={duplicate ? `${item.display_name} ${item.export_name}` : undefined}
      onClick={() => { setSectionSelected(false); selectComponent(item.id); }}>
      <span className="color-swatch" style={{ backgroundColor: rgb(item.source_color) }} />
      {item.display_name}
      {duplicate && <span className="tree-secondary">{item.export_name}</span>}
    </button></li>;
  }

  const groups = imported ? groupByParent(imported.components) : [];
  return (
    <>
      <Ribbon hasModel={Boolean(imported)} activePlane={activePlane} run={run}
        onOpen={() => { chooser.current?.click(); }} onPlane={setActivePlane}
        onCompute={() => { setSectionSelected(true); setViewportMode("2d"); run?.compute(); }}
        onExport={() => { setSectionSelected(true); run?.exportSection(); }} />
      <input ref={chooser} className="visually-hidden" type="file" accept=".step,.stp" aria-label="Choose STEP file"
        tabIndex={-1} disabled={loading}
        onChange={(event) => { const file = event.target.files?.[0]; event.target.value = ""; if (file) void load(() => importStepFile(file)); }} />
      <div className="workspace">
        <nav className="pane model-builder" aria-label="Model builder">
          <div className="pane-title">Model Builder</div>
          {imported ? <div className="tree">
            <button type="button" className="tree-node" aria-pressed={!sectionSelected && !selectedComponentId}
              onClick={() => { setSectionSelected(false); selectComponent(null); }}>
              <svg className="tree-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                strokeWidth="1.6" aria-hidden="true"><path d="M6 3h9l4 4v14H6z" /></svg>
              {imported.path.split(/[\\/]/).pop()}
            </button>
            <details open>
              <summary className="tree-group">Bodies ({imported.component_count})</summary>
              {groups.length === 1 ? <ul className="tree-list">{groups[0][1].map(bodyButton)}</ul> :
                groups.map(([parent, bodies]) => <details open key={parent} className="tree-subgroup">
                  <summary className="tree-group">{parent.split(" / ").pop() || "Root"} ({bodies.length})</summary>
                  <ul className="tree-list">{bodies.map(bodyButton)}</ul>
                </details>)}
            </details>
            <button type="button" className="tree-node" aria-pressed={sectionSelected}
              onClick={() => { setSectionSelected(true); setViewportMode("2d"); }}>
              <svg className="tree-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                strokeWidth="1.6" strokeLinejoin="round" aria-hidden="true"><path d="M3 15 12 10l9 5-9 5z" /></svg>
              Section {activePlane === "custom" ? "(custom plane)" : activePlane}
            </button>
          </div> : <div className="empty-panel"><strong>No model loaded</strong><span>Open a STEP file to inspect its bodies.</span></div>}
        </nav>
        <section className="pane settings" aria-label="Settings">
          <div className="pane-title">Settings</div>
          <div className="pane-body">
            {sectionSelected && run ? <SectionSettings run={run} /> : selected ? <BodySettings body={selected} /> :
              <ModelSettings imported={imported} loading={loading} error={error}
                onImportPath={(path) => { void load(() => importStep(path)); }}
                onBrowse={() => { chooser.current?.click(); }} />}
          </div>
        </section>
        <main className="pane graphics" aria-label="Graphics">
          <div className="pane-title graphics-title">
            <span>Graphics</span>
            <div className="view-tabs" role="tablist" aria-label="Graphics mode">
              <button type="button" role="tab" aria-selected={Boolean(imported) && viewportMode === "2d"}
                disabled={!imported} onClick={() => { setViewportMode("2d"); }}>2D Section</button>
              <button type="button" role="tab" aria-selected={Boolean(imported) && viewportMode === "3d"}
                disabled={!imported} onClick={() => { setPreviewOpened(true); setViewportMode("3d"); }}>3D Preview</button>
            </div>
          </div>
          {imported && run ? <>
            <div className="graphics-panel" hidden={viewportMode !== "2d"}>
              <SectionView run={run} />
            </div>
            {previewOpened && <div className="graphics-panel" hidden={viewportMode !== "3d"}>
              <PreviewView key={imported.import_id} imported={imported} />
            </div>}
          </> :
            <div className="graphics-empty">
              <p>Exact CAD geometry will appear here after import.</p>
            </div>}
        </main>
      </div>
      <section className="pane messages" aria-label="Messages">
        <div className="pane-title">Messages</div>
        <div className="messages-list">
          {imported && imported.diagnostics.length > 0 ? imported.diagnostics.map((item, index) =>
            <p key={`${item.code}-${String(index)}`} className={`message ${item.severity}`}>
              <span className="message-severity">{item.severity}</span>{item.message}
            </p>) : <p className="message">{imported ? "Import completed without messages." : "No messages."}</p>}
          {run && <SectionMessages run={run} />}
        </div>
      </section>
    </>
  );
}
