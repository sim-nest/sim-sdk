use std::sync::Arc;

use sim_kernel::{
    Args, Cx, DefaultFactory, Expr, NoopEvalPolicy, Ref, Symbol, Value,
    registry_catalog_read_capability,
};

use crate::runtime::{
    browse::schema::{
        BROWSE_TEST_FIELDS, BrowseTestBuilder, CARD_V2_FIELDS, COVERAGE_FIELDS, CoverageBuilder,
        FACET_FIELDS, FacetBuilder, HELP_FIELDS, HelpBuilder, REDACTION_FIELDS, RedactionBuilder,
        TEST_REPORT_FIELDS, TestReportBuilder, card_v2_for_ref,
    },
    install_core_runtime,
};

use super::support::table_value;

#[test]
fn card_v2_builder_preserves_spine_and_appends_b6_fields() {
    let mut cx = test_cx();
    let subject = Symbol::qualified("core", "help");
    let card = card_v2_for_ref(&mut cx, Ref::Symbol(subject.clone())).unwrap();
    let expr = table_expr(&mut cx, &card);

    assert_eq!(table_keys(&expr), symbols(CARD_V2_FIELDS));
    assert_eq!(
        table_value(&expr, &Symbol::new("subject")),
        Some(&Expr::Symbol(subject))
    );
    assert_eq!(
        table_value(&expr, &Symbol::new("kind")),
        Some(&Expr::Symbol(Symbol::qualified("core", "function")))
    );
    assert_eq!(
        table_keys(table_value(&expr, &Symbol::new("coverage")).expect("coverage")),
        symbols(COVERAGE_FIELDS)
    );
    assert_eq!(
        table_value(&expr, &Symbol::new("freshness")),
        Some(&Expr::Symbol(Symbol::new("unknown")))
    );
}

#[test]
fn registry_catalog_card_preserves_card_v2_spine() {
    let mut cx = test_cx();
    cx.grant(registry_catalog_read_capability());
    let subject = Symbol::qualified("registry", "catalog");
    let subject_value = cx.factory().symbol(subject.clone()).unwrap();
    let card = cx
        .call_function(
            &Symbol::qualified("core", "browse"),
            Args::new(vec![subject_value]),
        )
        .unwrap();
    let expr = table_expr(&mut cx, &card);

    assert_eq!(table_keys(&expr), symbols(CARD_V2_FIELDS));
    assert_eq!(
        table_value(&expr, &Symbol::new("subject")),
        Some(&Expr::Symbol(subject))
    );
    assert_eq!(
        table_value(&expr, &Symbol::new("kind")),
        Some(&Expr::Symbol(Symbol::qualified("registry", "catalog")))
    );
}

#[test]
fn help_builder_emits_exact_key_order() {
    let mut cx = test_cx();
    let subject = cx
        .factory()
        .symbol(Symbol::qualified("core", "help"))
        .unwrap();
    let value = HelpBuilder::new(subject).build(&mut cx).unwrap();

    assert_eq!(value_keys(&mut cx, &value), symbols(HELP_FIELDS));
}

#[test]
fn test_builder_emits_exact_key_order() {
    let mut cx = test_cx();
    let value = BrowseTestBuilder::new(
        Symbol::qualified("test", "sample"),
        Symbol::qualified("test", "runtime"),
    )
    .build(&mut cx)
    .unwrap();

    assert_eq!(value_keys(&mut cx, &value), symbols(BROWSE_TEST_FIELDS));
}

#[test]
fn coverage_facet_redaction_and_report_builders_emit_exact_key_order() {
    let mut cx = test_cx();
    let coverage = CoverageBuilder::default().build(&mut cx).unwrap();
    let facet = FacetBuilder::new(Symbol::qualified("browse", "schema"))
        .build(&mut cx)
        .unwrap();
    let redaction = RedactionBuilder::unavailable().build(&mut cx).unwrap();
    let report = TestReportBuilder::new(Symbol::qualified("test", "sample"))
        .build(&mut cx)
        .unwrap();

    assert_eq!(value_keys(&mut cx, &coverage), symbols(COVERAGE_FIELDS));
    assert_eq!(value_keys(&mut cx, &facet), symbols(FACET_FIELDS));
    assert_eq!(value_keys(&mut cx, &redaction), symbols(REDACTION_FIELDS));
    assert_eq!(value_keys(&mut cx, &report), symbols(TEST_REPORT_FIELDS));
}

#[test]
fn browse_schema_shapes_are_registered() {
    let cx = test_cx();

    for symbol in [
        Symbol::qualified("core", "Card"),
        Symbol::qualified("browse", "Help"),
        Symbol::qualified("browse", "Test"),
        Symbol::qualified("browse", "Coverage"),
        Symbol::qualified("browse", "Facet"),
        Symbol::qualified("browse", "Redaction"),
        Symbol::qualified("browse", "TestReport"),
    ] {
        assert!(
            cx.registry().shape_by_symbol(&symbol).is_some(),
            "{symbol} shape should be registered"
        );
    }
}

fn test_cx() -> Cx {
    let mut cx = Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    cx
}

fn value_keys(cx: &mut Cx, value: &Value) -> Vec<Symbol> {
    table_keys(&table_expr(cx, value))
}

fn table_expr(cx: &mut Cx, value: &Value) -> Expr {
    let expr = value.object().as_expr(cx).unwrap();
    assert!(matches!(expr, Expr::Map(_)), "expected table expr");
    expr
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
