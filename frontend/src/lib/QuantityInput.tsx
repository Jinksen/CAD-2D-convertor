import { useState } from "react";

import { formatNumber, parseNumber, type NumberFormat } from "./formatting";

interface QuantityInputProps {
  id: string;
  /** The value in the unit shown beside the field. */
  value: number;
  decimals: number;
  format: NumberFormat;
  unit?: string;
  disabled?: boolean;
  onChange: (value: number) => void;
}

/**
 * A numeric field written in the user's notation. Typing keeps the raw text; only a valid
 * number is committed, and the field shows the formatted value again once it loses focus.
 */
export function QuantityInput({ id, value, decimals, format, unit, disabled, onChange }: QuantityInputProps) {
  const [draft, setDraft] = useState<string | null>(null);
  const invalid = draft !== null && parseNumber(draft) === null;
  const input = <input id={id} type="text" inputMode="decimal" role="spinbutton" required autoComplete="off"
    aria-valuenow={value} aria-invalid={invalid || undefined} disabled={disabled}
    value={draft ?? formatNumber(value, decimals, format)} title={String(value)}
    onChange={(event) => {
      setDraft(event.target.value);
      const parsed = parseNumber(event.target.value);
      if (parsed !== null) onChange(parsed);
    }}
    onBlur={() => { setDraft(null); }} />;
  return unit ? <span className="quantity-input">{input}<span className="quantity-unit">{unit}</span></span> : input;
}
