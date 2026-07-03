use std::sync::Arc;

use sim_kernel::{Args, DefaultFactory, EagerPolicy, Expr, NumberLiteral, Symbol, Value};

use crate::runtime::install_core_runtime;

fn cx() -> sim_kernel::Cx {
    let mut cx = sim_kernel::Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    cx
}

fn number_expr(text: &str) -> Expr {
    Expr::Number(NumberLiteral {
        domain: Symbol::qualified("numbers", "f64"),
        canonical: text.to_owned(),
    })
}

fn expr_kind_shape(cx: &mut sim_kernel::Cx, kind: &str) -> Value {
    let kind = cx.factory().symbol(Symbol::new(kind)).unwrap();
    cx.call_class(
        &Symbol::qualified("core", "ExprKindShape"),
        Args::new(vec![kind]),
    )
    .unwrap()
}

fn exact_expr_shape(cx: &mut sim_kernel::Cx, expr: Expr) -> Value {
    let value = cx.factory().expr(expr).unwrap();
    cx.call_class(
        &Symbol::qualified("core", "ExactExprShape"),
        Args::new(vec![value]),
    )
    .unwrap()
}

fn table_entries(cx: &mut sim_kernel::Cx, value: &Value) -> Vec<(Symbol, Value)> {
    value.object().as_table_impl().unwrap().entries(cx).unwrap()
}

fn entry(entries: &[(Symbol, Value)], key: &str) -> Value {
    entries
        .iter()
        .find_map(|(candidate, value)| (candidate == &Symbol::new(key)).then_some(value.clone()))
        .unwrap()
}

fn expr_of(cx: &mut sim_kernel::Cx, value: Value) -> Expr {
    value.object().as_expr(cx).unwrap()
}

#[test]
fn runtime_shape_compare_returns_stable_table_keys() {
    let mut cx = cx();
    let exact = exact_expr_shape(&mut cx, number_expr("1"));
    let number = expr_kind_shape(&mut cx, "number");

    let relation = cx
        .call_function(
            &Symbol::qualified("shape", "compare"),
            Args::new(vec![exact, number]),
        )
        .unwrap();
    let entries = table_entries(&mut cx, &relation);
    let keys = entries
        .iter()
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();

    assert_eq!(
        keys,
        vec![
            Symbol::new("kind"),
            Symbol::new("proven"),
            Symbol::new("left"),
            Symbol::new("right"),
            Symbol::new("witness-count"),
            Symbol::new("witnesses"),
            Symbol::new("diagnostics"),
        ]
    );
    assert!(matches!(
        expr_of(&mut cx, entry(&entries, "kind")),
        Expr::Symbol(symbol) if symbol == Symbol::qualified("shape", "left-subshape")
    ));
    assert!(matches!(
        expr_of(&mut cx, entry(&entries, "proven")),
        Expr::Bool(true)
    ));
}

#[test]
fn runtime_shape_compare_with_parses_expr_probes() {
    let mut cx = cx();
    let number = expr_kind_shape(&mut cx, "number");
    let string = expr_kind_shape(&mut cx, "string");
    let kind = cx.factory().symbol(Symbol::new("expr")).unwrap();
    let label = cx.factory().string("bool".to_owned()).unwrap();
    let target = cx.factory().bool(true).unwrap();
    let probe = cx.factory().list(vec![kind, label, target]).unwrap();
    let probes = cx.factory().list(vec![probe]).unwrap();

    let relation = cx
        .call_function(
            &Symbol::qualified("shape", "compare-with"),
            Args::new(vec![number, string, probes]),
        )
        .unwrap();
    let entries = table_entries(&mut cx, &relation);

    assert!(matches!(
        expr_of(&mut cx, entry(&entries, "kind")),
        Expr::Symbol(symbol) if symbol == Symbol::qualified("shape", "unknown")
    ));
    assert!(expr_of(&mut cx, entry(&entries, "witness-count")).canonical_eq(&number_expr("1")));
}

#[test]
fn runtime_venn_helpers_are_registered_and_build_shapes() {
    let mut cx = cx();
    let number = expr_kind_shape(&mut cx, "number");
    let string = expr_kind_shape(&mut cx, "string");
    let number_member = cx
        .factory()
        .list(vec![
            cx.factory().symbol(Symbol::new("number")).unwrap(),
            number.clone(),
        ])
        .unwrap();
    let string_member = cx
        .factory()
        .list(vec![
            cx.factory().symbol(Symbol::new("string")).unwrap(),
            string.clone(),
        ])
        .unwrap();
    let members = cx
        .factory()
        .list(vec![number_member, string_member])
        .unwrap();
    let venn = cx
        .call_function(
            &Symbol::qualified("shape", "venn"),
            Args::new(vec![members]),
        )
        .unwrap();

    let union = cx
        .call_function(
            &Symbol::qualified("shape", "venn-union"),
            Args::new(vec![venn.clone()]),
        )
        .unwrap();
    let intersection = cx
        .call_function(
            &Symbol::qualified("shape", "venn-intersection"),
            Args::new(vec![venn.clone()]),
        )
        .unwrap();
    let only_name = cx.factory().symbol(Symbol::new("number")).unwrap();
    let only = cx
        .call_function(
            &Symbol::qualified("shape", "venn-only"),
            Args::new(vec![venn.clone(), only_name]),
        )
        .unwrap();
    let outside = cx
        .call_function(
            &Symbol::qualified("shape", "venn-outside"),
            Args::new(vec![venn.clone()]),
        )
        .unwrap();
    let exact_name = cx.factory().symbol(Symbol::new("number")).unwrap();
    let names = cx.factory().list(vec![exact_name]).unwrap();
    let exactly = cx
        .call_function(
            &Symbol::qualified("shape", "venn-exactly"),
            Args::new(vec![venn, names]),
        )
        .unwrap();

    assert!(union.object().as_shape().is_some());
    assert!(intersection.object().as_shape().is_some());
    assert!(only.object().as_shape().is_some());
    assert!(outside.object().as_shape().is_some());
    assert!(exactly.object().as_shape().is_some());
    assert!(
        union
            .object()
            .as_shape()
            .unwrap()
            .check_expr(&mut cx, &Expr::String("ok".to_owned()))
            .unwrap()
            .accepted
    );
    assert!(
        !only
            .object()
            .as_shape()
            .unwrap()
            .check_expr(&mut cx, &Expr::String("ok".to_owned()))
            .unwrap()
            .accepted
    );
}
