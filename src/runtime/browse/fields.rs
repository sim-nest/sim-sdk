//! Shared field helpers for the browse subsystem.
//!
//! Browse modules independently re-grew the same record-access helpers in three
//! shapes: a table-key builder (`field_symbol`/`field`), `Value`-record lookup
//! (`find_field`/`field_value`/`field`), and `Expr::Map` entry lookup
//! (`expr_field`/`bool_field`). This module is their single owner; each module
//! imports these under its existing local names.

use sim_kernel::{Expr, Symbol, Value};

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

/// Look up a field in `Expr::Map` entries by bare-symbol key.
pub(super) fn expr_entry_field<'a>(entries: &'a [(Expr, Expr)], name: &str) -> Option<&'a Expr> {
    let wanted = key(name);
    entries
        .iter()
        .find_map(|(candidate, value)| match candidate {
            Expr::Symbol(candidate) if candidate == &wanted => Some(value),
            _ => None,
        })
}

/// Look up a field on an `Expr::Map` value by bare-symbol key.
pub(super) fn expr_field<'a>(map: &'a Expr, name: &str) -> Option<&'a Expr> {
    let Expr::Map(entries) = map else {
        return None;
    };
    expr_entry_field(entries, name)
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
}
