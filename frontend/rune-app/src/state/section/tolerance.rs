//! Optional validation-tolerance overrides of a section (the service's `tolerance` object).
//! Empty fields keep the service defaults; typed values are checked against the service bounds
//! before they are kept, so an invalid entry never reaches a request.

use rune::TextEdit;

use crate::api::dto::Tolerance;
use crate::format::parse_number;

#[derive(Debug, Clone, Copy)]
pub struct Field {
    pub key: &'static str,
    pub label: &'static str,
    pub unit: &'static str,
    pub min: f64,
    pub max: f64,
    pub default: f64,
}

pub const FIELDS: [Field; 3] = [
    Field {
        key: "tol-small-region",
        label: "Small region",
        unit: "mm2",
        min: 1e-8,
        max: 1.0,
        default: 1e-4,
    },
    Field {
        key: "tol-sliver",
        label: "Sliver ratio",
        unit: "area / perimeter2",
        min: 1e-6,
        max: 0.05,
        default: 5e-4,
    },
    Field {
        key: "tol-join",
        label: "Join distance",
        unit: "mm",
        min: 1e-6,
        max: 1e-3,
        default: 1e-5,
    },
];

#[derive(Debug, Default)]
pub struct ToleranceState {
    /// The "Advanced tolerances" section is expanded.
    pub open: bool,
    pub value: Tolerance,
    pub text: [TextEdit; 3],
    /// Why the last entry of each field was rejected.
    pub error: [Option<String>; 3],
}

fn get(t: &Tolerance, i: usize) -> Option<f64> {
    [t.small_region_mm2, t.sliver_ratio, t.join_mm][i]
}

fn put(t: &mut Tolerance, i: usize, v: Option<f64>) {
    match i {
        0 => t.small_region_mm2 = v,
        1 => t.sliver_ratio = v,
        _ => t.join_mm = v,
    }
}

pub fn show(v: f64) -> String {
    format!("{v}")
}

impl ToleranceState {
    /// Applies the typed text of field `i`; true when the value changed. A rejected entry keeps
    /// the old value and records why.
    pub fn commit(&mut self, i: usize) -> bool {
        let f = FIELDS[i];
        let typed = self.text[i].text.trim().to_owned();
        self.error[i] = None;
        let new = if typed.is_empty() {
            None
        } else {
            match parse_number(&typed) {
                Some(v) if (f.min..=f.max).contains(&v) => Some(v),
                Some(_) => {
                    self.error[i] = Some(format!(
                        "{} must be between {} and {} {}.",
                        f.label,
                        show(f.min),
                        show(f.max),
                        f.unit
                    ));
                    return false;
                }
                None => {
                    self.error[i] = Some(format!("\"{typed}\" is not a number."));
                    return false;
                }
            }
        };
        let changed = get(&self.value, i) != new;
        put(&mut self.value, i, new);
        changed
    }

    pub fn commit_all(&mut self) -> bool {
        (0..3).fold(false, |c, i| self.commit(i) | c)
    }

    /// Replaces the values (project open, section switch) and rewrites the fields.
    pub fn restore(&mut self, value: Tolerance) {
        self.value = value;
        self.error = Default::default();
        for i in 0..3 {
            self.text[i].replace_text(get(&value, i).map(show).unwrap_or_default());
        }
    }

    pub fn is_default(&self) -> bool {
        self.value == Tolerance::default()
    }

    pub fn is_set(&self, i: usize) -> bool {
        get(&self.value, i).is_some()
    }

    /// The request object; `None` when everything is default.
    pub fn request(&self) -> Option<Tolerance> {
        (!self.is_default()).then_some(self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(s: &mut ToleranceState, i: usize, text: &str) -> bool {
        s.text[i].replace_text(text);
        s.commit(i)
    }

    #[test]
    fn values_inside_the_bounds_are_kept_and_empty_resets_to_default() {
        let mut s = ToleranceState::default();
        assert!(s.is_default() && s.request().is_none());
        assert!(typed(&mut s, 0, "1e-3"));
        assert!(typed(&mut s, 2, "0.0001"));
        assert_eq!(
            s.request(),
            Some(Tolerance {
                small_region_mm2: Some(1e-3),
                sliver_ratio: None,
                join_mm: Some(1e-4)
            })
        );
        assert!(typed(&mut s, 0, ""));
        assert!(s.value.small_region_mm2.is_none());
    }

    #[test]
    fn out_of_range_and_non_numeric_entries_are_rejected_inline_and_keep_the_old_value() {
        let mut s = ToleranceState::default();
        typed(&mut s, 1, "0.01");
        assert!(!typed(&mut s, 1, "0.5"));
        assert!(s.error[1].as_deref().unwrap().contains("between"));
        assert_eq!(s.value.sliver_ratio, Some(0.01));
        assert!(!typed(&mut s, 2, "abc"));
        assert!(s.error[2].as_deref().unwrap().contains("not a number"));
        assert!(!typed(&mut s, 2, "1e-9"), "below the lower bound");
        assert!(s.value.join_mm.is_none());
    }
}
