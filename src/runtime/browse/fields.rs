//! Shared field helpers for the browse subsystem.
//!
//! Browse modules independently re-grew the same record-access helpers in three
//! shapes: a table-key builder (`field_symbol`/`field`), `Value`-record lookup
//! (`find_field`/`field_value`/`field`), and `Expr::Map` entry lookup
//! (`expr_field`/`bool_field`). This module is their single owner; each module
//! imports these under its existing local names.

use sim_kernel::{Expr, Symbol, Value};

pub(super) use sim_value::access::{entry_field as expr_entry_field, field as expr_field};

/// A browse table key: a bare (unqualified) symbol named `name`.
pub(super) fn key(name: &str) -> Symbol {
    Symbol::new(name.to_owned())
}

/// Look up a field in a `Value` record's table entries by bare-symbol key.
pub(super) fn value_field<'a>(entries: &'a [(Symbol, Value)], name: &str) -> Option<&'a Value> {
    let wanted = key(name);
    entries
        .iter()
        .find_map(|(field, value)| (field == &wanted).then_some(value))
}

/// Read a boolean field on an `Expr::Map` value.
pub(super) fn bool_field(map: &Expr, name: &str) -> Option<bool> {
    match expr_field(map, name) {
        Some(Expr::Bool(value)) => Some(*value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_and_expr_field_match_bare_symbol_keys() {
        let entries = vec![(Expr::Symbol(Symbol::new("a")), Expr::Bool(true))];
        assert_eq!(expr_entry_field(&entries, "a"), Some(&Expr::Bool(true)));
        assert_eq!(expr_entry_field(&entries, "missing"), None);

        let map = Expr::Map(entries);
        assert_eq!(bool_field(&map, "a"), Some(true));
        assert_eq!(bool_field(&map, "missing"), None);
    }

    #[test]
    fn expr_fields_reject_string_and_qualified_keys() {
        let string_key = Expr::Map(vec![(Expr::String("a".into()), Expr::Bool(true))]);
        let qualified_key = Expr::Map(vec![(
            Expr::Symbol(Symbol::qualified("browse", "a")),
            Expr::Bool(true),
        )]);

        assert_eq!(expr_field(&string_key, "a"), None);
        assert_eq!(expr_field(&qualified_key, "a"), None);
    }
}
