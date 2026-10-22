"""Quantity notation in diagnostic messages, following ISO 80000-1 like the frontend.

A value is written as number, no-break space, unit symbol; the minus sign is U+2212 and
values outside 0.001 to 999 999 use a power of ten ("2.5 × 10⁻⁷ mm²"), so tolerances
and residual areas of very different magnitudes stay readable. The decimal marker is a
point because messages are produced before the user's display preferences are known.
"""

import math

NBSP = " "
MINUS = "−"
_SUPERSCRIPT = str.maketrans("-0123456789", "⁻⁰¹²³⁴⁵⁶⁷⁸⁹")


def format_number(value: float, significant: int = 4) -> str:
    if not math.isfinite(value):
        return "—"
    if value == 0:
        return "0"
    sign = MINUS if value < 0 else ""
    magnitude = abs(value)
    exponent = math.floor(math.log10(magnitude))
    mantissa = round(magnitude / 10 ** exponent, significant - 1)
    if mantissa >= 10:  # rounding carried into the next decade
        mantissa /= 10
        exponent += 1
    if -3 <= exponent < 6:
        decimals = max(0, significant - 1 - exponent)
        text = f"{round(magnitude, decimals):.{decimals}f}"
        return sign + (text.rstrip("0").rstrip(".") if "." in text else text)
    digits = f"{mantissa:.{significant - 1}f}".rstrip("0").rstrip(".")
    return f"{sign}{digits} × 10{str(exponent).translate(_SUPERSCRIPT)}"


def format_quantity(value: float, unit: str, significant: int = 4) -> str:
    return f"{format_number(value, significant)}{NBSP}{unit}"
