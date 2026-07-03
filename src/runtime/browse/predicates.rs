#![allow(dead_code)]

use sim_kernel::Symbol;

pub(crate) fn help_doc_predicate() -> Symbol {
    browse_symbol("help-doc")
}

pub(crate) fn test_predicate() -> Symbol {
    browse_symbol("test")
}

pub(crate) fn example_predicate() -> Symbol {
    browse_symbol("example")
}

pub(crate) fn coverage_predicate() -> Symbol {
    browse_symbol("coverage")
}

pub(crate) fn facet_predicate() -> Symbol {
    browse_symbol("facet")
}

pub(crate) fn provenance_predicate() -> Symbol {
    browse_symbol("provenance")
}

pub(crate) fn freshness_predicate() -> Symbol {
    browse_symbol("freshness")
}

pub(crate) fn schema_predicate() -> Symbol {
    browse_symbol("schema")
}

pub(crate) fn redacted_predicate() -> Symbol {
    browse_symbol("redacted")
}

pub(crate) fn event_predicate() -> Symbol {
    browse_symbol("event")
}

pub(crate) fn effect_predicate() -> Symbol {
    browse_symbol("effect")
}

fn browse_symbol(name: &str) -> Symbol {
    Symbol::qualified("browse", name)
}
