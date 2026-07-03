use std::sync::Arc;

#[cfg(feature = "numbers-complex")]
use sim_kernel::Args;
use sim_kernel::{DefaultFactory, Expr, NoopEvalPolicy, NumberLiteral, Symbol};

use crate::runtime::install_core_runtime;

use super::support::table_value;

#[cfg(feature = "numbers-f64")]
#[test]
fn number_domain_lib_exports_literal_class_and_shape_surfaces() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let domain = cx
        .resolve_number_domain(&Symbol::qualified("numbers", "f64"))
        .unwrap();
    let domain_table = domain.object().as_table(&mut cx).unwrap();
    let domain_expr = domain_table.object().as_expr(&mut cx).unwrap();
    assert!(
        cx.registry()
            .class_by_symbol(&Symbol::qualified("numbers", "f64-literal"))
            .is_some()
    );
    assert!(
        cx.registry()
            .shape_by_symbol(&Symbol::qualified("numbers/f64-literal", "instance-shape"))
            .is_some()
    );
    assert_eq!(
        table_value(&domain_expr, &Symbol::new("literal-class")),
        Some(&Expr::Symbol(Symbol::qualified("numbers", "f64-literal")))
    );
}

#[cfg(all(feature = "numbers-arith", feature = "numbers-f64"))]
#[test]
fn f64_domain_browse_table_includes_value_shape() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let domain = cx
        .resolve_number_domain(&Symbol::qualified("numbers", "f64"))
        .unwrap();
    let domain_table = domain.object().as_table(&mut cx).unwrap();
    let domain_expr = domain_table.object().as_expr(&mut cx).unwrap();
    assert_eq!(
        table_value(&domain_expr, &Symbol::new("value-shape")),
        Some(&Expr::Symbol(Symbol::qualified(
            "numbers/f64",
            "value-shape"
        )))
    );
}

#[cfg(all(feature = "numbers-arith", feature = "numbers-i64"))]
#[test]
fn i64_domain_browse_table_includes_value_shape() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let domain = cx
        .resolve_number_domain(&Symbol::qualified("numbers", "i64"))
        .unwrap();
    let domain_table = domain.object().as_table(&mut cx).unwrap();
    let domain_expr = domain_table.object().as_expr(&mut cx).unwrap();
    assert_eq!(
        table_value(&domain_expr, &Symbol::new("value-shape")),
        Some(&Expr::Symbol(Symbol::qualified(
            "numbers/i64",
            "value-shape"
        )))
    );
}

#[cfg(all(feature = "numbers-arith", feature = "numbers-rational"))]
#[test]
fn rational_domain_browse_table_includes_value_shape() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let domain = cx
        .resolve_number_domain(&Symbol::qualified("numbers", "rational"))
        .unwrap();
    let domain_table = domain.object().as_table(&mut cx).unwrap();
    let domain_expr = domain_table.object().as_expr(&mut cx).unwrap();
    assert_eq!(
        table_value(&domain_expr, &Symbol::new("value-shape")),
        Some(&Expr::Symbol(Symbol::qualified(
            "numbers/rational",
            "value-shape"
        )))
    );
}

#[cfg(all(feature = "numbers-arith", feature = "numbers-complex"))]
#[test]
fn complex_domain_browse_table_includes_value_shape() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let domain = cx
        .resolve_number_domain(&Symbol::qualified("numbers", "complex"))
        .unwrap();
    let domain_table = domain.object().as_table(&mut cx).unwrap();
    let domain_expr = domain_table.object().as_expr(&mut cx).unwrap();
    assert_eq!(
        table_value(&domain_expr, &Symbol::new("value-shape")),
        Some(&Expr::Symbol(Symbol::qualified(
            "numbers/complex",
            "value-shape"
        )))
    );
}

#[cfg(all(feature = "numbers-arith", feature = "numbers-f64"))]
#[test]
fn f64_literal_and_value_dispatch_match() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let left = cx
        .factory()
        .number_literal(Symbol::qualified("numbers", "f64"), "1.25".to_owned())
        .unwrap();
    let right = cx
        .factory()
        .number_literal(Symbol::qualified("numbers", "f64"), "2.5".to_owned())
        .unwrap();
    let literal = cx
        .apply_number_binary_op(
            &Symbol::qualified("math", "add"),
            NumberLiteral {
                domain: Symbol::qualified("numbers", "f64"),
                canonical: "1.25".to_owned(),
            },
            NumberLiteral {
                domain: Symbol::qualified("numbers", "f64"),
                canonical: "2.5".to_owned(),
            },
        )
        .unwrap();
    let value = cx
        .apply_value_number_binary_op(&Symbol::qualified("math", "add"), left, right)
        .unwrap();
    assert_eq!(
        literal.object().as_expr(&mut cx).unwrap(),
        value.object().as_expr(&mut cx).unwrap()
    );
}

#[cfg(all(feature = "numbers-arith", feature = "numbers-i64"))]
#[test]
fn i64_literal_and_value_dispatch_match() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let left = cx
        .factory()
        .number_literal(Symbol::qualified("numbers", "i64"), "4".to_owned())
        .unwrap();
    let right = cx
        .factory()
        .number_literal(Symbol::qualified("numbers", "i64"), "3".to_owned())
        .unwrap();
    let literal = cx
        .apply_number_binary_op(
            &Symbol::qualified("math", "mul"),
            NumberLiteral {
                domain: Symbol::qualified("numbers", "i64"),
                canonical: "4".to_owned(),
            },
            NumberLiteral {
                domain: Symbol::qualified("numbers", "i64"),
                canonical: "3".to_owned(),
            },
        )
        .unwrap();
    let value = cx
        .apply_value_number_binary_op(&Symbol::qualified("math", "mul"), left, right)
        .unwrap();
    assert_eq!(
        literal.object().as_expr(&mut cx).unwrap(),
        value.object().as_expr(&mut cx).unwrap()
    );
}

#[cfg(all(feature = "numbers-arith", feature = "numbers-rational"))]
#[test]
fn rational_literal_and_value_dispatch_match() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let left = cx
        .factory()
        .number_literal(Symbol::qualified("numbers", "rational"), "1/2".to_owned())
        .unwrap();
    let right = cx
        .factory()
        .number_literal(Symbol::qualified("numbers", "rational"), "1/3".to_owned())
        .unwrap();
    let literal = cx
        .apply_number_binary_op(
            &Symbol::qualified("math", "add"),
            NumberLiteral {
                domain: Symbol::qualified("numbers", "rational"),
                canonical: "1/2".to_owned(),
            },
            NumberLiteral {
                domain: Symbol::qualified("numbers", "rational"),
                canonical: "1/3".to_owned(),
            },
        )
        .unwrap();
    let value = cx
        .apply_value_number_binary_op(&Symbol::qualified("math", "add"), left, right)
        .unwrap();
    assert_eq!(
        literal.object().as_expr(&mut cx).unwrap(),
        value.object().as_expr(&mut cx).unwrap()
    );
}

#[cfg(all(feature = "numbers-arith", feature = "numbers-complex"))]
#[test]
fn complex_literal_and_value_dispatch_match() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let left = cx
        .factory()
        .number_literal(Symbol::qualified("numbers", "complex"), "1+2i".to_owned())
        .unwrap();
    let right = cx
        .factory()
        .number_literal(Symbol::qualified("numbers", "complex"), "3+4i".to_owned())
        .unwrap();
    let literal = cx
        .apply_number_binary_op(
            &Symbol::qualified("math", "mul"),
            NumberLiteral {
                domain: Symbol::qualified("numbers", "complex"),
                canonical: "1+2i".to_owned(),
            },
            NumberLiteral {
                domain: Symbol::qualified("numbers", "complex"),
                canonical: "3+4i".to_owned(),
            },
        )
        .unwrap();
    let value = cx
        .apply_value_number_binary_op(&Symbol::qualified("math", "mul"), left, right)
        .unwrap();
    assert_eq!(
        literal.object().as_expr(&mut cx).unwrap(),
        value.object().as_expr(&mut cx).unwrap()
    );
}

#[cfg(all(feature = "numbers-f64", feature = "numbers-complex"))]
#[test]
fn mixed_f64_and_complex_addition_promotes_to_complex() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let value = cx
        .call_function(
            &Symbol::qualified("math", "add"),
            Args::new(vec![
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "1.5".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "complex"), "0.5+2i".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "complex"),
            canonical: "2+2i".to_owned()
        })
    );
}

#[cfg(feature = "numbers-complex")]
#[test]
fn complex_reduction_product_is_registered() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let value = cx
        .call_function(
            &Symbol::qualified("math", "product"),
            Args::new(vec![
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "complex"), "1+2i".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "complex"), "3+4i".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "complex"),
            canonical: "-5+10i".to_owned()
        })
    );
}
