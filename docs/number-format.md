# Number and quantity notation

Displayed values follow ISO 80000-1 and the SI Brochure. The implementation is in
`frontend/src/lib/formatting.ts` (UI) and `backend/src/cad2maxwell_backend/formatting.py`
(diagnostic messages).

| Rule | Example |
|---|---|
| Number, no-break space, unit symbol | `12.500 mm`, `5 mm²` |
| Minus sign U+2212, never a negative zero | `−2.5 mm`, `0.000 mm` |
| Digit groups of three, narrow no-break space; four digits stay ungrouped | `12 345.678 9`, `1234.5` |
| Decimal marker point or comma; with a comma, lists use semicolons | `(0,0000; 0,6000; −0,8000)` |
| A unit applies to one number; products share it in parentheses | `(10.000 × 20.000 × 30.000) mm` |
| Backend messages: 4 significant digits, powers of ten outside 0.001–999 999 | `1 × 10⁻⁴ mm`, `2.5 × 10⁻⁷ mm²` |
| Quantity symbols in italic, vectors in bold italic (ISO 80000-2) | *z* = const, ***n*** · ***p*** = *d* |

## User settings

**Model Builder → Units and Number Format** sets:

| Setting | Default |
|---|---|
| Display length unit (mm, cm, m, in) | mm |
| Length decimal places (0–9) | 3, a resolution of 1 µm |
| Direction (unit vector) decimal places | 4 |
| Decimal marker | point |
| Digit grouping | on |
| Trailing zeros | kept |

The settings are stored on the local machine. They change only how values are displayed and typed.
Geometry, API requests and the DXF export always use exact millimetre values, and rounding is
for display only. Input fields accept either decimal marker, group spaces, U+2212 and exponents
(`1e-3`). Text that is not a number is marked invalid and is never committed.
Backend diagnostic messages always use a decimal point, because they are produced before
the user's display preferences are known.
