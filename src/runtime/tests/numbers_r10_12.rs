#[cfg(all(
    feature = "numbers-numeric",
    feature = "numbers-func",
    feature = "numbers-f64"
))]
use std::sync::Arc;

#[cfg(all(
    feature = "numbers-numeric",
    feature = "numbers-func",
    feature = "numbers-f64"
))]
use sim_kernel::{Args, Error, Expr, NumberLiteral, QuoteMode, Symbol};

#[cfg(all(
    feature = "numbers-numeric",
    feature = "numbers-func",
    feature = "numbers-f64"
))]
use sim_lib_numbers_func::Func;

#[cfg(all(
    feature = "numbers-numeric",
    feature = "numbers-func",
    feature = "numbers-f64"
))]
use super::support::eval_cx;

#[cfg(all(
    feature = "numbers-numeric",
    feature = "numbers-func",
    feature = "numbers-f64"
))]
fn f64_number(text: &str) -> Expr {
    Expr::Number(NumberLiteral {
        domain: Symbol::qualified("numbers", "f64"),
        canonical: text.to_owned(),
    })
}

#[cfg(all(
    feature = "numbers-numeric",
    feature = "numbers-func",
    feature = "numbers-f64"
))]
fn quoted(name: &str) -> Expr {
    Expr::Quote {
        mode: QuoteMode::Quote,
        expr: Box::new(Expr::Symbol(Symbol::new(name))),
    }
}

#[cfg(all(
    feature = "numbers-numeric",
    feature = "numbers-func",
    feature = "numbers-f64"
))]
#[test]
fn unknown_numeric_method_errors_cleanly() {
    let mut cx = eval_cx();
    let err = cx
        .eval_expr(Expr::Call {
            operator: Box::new(Expr::Symbol(Symbol::new("numeric-diff"))),
            args: vec![
                Expr::Call {
                    operator: Box::new(Expr::Symbol(Symbol::new("fn"))),
                    args: vec![
                        Expr::List(vec![Expr::Symbol(Symbol::new("x"))]),
                        Expr::Symbol(Symbol::new("x")),
                    ],
                },
                quoted("x"),
                f64_number("2.0"),
                Expr::Symbol(Symbol::new(":method")),
                quoted("no-such-method"),
            ],
        })
        .unwrap_err();
    assert!(matches!(err, Error::Eval(message) if message.contains("UnknownNumericMethod")));
}

#[cfg(all(
    feature = "numbers-numeric",
    feature = "numbers-func",
    feature = "numbers-f64"
))]
#[test]
fn native_func_can_be_differentiated_numerically() {
    let mut cx = eval_cx();
    let func = cx
        .factory()
        .opaque(Arc::new(Func::native(
            vec![Symbol::new("x")],
            Arc::new(|cx, args| {
                let [x] = args else {
                    return Err(Error::Eval("expected one arg".to_owned()));
                };
                let x2 = cx.apply_value_number_binary_op(
                    &Symbol::qualified("math", "mul"),
                    x.clone(),
                    x.clone(),
                )?;
                cx.apply_value_number_binary_op(&Symbol::qualified("math", "add"), x2, x.clone())
            }),
        )))
        .unwrap();
    let out = cx
        .call_function(
            &Symbol::new("numeric-diff"),
            Args::new(vec![
                func,
                cx.factory().expr(quoted("x")).unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "3.0".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    let rendered = out
        .object()
        .display(&mut cx)
        .unwrap()
        .parse::<f64>()
        .unwrap();
    assert!((rendered - 7.0).abs() < 1.0e-3);
}

#[cfg(all(
    feature = "numbers-numeric",
    feature = "numbers-func",
    feature = "numbers-f64"
))]
#[test]
fn symbolic_func_prefers_symbolic_derivative() {
    let mut cx = eval_cx();
    let out = cx
        .eval_expr(Expr::Call {
            operator: Box::new(Expr::Symbol(Symbol::new("numeric-diff"))),
            args: vec![
                Expr::Call {
                    operator: Box::new(Expr::Symbol(Symbol::new("fn"))),
                    args: vec![
                        Expr::List(vec![Expr::Symbol(Symbol::new("x"))]),
                        Expr::Call {
                            operator: Box::new(Expr::Symbol(Symbol::new("+"))),
                            args: vec![
                                Expr::Call {
                                    operator: Box::new(Expr::Symbol(Symbol::new("*"))),
                                    args: vec![quoted("x"), quoted("x")],
                                },
                                quoted("x"),
                            ],
                        },
                    ],
                },
                quoted("x"),
                f64_number("3.0"),
            ],
        })
        .unwrap();
    assert_eq!(out.object().display(&mut cx).unwrap(), "7");
    let diagnostics = cx.take_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("method=auto"))
    );
}
