/**
 * Notation of numbers and quantities, following ISO 80000-1 and the SI Brochure:
 * - a value is written as number, no-break space, unit symbol ("12.500 mm"); "°" is written without a space,
 * - the minus sign is U+2212, not a hyphen,
 * - digits may be grouped in threes by a narrow no-break space, never by a comma or a point,
 * - the decimal marker is a point or a comma; with a comma, list items are separated by a semicolon,
 * - a unit applies to one number, so a product of lengths is written "(10 × 20 × 30) mm".
 * Geometry is always stored and exported in millimetres; only the display unit changes.
 */

export type LengthUnit = "mm" | "cm" | "m" | "in";
export type DecimalMarker = "." | ",";

export interface NumberFormat {
  lengthUnit: LengthUnit;
  /** Decimal places of lengths in the display unit. */
  lengthDecimals: number;
  /** Decimal places of dimensionless direction (unit vector) components. */
  directionDecimals: number;
  decimalMarker: DecimalMarker;
  digitGrouping: boolean;
  /** Keep trailing zeros so every value shows the same resolution. */
  trailingZeros: boolean;
}

export const DEFAULT_NUMBER_FORMAT: NumberFormat = {
  lengthUnit: "mm", lengthDecimals: 3, directionDecimals: 4,
  decimalMarker: ".", digitGrouping: true, trailingZeros: true,
};

export const MAX_DECIMALS = 9;

export const lengthUnits: Record<LengthUnit, { name: string; mmPerUnit: number }> = {
  mm: { name: "millimetre", mmPerUnit: 1 },
  cm: { name: "centimetre", mmPerUnit: 10 },
  m: { name: "metre", mmPerUnit: 1000 },
  in: { name: "inch", mmPerUnit: 25.4 },
};

const NBSP = " ";
const GROUP = " ";
const MINUS = "−";

export function mmToDisplay(valueMm: number, unit: LengthUnit): number {
  return valueMm / lengthUnits[unit].mmPerUnit;
}

export function displayToMm(value: number, unit: LengthUnit): number {
  return value * lengthUnits[unit].mmPerUnit;
}

function group(digits: string, fromLeft: boolean): string {
  // ISO 80000-1 leaves four-digit groups ungrouped, as in "1234" or "0.1234".
  if (digits.length <= 4) return digits;
  const parts: string[] = [];
  if (fromLeft) {
    for (let index = 0; index < digits.length; index += 3) parts.push(digits.slice(index, index + 3));
  } else {
    for (let end = digits.length; end > 0; end -= 3) parts.unshift(digits.slice(Math.max(0, end - 3), end));
  }
  return parts.join(GROUP);
}

/** A plain number with a fixed number of decimal places. */
export function formatNumber(value: number, decimals: number, format: NumberFormat): string {
  if (!Number.isFinite(value)) return "—";
  const fixed = Math.abs(value).toFixed(decimals);
  let [integer, fraction = ""] = fixed.split(".");
  if (!format.trailingZeros) fraction = fraction.replace(/0+$/, "");
  const negative = value < 0 && /[1-9]/.test(integer + fraction);
  if (format.digitGrouping) {
    integer = group(integer, false);
    fraction = group(fraction, true);
  }
  return (negative ? MINUS : "") + integer + (fraction ? format.decimalMarker + fraction : "");
}

/** A length given in millimetres, converted to and labelled with the display unit. */
export function formatLength(valueMm: number, format: NumberFormat): string {
  return formatNumber(mmToDisplay(valueMm, format.lengthUnit), format.lengthDecimals, format)
    + NBSP + format.lengthUnit;
}

/** Extents such as a bounding box, sharing one unit: "(10.000 × 20.000 × 30.000) mm". */
export function formatLengthProduct(valuesMm: number[], format: NumberFormat): string {
  const numbers = valuesMm.map((value) =>
    formatNumber(mmToDisplay(value, format.lengthUnit), format.lengthDecimals, format));
  return `(${numbers.join(" × ")})${NBSP}${format.lengthUnit}`;
}

/** The separator of list items, so that a comma decimal marker is never ambiguous. */
export function listSeparator(format: NumberFormat): string {
  return format.decimalMarker === "," ? "; " : ", ";
}

/** A dimensionless direction such as a plane normal: "(0.0000, 0.0000, 1.0000)". */
export function formatDirection(vector: readonly number[], format: NumberFormat): string {
  return `(${vector.map((value) => formatNumber(value, format.directionDecimals, format)).join(listSeparator(format))})`;
}

/**
 * Parses typed input. Either decimal marker, spaces between digit groups, U+2212 and
 * exponents ("1e-3") are accepted; anything else returns null instead of a guessed value.
 */
export function parseNumber(text: string): number | null {
  const normalized = text.trim().replace(/\s/g, "").replace(MINUS, "-").replace(",", ".");
  if (!/^[+-]?(\d+\.?\d*|\.\d+)(e[+-]?\d+)?$/i.test(normalized)) return null;
  const value = Number(normalized);
  return Number.isFinite(value) ? value : null;
}
