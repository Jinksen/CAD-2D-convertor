import { describe, expect, it } from "vitest";

import {
  DEFAULT_NUMBER_FORMAT, displayToMm, formatDirection, formatLength, formatLengthProduct, formatNumber,
  parseNumber, type NumberFormat,
} from "./formatting";

const comma: NumberFormat = { ...DEFAULT_NUMBER_FORMAT, decimalMarker: "," };

describe("number notation", () => {
  it("writes a length with a fixed resolution and a no-break space before the unit", () => {
    expect(formatLength(15, DEFAULT_NUMBER_FORMAT)).toBe("15.000 mm");
  });

  it("uses the minus sign and never shows a negative zero", () => {
    expect(formatNumber(-2.5, 1, DEFAULT_NUMBER_FORMAT)).toBe("−2.5");
    expect(formatNumber(-0.0001, 3, DEFAULT_NUMBER_FORMAT)).toBe("0.000");
  });

  it("groups digits in threes with a narrow space, leaving four-digit groups alone", () => {
    expect(formatNumber(1234.5, 1, DEFAULT_NUMBER_FORMAT)).toBe("1234.5");
    expect(formatNumber(1234567.123456, 6, DEFAULT_NUMBER_FORMAT)).toBe("1 234 567.123 456");
    expect(formatNumber(1234567, 0, { ...DEFAULT_NUMBER_FORMAT, digitGrouping: false })).toBe("1234567");
  });

  it("uses a comma decimal marker with semicolon-separated lists", () => {
    expect(formatLength(12.5, comma)).toBe("12,500 mm");
    expect(formatDirection([0, 0.6, -0.8], comma)).toBe("(0,0000; 0,6000; −0,8000)");
    expect(formatDirection([0, 0, 1], DEFAULT_NUMBER_FORMAT)).toBe("(0.0000, 0.0000, 1.0000)");
  });

  it("can drop trailing zeros", () => {
    expect(formatLength(10, { ...DEFAULT_NUMBER_FORMAT, trailingZeros: false })).toBe("10 mm");
    expect(formatLength(10.25, { ...DEFAULT_NUMBER_FORMAT, trailingZeros: false })).toBe("10.25 mm");
  });

  it("writes extents with one shared unit", () => {
    expect(formatLengthProduct([10, 20, 30], DEFAULT_NUMBER_FORMAT)).toBe("(10.000 × 20.000 × 30.000) mm");
  });

  it("converts only the displayed length unit", () => {
    expect(formatLength(25.4, { ...DEFAULT_NUMBER_FORMAT, lengthUnit: "in" })).toBe("1.000 in");
    expect(formatLength(1500, { ...DEFAULT_NUMBER_FORMAT, lengthUnit: "m" })).toBe("1.500 m");
    expect(displayToMm(2, "cm")).toBe(20);
  });

  it("parses either decimal marker, group spaces and the minus sign, and rejects anything else", () => {
    expect(parseNumber("12,5")).toBe(12.5);
    expect(parseNumber(" 1 234.5 ")).toBe(1234.5);
    expect(parseNumber("−3")).toBe(-3);
    expect(parseNumber("1e-3")).toBe(0.001);
    expect(parseNumber("")).toBeNull();
    expect(parseNumber("1,234,5")).toBeNull();
    expect(parseNumber("12 mm")).toBeNull();
  });
});
