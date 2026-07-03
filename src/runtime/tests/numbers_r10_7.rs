#[cfg(all(
    feature = "numbers-cas",
    feature = "numbers-cas-diff",
    feature = "numbers-cas-eval",
    feature = "numbers-i64"
))]
use sim_kernel::{Args, Expr, NumberLiteral, QuoteMode, Symbol};
#[cfg(all(
    feature = "numbers-cas",
    feature = "numbers-cas-diff",
    feature = "numbers-cas-eval",
    feature = "numbers-i64"
))]
use sim_lib_numbers_cas::CasExpr;

#[cfg(all(
    feature = "numbers-cas",
    feature = "numbers-cas-diff",
    feature = "numbers-cas-eval",
    feature = "numbers-i64"
))]
use super::support::eval_cx;

#[cfg(all(
    feature = "numbers-cas",
    feature = "numbers-cas-diff",
    feature = "numbers-cas-eval",
    feature = "numbers-i64"
))]
fn quoted(name: &str) -> Expr {
    Expr::Quote {
        mode: QuoteMode::Quote,
        expr: Box::new(Expr::Symbol(Symbol::new(name))),
    }
}

#[cfg(all(
    feature = "numbers-cas",
    feature = "numbers-cas-diff",
    feature = "numbers-cas-eval",
    feature = "numbers-i64"
))]
fn polynomial_expr() -> Expr {
    Expr::Call {
        operator: Box::new(Expr::Symbol(Symbol::new("+"))),
        args: vec![
            Expr::Call {
                operator: Box::new(Expr::Symbol(Symbol::new("*"))),
                args: vec![
                    Expr::Number(NumberLiteral {
                        domain: Symbol::qualified("numbers", "i64"),
                        canonical: "2".to_owned(),
                    }),
                    quoted("x"),
                ],
            },
            Expr::Call {
                operator: Box::new(Expr::Symbol(Symbol::new("^"))),
                args: vec![
                    quoted("x"),
                    Expr::Number(NumberLiteral {
                        domain: Symbol::qualified("numbers", "i64"),
                        canonical: "2".to_owned(),
                    }),
                ],
            },
        ],
    }
}

#[cfg(all(
    feature = "numbers-cas",
    feature = "numbers-cas-diff",
    feature = "numbers-cas-eval",
    feature = "numbers-i64"
))]
#[test]
fn diff_polynomial_matches_simplified_surface() {
    let mut cx = eval_cx();
    let polynomial = cx.eval_expr(polynomial_expr()).unwrap();
    let var = cx.factory().expr(quoted("x")).unwrap();
    let value = cx
        .call_function(&Symbol::new("diff"), Args::new(vec![polynomial, var]))
        .unwrap();
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::List(vec![
            Expr::Symbol(Symbol::new("+")),
            Expr::Number(NumberLiteral {
                domain: Symbol::qualified("numbers", "i64"),
                canonical: "2".to_owned(),
            }),
            Expr::List(vec![
                Expr::Symbol(Symbol::new("*")),
                Expr::Number(NumberLiteral {
                    domain: Symbol::qualified("numbers", "i64"),
                    canonical: "2".to_owned(),
                }),
                Expr::Symbol(Symbol::new("x")),
            ]),
        ])
    );
}

#[cfg(all(
    feature = "numbers-cas",
    feature = "numbers-cas-diff",
    feature = "numbers-cas-eval",
    feature = "numbers-i64"
))]
#[test]
fn diff_sin_is_cos() {
    let mut cx = eval_cx();
    let derivative = sim_lib_numbers_cas_diff::diff_cas(
        &mut cx,
        &CasExpr::Op(Symbol::new("sin"), vec![CasExpr::Var(Symbol::new("x"))]),
        &Symbol::new("x"),
    )
    .unwrap();
    let value = sim_lib_numbers_cas::cas_expr_to_value(&mut cx, derivative).unwrap();
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::List(vec![
            Expr::Symbol(Symbol::new("cos")),
            Expr::Symbol(Symbol::new("x")),
        ])
    );
}

#[cfg(all(
    feature = "numbers-cas",
    feature = "numbers-cas-diff",
    feature = "numbers-cas-eval",
    feature = "numbers-i64"
))]
#[test]
fn diff_product_with_respect_to_x_is_y() {
    let mut cx = eval_cx();
    let product = cx
        .eval_expr(Expr::Call {
            operator: Box::new(Expr::Symbol(Symbol::new("*"))),
            args: vec![quoted("x"), quoted("y")],
        })
        .unwrap();
    let var = cx.factory().expr(quoted("x")).unwrap();
    let value = cx
        .call_function(&Symbol::new("diff"), Args::new(vec![product, var]))
        .unwrap();
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::Symbol(Symbol::new("y"))
    );
}

#[cfg(all(
    feature = "numbers-cas",
    feature = "numbers-cas-diff",
    feature = "numbers-cas-eval",
    feature = "numbers-i64"
))]
#[test]
fn eval_cas_uses_current_env_and_derivative_evaluates() {
    let mut cx = eval_cx();
    let polynomial = cx.eval_expr(polynomial_expr()).unwrap();
    let var = cx.factory().expr(quoted("x")).unwrap();
    let derivative = cx
        .call_function(&Symbol::new("diff"), Args::new(vec![polynomial, var]))
        .unwrap();
    let x = cx
        .factory()
        .number_literal(Symbol::qualified("numbers", "i64"), "3".to_owned())
        .unwrap();
    cx.env_mut().define(Symbol::new("x"), x);
    let value = cx
        .call_function(&Symbol::new("eval-cas"), Args::new(vec![derivative]))
        .unwrap();
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "i64"),
            canonical: "8".to_owned(),
        })
    );
}
