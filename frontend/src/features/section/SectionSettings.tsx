import type { SyntheticEvent } from "react";

import { useWorkspaceStore } from "../../app/workspaceStore";
import { formatNormal } from "./plane";
import type { SectionRun } from "./useSectionRun";

const axisLabels = ["X", "Y", "Z"] as const;

/** Settings-pane form for the section node: plane, offset, compute and export. */
export function SectionSettings({ run }: { run: SectionRun }) {
  const selectedComponentId = useWorkspaceStore((state) => state.selectedComponentId);
  const { imported, plane, section, busy } = run;
  const selectedName = imported.components.find((component) => component.id === selectedComponentId)?.display_name;

  function submit(event: SyntheticEvent<HTMLFormElement>) {
    event.preventDefault();
    run.compute();
  }

  return <>
    <div className="settings-heading">
      <strong>Section</strong>
      <span className="settings-subtitle">Exact planar cut of all bodies</span>
    </div>
    <form className="section-form" onSubmit={submit}>
      <h3 className="settings-section">Plane</h3>
      <div className="field-grid">
        <span className="field-label">Plane</span>
        <span className="field-value">{plane === "custom" ? "Custom" : plane}</span>
        {plane === "custom" && axisLabels.map((axis, index) => <span className="field-pair" key={axis}>
          <label htmlFor={`section-normal-${axis}`}>Normal {axis}</label>
          <input id={`section-normal-${axis}`} type="number" step="any" required
            value={run.customNormal[index]} disabled={busy}
            onChange={(event) => { run.setNormalComponent(index, Number(event.target.value)); }} />
        </span>)}
        <label htmlFor="section-offset">{plane === "custom" ? "Offset along normal" : `${plane} offset`} (mm)</label>
        <input id="section-offset" type="number" step="any" required value={run.offset} disabled={busy}
          onChange={(event) => { run.setOffset(Number(event.target.value)); }} />
      </div>
      {plane === "custom" && <div className="form-buttons">
        <button type="button" disabled={busy || !selectedComponentId}
          title={selectedName ? `Use the sheet plane of ${selectedName}` : "Select a body in the model tree or 3D view first"}
          onClick={run.alignToSelectedBody}>{run.aligning ? "Aligning…" : "Align to selected body"}</button>
      </div>}
      <p className="settings-note">{plane === "custom"
        ? "A custom plane is n · p = offset. Select a sheet and click Align to selected body to cut it in its own plane."
        : `${plane} moves along ${plane === "XY" ? "Z" : plane === "XZ" ? "Y" : "X"}; the initial offset is the model midpoint.`}</p>
      <div className="form-buttons">
        <button type="submit" className="primary" disabled={busy}>{run.loading ? "Computing…" : "Compute Section"}</button>
      </div>
      {run.error && <p role="alert" className="form-error">{run.error}</p>}
    </form>
    {section && <>
      <h3 className="settings-section">Result</h3>
      <p className="settings-note">{plane === "custom" && `Normal (${formatNormal(run.basis?.normal ?? run.customNormal)}) · `}
        {section.components.length} of {imported.component_count} bodies intersected · {run.closedRegions} closed regions</p>
    </>}
    {(run.exportable || run.exportLocation) && <>
      <h3 className="settings-section">Export</h3>
      <div className="section-form">
        {run.exportable && <>
          <p className="settings-note">Draft DXF bundle: complex Maxwell import still needs verification.</p>
          <div className="form-buttons">
            <button type="button" disabled={run.exporting}
              onClick={run.exportSection}>{run.exporting ? "Exporting..." : "Export draft DXF"}</button>
          </div>
        </>}
        {run.exportLocation && <p role="status" className="settings-note">
          DXF bundle: {run.exportLocation}. Extract section.dxf and section.json before importing into Maxwell.
        </p>}
      </div>
    </>}
  </>;
}

/** Section diagnostics for the Messages pane, with links back to the named bodies. */
export function SectionMessages({ run }: { run: SectionRun }) {
  const selectComponent = useWorkspaceStore((state) => state.selectComponent);
  const { section, imported } = run;
  if (!section) return null;
  return <>
    {section.diagnostics.map((item, index) => <p key={`${item.code}-${String(index)}`}
      className={`message ${item.severity}`}>
      <span className="message-severity">{item.severity}</span>{item.message}
      {[item.component_id, item.related_component_id].filter((id): id is string => Boolean(id))
        .map((id) => <button key={id} type="button" className="message-link" onClick={() => { selectComponent(id); }}>
          Select {imported.components.find((component) => component.id === id)?.display_name ?? id}
        </button>)}
    </p>)}
  </>;
}
