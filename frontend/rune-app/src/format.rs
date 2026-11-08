//! Notation of numbers and quantities, following ISO 80000-1 (port of `frontend/src/lib/formatting.ts`):
//! "12.500 mm" with a no-break space, U+2212 as minus, digits grouped in threes by a narrow
//! no-break space, a point or comma as decimal marker (a comma makes `;` the list separator).
//! Geometry is always stored and exported in millimetres; only the display unit changes.

use serde::{Deserialize, Serialize};

pub const MAX_DECIMALS: u8 = 9;
const NBSP: char = '\u{a0}';
const GROUP: char = '\u{202f}';
const MINUS: char = '\u{2212}';

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LengthUnit {
    #[serde(rename = "mm")]
    Mm,
    #[serde(rename = "cm")]
    Cm,
    #[serde(rename = "m")]
    M,
    #[serde(rename = "in")]
    In,
}

impl LengthUnit {
    pub const ALL: [LengthUnit; 4] = [Self::Mm, Self::Cm, Self::M, Self::In];

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Mm => "mm",
            Self::Cm => "cm",
            Self::M => "m",
            Self::In => "in",
        }
    }

    pub fn mm_per_unit(self) -> f64 {
        match self {
            Self::Mm => 1.0,
            Self::Cm => 10.0,
            Self::M => 1000.0,
            Self::In => 25.4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecimalMarker {
    #[serde(rename = ".")]
    Point,
    #[serde(rename = ",")]
    Comma,
}

/// Field names match the React app's `.c2mproj` files; missing fields take their defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NumberFormat {
    pub length_unit: LengthUnit,
    pub length_decimals: u8,
    pub direction_decimals: u8,
    pub decimal_marker: DecimalMarker,
    pub digit_grouping: bool,
    pub trailing_zeros: bool,
}

impl Default for NumberFormat {
    fn default() -> Self {
        NumberFormat {
            length_unit: LengthUnit::Mm,
            length_decimals: 3,
            direction_decimals: 4,
            decimal_marker: DecimalMarker::Point,
            digit_grouping: true,
            trailing_zeros: true,
        }
    }
}

fn group(digits: &str, from_left: bool) -> String {
    // ISO 80000-1 leaves four-digit groups ungrouped, as in "1234" or "0.1234".
    if digits.len() <= 4 {
        return digits.to_owned();
    }
    let lead = if from_left { 0 } else { digits.len() % 3 };
    let mut out = String::new();
    let mut start = 0;
    let mut end = if lead == 0 { 3 } else { lead };
    while start < digits.len() {
        if start > 0 {
            out.push(GROUP);
        }
        out.push_str(&digits[start..end]);
        start = end;
        end = (end + 3).min(digits.len());
    }
    out
}

impl NumberFormat {
    /// A plain number with a fixed number of decimal places.
    pub fn number(&self, value: f64, decimals: u8) -> String {
        if !value.is_finite() {
            return "\u{2014}".into();
        }
        let fixed = format!("{:.*}", usize::from(decimals), value.abs());
        let (int, frac) = fixed.split_once('.').unwrap_or((&fixed, ""));
        let frac = if self.trailing_zeros { frac } else { frac.trim_end_matches('0') };
        let negative = value < 0.0 && int.bytes().chain(frac.bytes()).any(|b| b != b'0');
        let (int, frac) = if self.digit_grouping {
            (group(int, false), group(frac, true))
        } else {
            (int.to_owned(), frac.to_owned())
        };
        let mut out = String::new();
        if negative {
            out.push(MINUS);
        }
        out.push_str(&int);
        if !frac.is_empty() {
            out.push(if self.decimal_marker == DecimalMarker::Comma { ',' } else { '.' });
            out.push_str(&frac);
        }
        out
    }

    fn length_number(&self, mm: f64) -> String {
        self.number(mm / self.length_unit.mm_per_unit(), self.length_decimals)
    }

    /// A length given in millimetres, converted to and labelled with the display unit.
    pub fn length(&self, mm: f64) -> String {
        format!("{}{NBSP}{}", self.length_number(mm), self.length_unit.symbol())
    }

    /// Extents sharing one unit: "(10.000 x 20.000 x 30.000) mm".
    pub fn length_product(&self, mm: &[f64]) -> String {
        let numbers: Vec<String> = mm.iter().map(|v| self.length_number(*v)).collect();
        format!("({}){NBSP}{}", numbers.join(" \u{d7} "), self.length_unit.symbol())
    }

    /// Separates list items so a comma decimal marker is never ambiguous.
    pub fn list_separator(&self) -> &'static str {
        if self.decimal_marker == DecimalMarker::Comma { "; " } else { ", " }
    }

    /// A dimensionless direction such as a plane normal: "(0.0000, 0.0000, 1.0000)".
    pub fn direction(&self, v: &[f64]) -> String {
        let parts: Vec<String> = v.iter().map(|x| self.number(*x, self.direction_decimals)).collect();
        format!("({})", parts.join(self.list_separator()))
    }
}

/// Parses typed input: either decimal marker, spaces between digit groups, U+2212 and
/// exponents are accepted; anything else is `None` rather than a guessed value.
#[allow(dead_code)] // for numeric fields (custom plane offset), which the section feature owns
pub fn parse_number(text: &str) -> Option<f64> {
    let s: String = text
        .split_whitespace()
        .collect::<String>()
        .replace(MINUS, "-")
        .replacen(',', ".", 1);
    let body = s.strip_prefix(['+', '-']).unwrap_or(&s);
    let (mantissa, exp) = match body.find(['e', 'E']) {
        Some(i) => (&body[..i], Some(&body[i + 1..])),
        None => (body, None),
    };
    let digits = |t: &str| t.bytes().all(|b| b.is_ascii_digit());
    let mantissa_ok = match mantissa.split_once('.') {
        Some((i, f)) => digits(i) && digits(f) && !(i.is_empty() && f.is_empty()),
        None => !mantissa.is_empty() && digits(mantissa),
    };
    let exp_ok = exp.is_none_or(|e| {
        let e = e.strip_prefix(['+', '-']).unwrap_or(e);
        !e.is_empty() && digits(e)
    });
    if !mantissa_ok || !exp_ok {
        return None;
    }
    s.parse::<f64>().ok().filter(|v| v.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: NumberFormat = NumberFormat {
        length_unit: LengthUnit::Mm,
        length_decimals: 3,
        direction_decimals: 4,
        decimal_marker: DecimalMarker::Point,
        digit_grouping: true,
        trailing_zeros: true,
    };
    const COMMA: NumberFormat = NumberFormat { decimal_marker: DecimalMarker::Comma, ..D };

    #[test]
    fn writes_a_length_with_a_fixed_resolution_and_a_no_break_space_before_the_unit() {
        assert_eq!(D.length(15.0), "15.000\u{a0}mm");
    }

    #[test]
    fn uses_the_minus_sign_and_never_shows_a_negative_zero() {
        assert_eq!(D.number(-2.5, 1), "\u{2212}2.5");
        assert_eq!(D.number(-0.0001, 3), "0.000");
    }

    #[test]
    fn groups_digits_in_threes_leaving_four_digit_groups_alone() {
        assert_eq!(D.number(1234.5, 1), "1234.5");
        assert_eq!(D.number(1234567.123456, 6), "1\u{202f}234\u{202f}567.123\u{202f}456");
        assert_eq!(NumberFormat { digit_grouping: false, ..D }.number(1234567.0, 0), "1234567");
    }

    #[test]
    fn comma_marker_uses_semicolon_separated_lists() {
        assert_eq!(COMMA.length(12.5), "12,500\u{a0}mm");
        assert_eq!(COMMA.direction(&[0.0, 0.6, -0.8]), "(0,0000; 0,6000; \u{2212}0,8000)");
        assert_eq!(D.direction(&[0.0, 0.0, 1.0]), "(0.0000, 0.0000, 1.0000)");
    }

    #[test]
    fn can_drop_trailing_zeros() {
        let f = NumberFormat { trailing_zeros: false, ..D };
        assert_eq!(f.length(10.0), "10\u{a0}mm");
        assert_eq!(f.length(10.25), "10.25\u{a0}mm");
    }

    #[test]
    fn writes_extents_with_one_shared_unit() {
        assert_eq!(D.length_product(&[10.0, 20.0, 30.0]), "(10.000 \u{d7} 20.000 \u{d7} 30.000)\u{a0}mm");
    }

    #[test]
    fn converts_only_the_displayed_length_unit() {
        let unit = |u| NumberFormat { length_unit: u, ..D };
        assert_eq!(unit(LengthUnit::In).length(25.4), "1.000\u{a0}in");
        assert_eq!(unit(LengthUnit::M).length(1500.0), "1.500\u{a0}m");
    }

    #[test]
    fn parses_either_marker_group_spaces_and_minus_and_rejects_anything_else() {
        assert_eq!(parse_number("12,5"), Some(12.5));
        assert_eq!(parse_number(" 1 234.5 "), Some(1234.5));
        assert_eq!(parse_number("\u{2212}3"), Some(-3.0));
        assert_eq!(parse_number("1e-3"), Some(0.001));
        assert_eq!(parse_number(""), None);
        assert_eq!(parse_number("1,234,5"), None);
        assert_eq!(parse_number("12 mm"), None);
    }

    #[test]
    fn project_files_of_the_react_app_deserialize() {
        let f: NumberFormat = serde_json::from_str(r#"{"lengthUnit":"in","decimalMarker":","}"#).unwrap();
        assert_eq!((f.length_unit, f.decimal_marker, f.length_decimals), (LengthUnit::In, DecimalMarker::Comma, 3));
    }
}
