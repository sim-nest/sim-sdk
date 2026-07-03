#[cfg(any(feature = "numbers-rational", feature = "numbers-i64"))]
use std::sync::Arc;

#[cfg(any(feature = "numbers-rational", feature = "numbers-i64"))]
use crate::runtime::install_core_runtime;
#[cfg(any(feature = "numbers-rational", feature = "numbers-i64"))]
use sim_kernel::{
    Args, DefaultFactory, Expr, NoopEvalPolicy, NumberBinaryOp, NumberLiteral, PromotionRule,
    Symbol, ValueNumberBinaryOp, ValuePromotionRule,
};

#[cfg(any(feature = "numbers-rational", feature = "numbers-i64"))]
use super::number_dispatch_support::*;

#[cfg(feature = "numbers-rational")]
#[test]
fn numeric_dispatch_reports_ambiguous_promotion_routes() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    install_test_domain(&mut cx, Symbol::qualified("numbers", "decimal-test"));
    cx.registry_mut().register_promotion_rule(PromotionRule {
        from_domain: Symbol::qualified("numbers", "f64"),
        to_domain: Symbol::qualified("numbers", "decimal-test"),
        cost: 1,
        convert: promote_f64_to_decimal,
    });
    cx.registry_mut().register_promotion_rule(PromotionRule {
        from_domain: Symbol::qualified("numbers", "rational"),
        to_domain: Symbol::qualified("numbers", "decimal-test"),
        cost: 0,
        convert: promote_rational_to_decimal,
    });
    cx.registry_mut().register_number_binary_op(NumberBinaryOp {
        operator: Symbol::qualified("math", "add"),
        left_domain: Symbol::qualified("numbers", "decimal-test"),
        right_domain: Symbol::qualified("numbers", "decimal-test"),
        cost: 0,
        apply: decimal_add_rule,
    });
    let error = cx
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
        .unwrap_err();
    assert!(
        matches!(error, sim_kernel::Error::AmbiguousNumberDispatch { operator, candidates } if operator == Symbol::qualified("math", "add") && candidates.len() == 2)
    );
}

#[cfg(feature = "numbers-i64")]
#[test]
fn numeric_dispatch_finds_multi_hop_promotion_paths() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    install_test_domain(&mut cx, Symbol::qualified("numbers", "decimal-test"));
    cx.registry_mut().register_promotion_rule(PromotionRule {
        from_domain: Symbol::qualified("numbers", "f64"),
        to_domain: Symbol::qualified("numbers", "decimal-test"),
        cost: 1,
        convert: promote_f64_to_decimal,
    });
    cx.registry_mut().register_number_binary_op(NumberBinaryOp {
        operator: Symbol::qualified("math", "add"),
        left_domain: Symbol::qualified("numbers", "decimal-test"),
        right_domain: Symbol::qualified("numbers", "decimal-test"),
        cost: 0,
        apply: decimal_add_rule,
    });
    let value = cx
        .call_function(
            &Symbol::qualified("math", "add"),
            Args::new(vec![
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "i64"), "2".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(
                        Symbol::qualified("numbers", "decimal-test"),
                        "0.5".to_owned(),
                    )
                    .unwrap(),
            ]),
        )
        .unwrap();
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "decimal-test"),
            canonical: "2.5".to_owned()
        })
    );
}

#[cfg(feature = "numbers-i64")]
#[test]
fn opaque_number_value_participates_in_math_add() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    install_test_domain(&mut cx, Symbol::qualified("numbers", "opaque-start-test"));
    install_test_domain(&mut cx, Symbol::qualified("numbers", "opaque-middle-test"));
    install_test_domain(&mut cx, Symbol::qualified("numbers", "opaque-target-test"));
    cx.registry_mut()
        .register_value_promotion_rule(ValuePromotionRule {
            from_domain: Symbol::qualified("numbers", "opaque-start-test"),
            to_domain: Symbol::qualified("numbers", "opaque-middle-test"),
            cost: 1,
            convert: promote_opaque_start_to_middle,
        });
    cx.registry_mut()
        .register_value_promotion_rule(ValuePromotionRule {
            from_domain: Symbol::qualified("numbers", "opaque-middle-test"),
            to_domain: Symbol::qualified("numbers", "opaque-target-test"),
            cost: 1,
            convert: promote_opaque_middle_to_target,
        });
    cx.registry_mut()
        .register_value_number_binary_op(ValueNumberBinaryOp {
            operator: Symbol::qualified("math", "add"),
            left_domain: Symbol::qualified("numbers", "opaque-target-test"),
            right_domain: Symbol::qualified("numbers", "opaque-target-test"),
            cost: 0,
            apply: opaque_add_rule,
        });

    let value = cx
        .call_function(
            &Symbol::qualified("math", "add"),
            Args::new(vec![
                opaque_number_value(&cx, Symbol::qualified("numbers", "opaque-start-test"), 1.5),
                opaque_number_value(&cx, Symbol::qualified("numbers", "opaque-target-test"), 2.0),
            ]),
        )
        .unwrap();
    let out = read_opaque_number(&value);
    assert_eq!(
        out.domain,
        Symbol::qualified("numbers", "opaque-target-test")
    );
    assert_eq!(out.value, 3.5);
}

#[cfg(feature = "numbers-i64")]
#[test]
fn value_level_promotion_ambiguity_reports_both_best_domain_pairs() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    install_test_domain(&mut cx, Symbol::qualified("numbers", "opaque-start-test"));
    install_test_domain(&mut cx, Symbol::qualified("numbers", "opaque-middle-test"));
    install_test_domain(&mut cx, Symbol::qualified("numbers", "opaque-target-test"));
    install_test_domain(
        &mut cx,
        Symbol::qualified("numbers", "opaque-alt-target-test"),
    );
    cx.registry_mut()
        .register_value_promotion_rule(ValuePromotionRule {
            from_domain: Symbol::qualified("numbers", "opaque-start-test"),
            to_domain: Symbol::qualified("numbers", "opaque-middle-test"),
            cost: 1,
            convert: promote_opaque_start_to_middle,
        });
    cx.registry_mut()
        .register_value_promotion_rule(ValuePromotionRule {
            from_domain: Symbol::qualified("numbers", "opaque-middle-test"),
            to_domain: Symbol::qualified("numbers", "opaque-target-test"),
            cost: 1,
            convert: promote_opaque_middle_to_target,
        });
    cx.registry_mut()
        .register_value_promotion_rule(ValuePromotionRule {
            from_domain: Symbol::qualified("numbers", "opaque-start-test"),
            to_domain: Symbol::qualified("numbers", "opaque-alt-target-test"),
            cost: 2,
            convert: promote_opaque_start_to_alt_target,
        });
    cx.registry_mut()
        .register_value_number_binary_op(ValueNumberBinaryOp {
            operator: Symbol::qualified("math", "add"),
            left_domain: Symbol::qualified("numbers", "opaque-target-test"),
            right_domain: Symbol::qualified("numbers", "opaque-target-test"),
            cost: 0,
            apply: opaque_add_rule,
        });
    cx.registry_mut()
        .register_value_number_binary_op(ValueNumberBinaryOp {
            operator: Symbol::qualified("math", "add"),
            left_domain: Symbol::qualified("numbers", "opaque-alt-target-test"),
            right_domain: Symbol::qualified("numbers", "opaque-alt-target-test"),
            cost: 0,
            apply: opaque_add_alt_rule,
        });

    let error = cx
        .call_function(
            &Symbol::qualified("math", "add"),
            Args::new(vec![
                opaque_number_value(&cx, Symbol::qualified("numbers", "opaque-start-test"), 1.0),
                opaque_number_value(&cx, Symbol::qualified("numbers", "opaque-start-test"), 2.0),
            ]),
        )
        .unwrap_err();

    assert!(matches!(
        error,
        sim_kernel::Error::AmbiguousNumberDispatch { operator, candidates }
        if operator == Symbol::qualified("math", "add")
            && candidates.contains(&(
                Symbol::qualified("numbers", "opaque-target-test"),
                Symbol::qualified("numbers", "opaque-target-test")
            ))
            && candidates.contains(&(
                Symbol::qualified("numbers", "opaque-alt-target-test"),
                Symbol::qualified("numbers", "opaque-alt-target-test")
            ))
    ));
}

#[cfg(feature = "numbers-i64")]
#[test]
fn core_number_shape_accepts_opaque_number_values() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    install_test_domain(&mut cx, Symbol::qualified("numbers", "opaque-start-test"));
    let shape = cx
        .registry()
        .shape_by_symbol(&Symbol::qualified("core", "Number"))
        .unwrap()
        .clone();
    let value = opaque_number_value(&cx, Symbol::qualified("numbers", "opaque-start-test"), 4.0);
    let matched = shape
        .object()
        .as_shape()
        .unwrap()
        .check_value(&mut cx, value)
        .unwrap();
    assert!(matched.accepted);
}

#[cfg(feature = "numbers-i64")]
#[test]
fn non_number_value_passed_to_math_add_reports_error() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let error = cx
        .call_function(
            &Symbol::qualified("math", "add"),
            Args::new(vec![
                cx.factory().string("oops".to_owned()).unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "i64"), "1".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap_err();
    assert!(matches!(
        error,
        sim_kernel::Error::TypeMismatch { .. }
            | sim_kernel::Error::WrongShape { .. }
            | sim_kernel::Error::NoMatchingOverload { .. }
            | sim_kernel::Error::Eval(_)
    ));
}
