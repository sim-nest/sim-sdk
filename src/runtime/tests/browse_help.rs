use std::sync::Arc;

use sim_kernel::{
    Args, ClaimPattern, Datum, DatumStore, DefaultFactory, Expr, NoopEvalPolicy, Ref, Symbol,
};

use crate::runtime::{
    browse::{
        predicates::help_doc_predicate,
        schema::{HELP_FIELDS, card_v2_for_ref},
    },
    install_core_runtime,
};

use super::support::table_value;

#[test]
fn help_projection_publishes_content_addressed_browse_help_doc_claim() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    call_help_for_core_help(&mut cx);

    let claims = cx
        .query_facts(ClaimPattern {
            subject: Some(Ref::Symbol(Symbol::qualified("core", "help"))),
            predicate: Some(help_doc_predicate()),
            object: None,
            include_revoked: false,
        })
        .unwrap();
    assert_eq!(claims.len(), 1);
    let Ref::Content(id) = &claims[0].object else {
        panic!("browse/help-doc should store a content-addressed Help object");
    };
    let Datum::Map(entries) = cx.datum_store().get(id).unwrap().unwrap() else {
        panic!("browse/help-doc should store a Help map");
    };

    assert_eq!(
        datum_field(entries, "subject"),
        Some(&Datum::Symbol(Symbol::qualified("core", "help")))
    );
    assert_eq!(
        datum_field(entries, "kind"),
        Some(&Datum::Symbol(Symbol::qualified("core", "function")))
    );
    assert!(matches!(
        datum_field(entries, "summary"),
        Some(Datum::String(summary)) if summary.contains("returns and publishes")
    ));
    assert!(datum_field(entries, "purpose").is_none());
}

#[test]
fn card_v2_uses_help_doc_as_fixed_help_table() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    call_help_for_core_help(&mut cx);

    let card = card_v2_for_ref(&mut cx, Ref::Symbol(Symbol::qualified("core", "help"))).unwrap();
    let expr = card.object().as_expr(&mut cx).unwrap();
    let help = table_value(&expr, &Symbol::new("help")).expect("Card help");

    assert_eq!(table_keys(help), symbols(HELP_FIELDS));
    assert_eq!(
        table_value(help, &Symbol::new("kind")),
        Some(&Expr::Symbol(Symbol::qualified("core", "function")))
    );
    assert!(matches!(
        table_value(help, &Symbol::new("summary")),
        Some(Expr::String(summary)) if summary.contains("returns and publishes")
    ));
    assert!(table_value(help, &Symbol::new("purpose")).is_none());
}

fn call_help_for_core_help(cx: &mut sim_kernel::Cx) {
    let subject = cx
        .factory()
        .symbol(Symbol::qualified("core", "help"))
        .unwrap();
    cx.call_function(&Symbol::qualified("core", "help"), Args::new(vec![subject]))
        .unwrap();
}

fn datum_field<'a>(entries: &'a [(Datum, Datum)], name: &str) -> Option<&'a Datum> {
    let key = Datum::Symbol(Symbol::new(name));
    entries
        .iter()
        .find_map(|(field, value)| (field == &key).then_some(value))
}

fn table_keys(expr: &Expr) -> Vec<Symbol> {
    let Expr::Map(entries) = expr else {
        panic!("expected map");
    };
    entries
        .iter()
        .map(|(key, _)| match key {
            Expr::Symbol(symbol) => symbol.clone(),
            other => panic!("expected symbol key, got {other:?}"),
        })
        .collect()
}

fn symbols(fields: &[&str]) -> Vec<Symbol> {
    fields.iter().map(|field| Symbol::new(*field)).collect()
}
