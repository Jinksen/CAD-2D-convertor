import { useState, type SyntheticEvent } from "react";

import { useNumberFormat } from "../../app/formatStore";
import { formatLengthProduct, listSeparator, type NumberFormat } from "../../lib/formatting";
import { rgb } from "./color";
import type { BoundsMm, ImportComponent, ImportSummary } from "./contracts";

function dimensions(bounds: BoundsMm, format: NumberFormat): string {
  return formatLengthProduct(bounds.max_xyz.map((high, index) => high - bounds.min_xyz[index]), format);
}

export function BodySettings({ body }: { body: ImportComponent }) {
  const format = useNumberFormat();
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
        RGB ({body.source_color.join(listSeparator(format))}) · {body.color_provenance ?? "unknown"}
      </> : "None in source"}</dd>
    </dl>
    <h3 className="settings-section">3D body</h3>
    <dl className="field-list">
      <dt title="Bounding box extents Δx × Δy × Δz">Size</dt><dd>{dimensions(body.bounds_mm, format)}</dd>
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

export function ModelSettings({ imported, loading, error, onImportPath, onBrowse }: ModelSettingsProps) {
  const [path, setPath] = useState("");
  const format = useNumberFormat();
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
        <dt title="Bounding box extents Δx × Δy × Δz">Size</dt><dd>{dimensions(imported.bounds_mm, format)}</dd>
      </dl>
    </>}
  </>;
}
