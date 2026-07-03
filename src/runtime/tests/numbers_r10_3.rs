#[cfg(feature = "numbers-i64")]
use std::sync::Arc;

#[cfg(feature = "numbers-i64")]
use sim_kernel::{Args, DefaultFactory, Expr, NoopEvalPolicy, NumberLiteral, Symbol};

#[cfg(feature = "numbers-i64")]
use crate::runtime::install_core_runtime;

#[cfg(all(feature = "numbers-bool", feature = "numbers-fixed"))]
use super::support::table_value;

#[cfg(all(feature = "numbers-bool", feature = "numbers-fixed"))]
#[test]
fn bool_domain_browse_exposes_value_shape() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let domain = cx
        .resolve_number_domain(&Symbol::qualified("numbers", "bool"))
        .unwrap()
        .clone();
    let domain_table = domain.object().as_table(&mut cx).unwrap();
    let domain_expr = domain_table.object().as_expr(&mut cx).unwrap();
    assert_eq!(
        table_value(&domain_expr, &Symbol::new("value-shape")),
        Some(&Expr::Symbol(Symbol::qualified(
            "numbers/bool",
            "value-shape"
        )))
    );
}

#[cfg(all(feature = "numbers-bool", feature = "numbers-fixed"))]
#[test]
fn bool_arithmetic_and_promotion_dispatches() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let bool_sum = cx
        .call_function(
            &Symbol::qualified("math", "add"),
            Args::new(vec![
                cx.factory().bool(true).unwrap(),
                cx.factory().bool(false).unwrap(),
            ]),
        )
        .unwrap();
    assert_eq!(
        bool_sum.object().as_expr(&mut cx).unwrap(),
        Expr::Bool(true)
    );

    let bool_product = cx
        .call_function(
            &Symbol::qualified("math", "mul"),
            Args::new(vec![
                cx.factory().bool(true).unwrap(),
                cx.factory().bool(true).unwrap(),
            ]),
        )
        .unwrap();
    assert_eq!(
        bool_product.object().as_expr(&mut cx).unwrap(),
        Expr::Bool(true)
    );

    let promoted = cx
        .call_function(
            &Symbol::qualified("math", "add"),
            Args::new(vec![
                cx.factory().bool(true).unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "i64"), "1".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    assert_eq!(
        promoted.object().as_expr(&mut cx).unwrap(),
        Expr::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "i64"),
            canonical: "2".to_owned()
        })
    );
}

#[cfg(all(feature = "numbers-i64", not(feature = "numbers-rational")))]
#[test]
fn i64_division_stays_integer_without_rational() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let value = cx
        .call_function(
            &Symbol::qualified("math", "div"),
            Args::new(vec![
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "i64"), "1".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "i64"), "2".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "i64"),
            canonical: "0".to_owned()
        })
    );
}

#[cfg(all(feature = "numbers-i64", feature = "numbers-rational"))]
#[test]
fn i64_division_prefers_rational_when_installed() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let value = cx
        .call_function(
            &Symbol::qualified("math", "div"),
            Args::new(vec![
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "i64"), "1".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "i64"), "2".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "rational"),
            canonical: "1/2".to_owned()
        })
    );
}

#[cfg(all(feature = "numbers-bigint", feature = "numbers-i64"))]
#[test]
fn overflowing_i64_multiplication_yields_bigint() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let value = cx
        .call_function(
            &Symbol::qualified("math", "mul"),
            Args::new(vec![
                cx.factory()
                    .number_literal(
                        Symbol::qualified("numbers", "i64"),
                        "1000000000000".to_owned(),
                    )
                    .unwrap(),
                cx.factory()
                    .number_literal(
                        Symbol::qualified("numbers", "i64"),
                        "1000000000000".to_owned(),
                    )
                    .unwrap(),
            ]),
        )
        .unwrap();
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "bigint"),
            canonical: "1000000000000000000000000".to_owned()
        })
    );
}

#[cfg(all(feature = "numbers-bigint", feature = "numbers-i64"))]
#[test]
fn i64_pow_overflow_promotes_to_bigint() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let power = cx
        .call_function(
            &Symbol::qualified("math", "pow"),
            Args::new(vec![
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "i64"), "2".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "i64"), "200".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    let value = cx
        .call_function(
            &Symbol::qualified("math", "add"),
            Args::new(vec![
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "i64"), "1".to_owned())
                    .unwrap(),
                power,
            ]),
        )
        .unwrap();
    let Expr::Number(number) = value.object().as_expr(&mut cx).unwrap() else {
        panic!("expected bigint result");
    };
    assert_eq!(number.domain, Symbol::qualified("numbers", "bigint"));
    assert!(
        number
            .canonical
            .starts_with("1606938044258990275541962092341162602522202993782792835301377")
    );
}
