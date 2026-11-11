//! `OptionGroup` and `OptionGroupMultiple`: a group of native radios, checkboxes or switches
//! built from [`Options`](crate::Options) (ADR-0012).
//!
//! The selection rules (is an option selected, add/remove one from a list) are pure functions
//! here, tested on the host; [`component`] holds the two public components.

mod component;

pub use component::{OptionGroup, OptionGroupMultiple, OptionGroupType};

/// `true` if `list` already holds `value` (by the options' identity).
pub(crate) fn is_selected<V>(list: &[V], value: &V, same: impl Fn(&V, &V) -> bool) -> bool {
    list.iter().any(|v| same(v, value))
}

/// Adds `value` to the end of `list`, or removes it when it is already there. Returns `true` if
/// it is selected afterwards.
pub(crate) fn toggle<V: Clone>(
    list: &mut Vec<V>,
    value: &V,
    same: impl Fn(&V, &V) -> bool,
) -> bool {
    match list.iter().position(|v| same(v, value)) {
        Some(index) => {
            list.remove(index);
            false
        }
        None => {
            list.push(value.clone());
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{is_selected, toggle};

    #[test]
    fn toggle_adds_at_the_end_then_removes() {
        let mut list = vec!["a"];
        assert!(toggle(&mut list, &"c", |a, b| a == b));
        assert_eq!(list, ["a", "c"]);
        assert!(!toggle(&mut list, &"a", |a, b| a == b));
        assert_eq!(list, ["c"]);
    }

    #[test]
    fn identity_decides_selection() {
        // Same key, different payload: the options' identity (here the first field) wins.
        let list = vec![(1, "x")];
        assert!(is_selected(&list, &(1, "other"), |a, b| a.0 == b.0));
        assert!(!is_selected(&list, &(2, "x"), |a, b| a.0 == b.0));
        assert!(!is_selected(&[], &(1, "x"), |a: &(i32, &str), b| a.0 == b.0));
    }
}
