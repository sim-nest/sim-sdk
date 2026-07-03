#[cfg(all(
    feature = "numbers-func",
    feature = "numbers-i64",
    feature = "numbers-cas-diff",
    feature = "numbers-cas-eval"
))]
use std::sync::Arc;

#[cfg(all(
    feature = "numbers-func",
    feature = "numbers-i64",
    feature = "numbers-cas-diff",
    feature = "numbers-cas-eval"
))]
use sim_kernel::{Args, Error, Expr, NumberLiteral, QuoteMode, Symbol};

#[cfg(all(
    feature = "numbers-func",
    feature = "numbers-i64",
    feature = "numbers-cas-diff",
    feature = "numbers-cas-eval"
))]
use sim_lib_numbers_func::Func;

#[cfg(all(
    feature = "numbers-func",
    feature = "numbers-i64",
    feature = "numbers-cas-diff",
    feature = "numbers-cas-eval"
))]
use super::support::eval_cx;

#[cfg(all(
    feature = "numbers-func",
    feature = "numbers-i64",
    feature = "numbers-cas-diff",
    feature = "numbers-cas-eval"
))]
fn number(text: &str) -> Expr {
    Expr::Number(NumberLiteral {
        domain: Symbol::qualified("numbers", "i64"),
        canonical: text.to_owned(),
    })
}

#[cfg(all(
    feature = "numbers-func",
    feature = "numbers-i64",
    feature = "numbers-cas-diff",
    feature = "numbers-cas-eval"
))]
fn quoted(name: &str) -> Expr {
    Expr::Quote {
        mode: QuoteMode::Quote,
        expr: Box::new(Expr::Symbol(Symbol::new(name))),
    }
}

#[cfg(all(
    feature = "numbers-func",
    feature = "numbers-i64",
    feature = "numbers-cas-diff",
    feature = "numbers-cas-eval"
))]
#[test]
fn call_surface_invokes_func_values() {
    let mut cx = eval_cx();
    let out = cx
        .eval_expr(Expr::Call {
            operator: Box::new(Expr::Symbol(Symbol::new("call"))),
            args: vec![
                Expr::Call {
                    operator: Box::new(Expr::Symbol(Symbol::new("fn"))),
                    args: vec![
                        Expr::List(vec![
                            Expr::Symbol(Symbol::new("x")),
                            Expr::Symbol(Symbol::new("y")),
                        ]),
                        Expr::Call {
                            operator: Box::new(Expr::Symbol(Symbol::new("+"))),
                            args: vec![
                                Expr::Symbol(Symbol::new("x")),
                                Expr::Symbol(Symbol::new("y")),
                            ],
                        },
                    ],
                },
                number("2"),
                number("3"),
            ],
        })
        .unwrap();
    assert_eq!(out.object().as_expr(&mut cx).unwrap(), number("5"));
}

#[cfg(all(
    feature = "numbers-func",
    feature = "numbers-i64",
    feature = "numbers-cas-diff",
    feature = "numbers-cas-eval"
))]
#[test]
fn diff_returns_callable_func_value() {
    let mut cx = eval_cx();
    let out = cx
        .eval_expr(Expr::Call {
            operator: Box::new(Expr::Call {
                operator: Box::new(Expr::Symbol(Symbol::new("diff"))),
                args: vec![
                    Expr::Call {
                        operator: Box::new(Expr::Symbol(Symbol::new("fn"))),
                        args: vec![
                            Expr::List(vec![Expr::Symbol(Symbol::new("x"))]),
                            Expr::Call {
                                operator: Box::new(Expr::Symbol(Symbol::new("*"))),
                                args: vec![quoted("x"), quoted("x")],
                            },
                        ],
                    },
                    quoted("x"),
                ],
            }),
            args: vec![number("3")],
        })
        .unwrap();
    assert_eq!(out.object().as_expr(&mut cx).unwrap(), number("6"));
}

#[cfg(all(
    feature = "numbers-func",
    feature = "numbers-i64",
    feature = "numbers-cas-diff",
    feature = "numbers-cas-eval"
))]
#[test]
fn arithmetic_on_funcs_produces_callable_values() {
    let mut cx = eval_cx();
    let func = cx
        .eval_expr(Expr::Call {
            operator: Box::new(Expr::Symbol(Symbol::new("fn"))),
            args: vec![
                Expr::List(vec![Expr::Symbol(Symbol::new("x"))]),
                Expr::Symbol(Symbol::new("x")),
            ],
        })
        .unwrap();
    let plus = cx
        .call_function(
            &Symbol::new("+"),
            Args::new(vec![
                func,
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "i64"), "1".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    let out = cx
        .call_value(
            plus,
            Args::new(vec![
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "i64"), "4".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    assert_eq!(out.object().as_expr(&mut cx).unwrap(), number("5"));
}

#[cfg(all(
    feature = "numbers-func",
    feature = "numbers-i64",
    feature = "numbers-cas-diff",
    feature = "numbers-cas-eval"
))]
#[test]
fn native_only_funcs_report_not_differentiable() {
    let mut cx = eval_cx();
    let native = cx
        .factory()
        .opaque(Arc::new(Func::native(
            vec![Symbol::new("x")],
            Arc::new(|cx, args| {
                let [value] = args else {
                    return Err(Error::Eval("expected one arg".to_owned()));
                };
                cx.apply_value_number_binary_op(
                    &Symbol::qualified("math", "add"),
                    value.clone(),
                    cx.factory()
                        .number_literal(Symbol::qualified("numbers", "i64"), "1".to_owned())?,
                )
            }),
        )))
        .unwrap();
    let err = cx
        .call_function(
            &Symbol::new("diff"),
            Args::new(vec![native, cx.factory().expr(quoted("x")).unwrap()]),
        )
        .unwrap_err();
    assert!(matches!(err, Error::Eval(message) if message.contains("NotDifferentiable")));
}
