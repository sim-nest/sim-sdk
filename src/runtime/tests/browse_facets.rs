use std::sync::Arc;

use sim_kernel::{Args, Cx, DefaultFactory, Expr, NoopEvalPolicy, Symbol, Value};

use crate::runtime::{
    browse::schema::FacetBuilder, browse_internal_capability, install_core_runtime,
};

use super::support::table_value;

#[test]
fn cards_include_initial_facets_and_redact_internal_payloads() {
    let mut cx = test_cx();
    let card = browse_help_card(&mut cx);
    let card = expr(&mut cx, &card);
    let facets = list_items(table_value(&card, &field("facets")).expect("facets"));

    assert!(facet_by_name(facets, Symbol::qualified("browse", "schema")).is_some());
    assert!(facet_by_name(facets, Symbol::qualified("browse", "examples")).is_some());
    let operations =
        facet_by_name(facets, Symbol::qualified("browse", "operations")).expect("operations");

    assert_eq!(
        table_value(operations, &field("visibility")),
        Some(&Expr::Symbol(Symbol::new("private")))
    );
    assert_list_contains_symbol(
        table_value(operations, &field("requires")).expect("requires"),
        Symbol::qualified("capability", "browse.internal"),
    );

    let redaction = table_value(operations, &field("value")).expect("redaction");
    assert_eq!(
        table_value(redaction, &field("reason")),
        Some(&Expr::Symbol(Symbol::new("capability-required")))
    );
    assert_list_contains_symbol(
        table_value(redaction, &field("requires")).expect("redaction requires"),
        Symbol::qualified("capability", "browse.internal"),
    );
}

#[test]
fn browse_internal_reveals_operations_facet_payload() {
    let mut cx = test_cx();
    cx.grant(browse_internal_capability());

    let card = browse_help_card(&mut cx);
    let card = expr(&mut cx, &card);
    let facets = list_items(table_value(&card, &field("facets")).expect("facets"));
    let operations =
        facet_by_name(facets, Symbol::qualified("browse", "operations")).expect("operations");
    let payload = table_value(operations, &field("value")).expect("operations payload");
    let operation_entries = list_items(payload);

    assert!(operation_entries.iter().any(|entry| {
        table_value(entry, &field("key")) == Some(&Expr::String("core/call.v1".to_owned()))
    }));
}

#[test]
fn facet_shape_accepts_complete_facets_and_rejects_missing_fields() {
    let mut cx = test_cx();
    let valid = FacetBuilder::new(Symbol::qualified("browse", "schema"))
        .build(&mut cx)
        .unwrap();
    assert!(shape_accepts(
        &mut cx,
        Symbol::qualified("browse", "Facet"),
        valid
    ));

    let malformed = cx
        .factory()
        .table(vec![
            (
                field("name"),
                cx.factory()
                    .symbol(Symbol::qualified("browse", "schema"))
                    .unwrap(),
            ),
            (
                field("version"),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "i64"), "1".to_owned())
                    .unwrap(),
            ),
        ])
        .unwrap();
    assert!(!shape_accepts(
        &mut cx,
        Symbol::qualified("browse", "Facet"),
        malformed
    ));
}

fn test_cx() -> Cx {
    let mut cx = Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    cx
}

fn browse_help_card(cx: &mut Cx) -> Value {
    let subject = cx
        .factory()
        .symbol(Symbol::qualified("core", "help"))
        .unwrap();
    cx.call_function(
        &Symbol::qualified("core", "browse"),
        Args::new(vec![subject]),
    )
    .unwrap()
}

fn shape_accepts(cx: &mut Cx, symbol: Symbol, value: Value) -> bool {
    let shape = cx.resolve_shape(&symbol).unwrap();
    shape
        .object()
        .as_shape()
        .unwrap()
        .check_value(cx, value)
        .unwrap()
        .accepted
}

fn expr(cx: &mut Cx, value: &Value) -> Expr {
    value.object().as_expr(cx).unwrap()
}

fn list_items(expr: &Expr) -> &[Expr] {
    let Expr::List(items) = expr else {
        panic!("expected list");
    };
    items
}

fn facet_by_name(facets: &[Expr], name: Symbol) -> Option<&Expr> {
    facets
        .iter()
        .find(|facet| table_value(facet, &field("name")) == Some(&Expr::Symbol(name.clone())))
}

fn assert_list_contains_symbol(expr: &Expr, expected: Symbol) {
    let Expr::List(items) = expr else {
        panic!("expected list");
    };
    assert!(
        items
            .iter()
            .any(|item| matches!(item, Expr::Symbol(symbol) if symbol == &expected))
    );
}

fn field(name: &str) -> Symbol {
    Symbol::new(name.to_owned())
}
