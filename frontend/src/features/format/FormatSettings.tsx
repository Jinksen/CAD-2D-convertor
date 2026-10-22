import { useFormatStore } from "../../app/formatStore";
import {
  formatDirection, formatLength, formatLengthProduct, formatNumber, lengthUnits, MAX_DECIMALS,
  type DecimalMarker, type LengthUnit,
} from "../../lib/formatting";

const decimalChoices = Array.from({ length: MAX_DECIMALS + 1 }, (_, index) => index);

/** Settings-pane form for display units and number notation. */
export function FormatSettings() {
  const { format, setFormat, resetFormat } = useFormatStore();
  const resolution = formatLength(10 ** -format.lengthDecimals * lengthUnits[format.lengthUnit].mmPerUnit,
    { ...format, trailingZeros: true });
  return <>
    <div className="settings-heading">
      <strong>Units and Number Format</strong>
      <span className="settings-subtitle">Display notation per ISO 80000-1; geometry and DXF stay in mm</span>
    </div>
    <h3 className="settings-section">Length</h3>
    <div className="field-grid">
      <label htmlFor="format-length-unit">Display unit</label>
      <select id="format-length-unit" value={format.lengthUnit}
        onChange={(event) => { setFormat({ lengthUnit: event.target.value as LengthUnit }); }}>
        {Object.entries(lengthUnits).map(([unit, { name }]) => <option key={unit} value={unit}>{unit} ({name})</option>)}
      </select>
      <label htmlFor="format-length-decimals">Decimal places</label>
      <select id="format-length-decimals" value={format.lengthDecimals}
        onChange={(event) => { setFormat({ lengthDecimals: Number(event.target.value) }); }}>
        {decimalChoices.map((count) => <option key={count} value={count}>{count}</option>)}
      </select>
      <span className="field-label">Resolution</span>
      <span className="field-value">{resolution}</span>
    </div>
    <h3 className="settings-section">Direction vectors</h3>
    <div className="field-grid">
      <label htmlFor="format-direction-decimals">Decimal places</label>
      <select id="format-direction-decimals" value={format.directionDecimals}
        onChange={(event) => { setFormat({ directionDecimals: Number(event.target.value) }); }}>
        {decimalChoices.map((count) => <option key={count} value={count}>{count}</option>)}
      </select>
    </div>
    <h3 className="settings-section">Notation</h3>
    <div className="field-grid">
      <label htmlFor="format-decimal-marker">Decimal marker</label>
      <select id="format-decimal-marker" value={format.decimalMarker}
        onChange={(event) => { setFormat({ decimalMarker: event.target.value as DecimalMarker }); }}>
        <option value=".">Point (12.5)</option>
        <option value=",">Comma (12,5)</option>
      </select>
      <span className="field-pair">
        <input id="format-grouping" type="checkbox" className="field-check" checked={format.digitGrouping}
          onChange={(event) => { setFormat({ digitGrouping: event.target.checked }); }} />
        <label htmlFor="format-grouping" className="field-check-label">Group digits in threes</label>
      </span>
      <span className="field-pair">
        <input id="format-trailing-zeros" type="checkbox" className="field-check" checked={format.trailingZeros}
          onChange={(event) => { setFormat({ trailingZeros: event.target.checked }); }} />
        <label htmlFor="format-trailing-zeros" className="field-check-label">Keep trailing zeros</label>
      </span>
    </div>
    <h3 className="settings-section">Preview</h3>
    <dl className="field-list">
      <dt>Length</dt><dd>{formatLength(-12345.6789, format)}</dd>
      <dt>Size</dt><dd>{formatLengthProduct([120, 85.5, 0.35], format)}</dd>
      <dt>Normal <var>n</var></dt><dd>{formatDirection([Math.SQRT1_2, 0, -Math.SQRT1_2], format)}</dd>
      <dt>Number</dt><dd>{formatNumber(1234567.25, 2, format)}</dd>
    </dl>
    <div className="form-buttons settings-buttons">
      <button type="button" onClick={resetFormat}>Restore defaults</button>
    </div>
    <p className="settings-note">Values are rounded for display only; computation uses the full value, shown when hovering an input.
      Typed input accepts either decimal marker.</p>
  </>;
}
