import type { SyntheticEvent } from "react";

import { useNumberFormat } from "../../app/formatStore";
import { useWorkspaceStore } from "../../app/workspaceStore";
import { displayToMm, formatDirection, mmToDisplay } from "../../lib/formatting";
import { QuantityInput } from "../../lib/QuantityInput";
import { planeAxis } from "./plane";
import type { SectionRun } from "./useSectionRun";

const axisLabels = ["x", "y", "z"] as const;

/** Settings-pane form for the section node: plane, offset, compute and export. */
export function SectionSettings({ run }: { run: SectionRun }) {
  const selectedComponentId = useWorkspaceStore((state) => state.selectedComponentId);
  const format = useNumberFormat();
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
          <label htmlFor={`section-normal-${axis}`}>Normal <var>n</var><sub><var>{axis}</var></sub></label>
          <QuantityInput id={`section-normal-${axis}`} value={run.customNormal[index]} disabled={busy}
            decimals={format.directionDecimals} format={format}
            onChange={(value) => { run.setNormalComponent(index, value); }} />
        </span>)}
        <label htmlFor="section-offset">
          {plane === "custom" ? <>Offset along normal <var>d</var></> : <>Offset <var>{planeAxis(plane)}</var></>}
        </label>
        <QuantityInput id="section-offset" value={mmToDisplay(run.offset, format.lengthUnit)} disabled={busy}
          decimals={format.lengthDecimals} format={format} unit={format.lengthUnit}
          onChange={(value) => { run.setOffset(displayToMm(value, format.lengthUnit)); }} />
      </div>
      {plane === "custom" && <div className="form-buttons">
        <button type="button" disabled={busy || !selectedComponentId}
          title={selectedName ? `Use the sheet plane of ${selectedName}` : "Select a body in the model tree or 3D view first"}
          onClick={run.alignToSelectedBody}>{run.aligning ? "Aligning…" : "Align to selected body"}</button>
      </div>}
      <p className="settings-note">{plane === "custom"
        ? <>A custom plane is <var className="vector">n</var> · <var className="vector">p</var> = <var>d</var>.
          Select a sheet and click Align to selected body to cut it in its own plane.</>
        : <>The {plane} plane is <var>{planeAxis(plane)}</var> = const; the initial offset is the model midpoint.</>}</p>
      <div className="form-buttons">
        <button type="submit" className="primary" disabled={busy}>{run.loading ? "Computing…" : "Compute Section"}</button>
      </div>
      {run.error && <p role="alert" className="form-error">{run.error}</p>}
    </form>
    {section && <>
      <h3 className="settings-section">Result</h3>
      <p className="settings-note">{plane === "custom" && <>Unit normal <var className="vector">n</var> = {formatDirection(run.basis?.normal ?? run.customNormal, format)} · </>}
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
