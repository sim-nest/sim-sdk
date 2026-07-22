use std::sync::Arc;

use sim_kernel::{
    Args, DefaultFactory, EagerPolicy, Expr, NumberLiteral, Symbol, macro_expand_eval_capability,
};

use crate::runtime::install_core_runtime;

fn cx() -> sim_kernel::Cx {
    let mut cx = sim_kernel::Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    cx.grant(macro_expand_eval_capability());
    cx
}

fn number_expr(text: &str) -> Expr {
    Expr::Number(NumberLiteral {
        domain: Symbol::qualified("numbers", "f64"),
        canonical: text.to_owned(),
    })
}

fn number_value(cx: &mut sim_kernel::Cx, text: &str) -> sim_kernel::Value {
    cx.factory()
        .number_literal(Symbol::qualified("numbers", "f64"), text.to_owned())
        .unwrap()
}

fn expr_kind_shape(cx: &mut sim_kernel::Cx, kind: &str) -> sim_kernel::Value {
    let kind = cx.factory().symbol(Symbol::new(kind)).unwrap();
    cx.call_class(
        &Symbol::qualified("core", "ExprKindShape"),
        Args::new(vec![kind]),
    )
    .unwrap()
}

#[test]
fn runtime_combiner_constructors_are_registered() {
    let mut cx = cx();
    let any = cx.resolve_shape(&Symbol::qualified("core", "Any")).unwrap();
    let number = expr_kind_shape(&mut cx, "number");
    let parts = cx.factory().list(vec![any, number]).unwrap();
    let shape = cx
        .call_function(&Symbol::qualified("shape", "and"), Args::new(vec![parts]))
        .unwrap();

    let matched = shape
        .object()
        .as_shape()
        .unwrap()
        .check_expr(&mut cx, &number_expr("1"))
        .unwrap();
    assert!(matched.accepted);

    let matched = shape
        .object()
        .as_shape()
        .unwrap()
        .check_expr(&mut cx, &Expr::String("no".to_owned()))
        .unwrap();
    assert!(!matched.accepted);
    assert!(matched.diagnostics[0].message.starts_with("shape-and:"));
}

#[test]
fn runtime_alias_constructors_are_registered() {
    let mut cx = cx();
    let any = cx.resolve_shape(&Symbol::qualified("core", "Any")).unwrap();
    let number = expr_kind_shape(&mut cx, "number");
    let choices = cx.factory().list(vec![number]).unwrap();
    let none = cx
        .call_function(&Symbol::qualified("shape", "none"), Args::new(vec![any]))
        .unwrap();
    let any_shape = cx
        .call_function(&Symbol::qualified("shape", "any"), Args::new(vec![choices]))
        .unwrap();
    let without = cx
        .call_function(
            &Symbol::qualified("shape", "without"),
            Args::new(vec![any_shape, none]),
        )
        .unwrap();

    assert!(without.object().as_shape().is_some());
}

#[test]
fn runtime_list_rest_constructor_builds_variadic_list_shape() {
    let mut cx = cx();
    let any = cx.resolve_shape(&Symbol::qualified("core", "Any")).unwrap();
    let number = expr_kind_shape(&mut cx, "number");
    let prefix = cx.factory().list(vec![any]).unwrap();
    let shape = cx
        .call_function(
            &Symbol::qualified("shape", "list-rest"),
            Args::new(vec![prefix, number]),
        )
        .unwrap();

    let matched = shape
        .object()
        .as_shape()
        .unwrap()
        .check_expr(
            &mut cx,
            &Expr::List(vec![Expr::Bool(true), number_expr("1"), number_expr("2")]),
        )
        .unwrap();

    assert!(matched.accepted);
}

#[test]
fn runtime_table_and_repeat_constructors_work() {
    let mut cx = cx();
    let number = expr_kind_shape(&mut cx, "number");
    let key = cx.factory().symbol(Symbol::new("n")).unwrap();
    let field = cx.factory().list(vec![key, number.clone()]).unwrap();
    let fields = cx.factory().list(vec![field]).unwrap();
    let table_shape = cx
        .call_function(
            &Symbol::qualified("shape", "table-closed"),
            Args::new(vec![fields]),
        )
        .unwrap();
    let extra = number_value(&mut cx, "2");
    let required = number_value(&mut cx, "1");
    let table = cx
        .factory()
        .table(vec![
            (Symbol::new("n"), required),
            (Symbol::new("extra"), extra),
        ])
        .unwrap();

    let matched = table_shape
        .object()
        .as_shape()
        .unwrap()
        .check_value(&mut cx, table)
        .unwrap();
    assert!(!matched.accepted);
    assert!(matched.diagnostics[0].message.starts_with("shape-table:"));

    let min = number_value(&mut cx, "1");
    let max = number_value(&mut cx, "2");
    let repeat_shape = cx
        .call_function(
            &Symbol::qualified("shape", "repeat-bounds"),
            Args::new(vec![number, min, max]),
        )
        .unwrap();
    let matched = repeat_shape
        .object()
        .as_shape()
        .unwrap()
        .check_expr(
            &mut cx,
            &Expr::Vector(vec![number_expr("1"), number_expr("2")]),
        )
        .unwrap();
    assert!(matched.accepted);
}

#[test]
fn runtime_shape_constructors_encode_as_expr_calls() {
    let mut cx = cx();
    let any = cx.resolve_shape(&Symbol::qualified("core", "Any")).unwrap();
    let parts = cx.factory().list(vec![any]).unwrap();
    let shape = cx
        .call_function(&Symbol::qualified("shape", "and"), Args::new(vec![parts]))
        .unwrap();

    let expr = shape.object().as_expr(&mut cx).unwrap();

    assert!(matches!(
        expr,
        Expr::Call { operator, .. }
            if *operator == Expr::Symbol(Symbol::qualified("shape", "and"))
    ));
}

#[cfg(feature = "codec-lisp")]
#[test]
fn runtime_shape_constructor_round_trips_through_lisp_codec() {
    use sim_codec::{Input, Output, decode_with_codec, encode_with_codec};
    use sim_codec_lisp::LispCodecLib;
    use sim_kernel::{EncodeOptions, ReadPolicy};

    let mut cx = cx();
    let codec_id = cx.registry_mut().fresh_codec_id();
    cx.load_lib(&LispCodecLib::new(codec_id).unwrap()).unwrap();
    let any = cx.resolve_shape(&Symbol::qualified("core", "Any")).unwrap();
    let parts = cx.factory().list(vec![any]).unwrap();
    let shape = cx
        .call_function(&Symbol::qualified("shape", "and"), Args::new(vec![parts]))
        .unwrap();
    let expr = shape.object().as_expr(&mut cx).unwrap();
    let output = encode_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "lisp"),
        &expr,
        EncodeOptions::default(),
    )
    .unwrap();
    let input = match output {
        Output::Text(text) => Input::Text(text),
        Output::Bytes(bytes) => Input::Bytes(bytes),
    };
    let decoded = decode_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "lisp"),
        input,
        ReadPolicy::default(),
    )
    .unwrap();
    let value = cx.eval_expr(decoded).unwrap();

    assert!(value.object().as_shape().is_some());
}
