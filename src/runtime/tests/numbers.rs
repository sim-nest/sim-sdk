use std::sync::Arc;

use sim_kernel::{Args, DefaultFactory, Expr, NoopEvalPolicy, NumberLiteral, Symbol};

use crate::runtime::install_core_runtime;

use super::support::table_value;

#[cfg(feature = "numbers-f64")]
#[test]
fn installs_f64_domain_owned_math_functions() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let value = cx
        .call_function(
            &Symbol::qualified("math", "add"),
            Args::new(vec![
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "1.25".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "2.5".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "f64"),
            canonical: "3.75".to_owned()
        })
    );
}

#[cfg(all(feature = "numbers-f64", not(feature = "numbers-rational")))]
#[test]
fn f64_math_functions_reject_other_number_domains() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let error = cx
        .call_function(
            &Symbol::qualified("math", "add"),
            Args::new(vec![
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "rational"), "1/3".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "2".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap_err();
    assert!(
        matches!(error, sim_kernel::Error::Eval(message) if message.contains("uses unloaded number domain numbers/rational"))
    );
}

#[cfg(all(feature = "numbers-f64", feature = "numbers-rational"))]
#[test]
fn mixed_f64_and_rational_addition_promotes_to_rational() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let value = cx
        .call_function(
            &Symbol::qualified("math", "add"),
            Args::new(vec![
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "0.25".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "rational"), "1/2".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "rational"),
            canonical: "3/4".to_owned()
        })
    );
}

#[cfg(all(
    feature = "numbers-f64",
    feature = "numbers-i64",
    feature = "numbers-rational"
))]
#[test]
fn n_ary_addition_folds_through_numeric_dispatch() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let value = cx
        .call_function(
            &Symbol::qualified("math", "add"),
            Args::new(vec![
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "i64"), "1".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "rational"), "1/2".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "0.25".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "rational"),
            canonical: "7/4".to_owned()
        })
    );
}

#[cfg(all(
    feature = "numbers-f64",
    feature = "numbers-i64",
    feature = "numbers-rational"
))]
#[test]
fn n_ary_subtraction_folds_through_numeric_dispatch() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let value = cx
        .call_function(
            &Symbol::qualified("math", "sub"),
            Args::new(vec![
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "i64"), "5".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "rational"), "1/2".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "0.25".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "rational"),
            canonical: "17/4".to_owned()
        })
    );
}

#[cfg(all(
    feature = "numbers-f64",
    feature = "numbers-i64",
    feature = "numbers-rational"
))]
#[test]
fn n_ary_multiplication_folds_through_numeric_dispatch() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let value = cx
        .call_function(
            &Symbol::qualified("math", "mul"),
            Args::new(vec![
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "i64"), "2".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "rational"), "3/2".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "0.25".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "rational"),
            canonical: "3/4".to_owned()
        })
    );
}

#[cfg(all(
    feature = "numbers-f64",
    feature = "numbers-i64",
    feature = "numbers-rational"
))]
#[test]
fn n_ary_division_folds_through_numeric_dispatch() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let value = cx
        .call_function(
            &Symbol::qualified("math", "div"),
            Args::new(vec![
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "i64"), "8".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "rational"), "1/2".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "0.25".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "rational"),
            canonical: "64/1".to_owned()
        })
    );
}

#[cfg(feature = "numbers-i64")]
#[test]
fn unary_negation_dispatches_through_registered_number_rules() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let value = cx
        .call_function(
            &Symbol::qualified("math", "neg"),
            Args::new(vec![
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
            canonical: "-2".to_owned()
        })
    );
}

#[cfg(all(
    feature = "numbers-f64",
    feature = "numbers-i64",
    feature = "numbers-rational"
))]
#[test]
fn reduction_ops_dispatch_through_registered_number_rules() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let sum = cx
        .call_function(
            &Symbol::qualified("math", "sum"),
            Args::new(vec![
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "i64"), "1".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "rational"), "1/2".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "0.25".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    let product = cx
        .call_function(
            &Symbol::qualified("math", "product"),
            Args::new(vec![
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "i64"), "2".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "rational"), "3/2".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "0.25".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    assert_eq!(
        sum.object().as_expr(&mut cx).unwrap(),
        Expr::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "rational"),
            canonical: "7/4".to_owned()
        })
    );
    assert_eq!(
        product.object().as_expr(&mut cx).unwrap(),
        Expr::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "rational"),
            canonical: "3/4".to_owned()
        })
    );
}

#[cfg(all(feature = "numbers-arith", feature = "numbers-f64"))]
#[test]
fn numeric_metadata_is_browseable() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let domain_table = cx
        .resolve_number_domain(&Symbol::qualified("numbers", "f64"))
        .unwrap()
        .object()
        .as_table(&mut cx)
        .unwrap();
    let function_table = cx
        .resolve_function(&Symbol::qualified("math", "sum"))
        .unwrap()
        .object()
        .as_table(&mut cx)
        .unwrap();
    let domain_expr = domain_table.object().as_expr(&mut cx).unwrap();
    let function_expr = function_table.object().as_expr(&mut cx).unwrap();
    assert_eq!(
        table_value(&domain_expr, &Symbol::new("numeric-family")),
        Some(&Expr::String("real".to_owned()))
    );
    assert_eq!(
        table_value(&domain_expr, &Symbol::new("canonical-form")),
        Some(&Expr::String("f64".to_owned()))
    );
    assert_eq!(table_value(&function_expr, &Symbol::new("dispatch")), None);
}
