#[cfg(all(
    feature = "numbers-f64",
    feature = "numbers-func",
    feature = "numbers-numeric",
    feature = "numbers-quad",
    feature = "numbers-rk"
))]
use std::sync::Arc;

#[cfg(all(
    feature = "numbers-f64",
    feature = "numbers-func",
    feature = "numbers-numeric",
    feature = "numbers-quad",
    feature = "numbers-rk"
))]
use sim_kernel::{Args, Error, Expr, NumberLiteral, QuoteMode, Symbol};

#[cfg(all(
    feature = "numbers-f64",
    feature = "numbers-func",
    feature = "numbers-numeric",
    feature = "numbers-quad",
    feature = "numbers-rk"
))]
use sim_lib_numbers_func::Func;

#[cfg(all(
    feature = "numbers-f64",
    feature = "numbers-func",
    feature = "numbers-numeric",
    feature = "numbers-quad",
    feature = "numbers-rk"
))]
use super::support::eval_cx;

#[cfg(all(
    feature = "numbers-f64",
    feature = "numbers-func",
    feature = "numbers-numeric",
    feature = "numbers-quad",
    feature = "numbers-rk"
))]
fn f64_number(text: &str) -> Expr {
    Expr::Number(NumberLiteral {
        domain: Symbol::qualified("numbers", "f64"),
        canonical: text.to_owned(),
    })
}

#[cfg(all(
    feature = "numbers-f64",
    feature = "numbers-func",
    feature = "numbers-numeric",
    feature = "numbers-quad",
    feature = "numbers-rk"
))]
fn quoted(name: &str) -> Expr {
    Expr::Quote {
        mode: QuoteMode::Quote,
        expr: Box::new(Expr::Symbol(Symbol::new(name))),
    }
}

#[cfg(all(
    feature = "numbers-f64",
    feature = "numbers-func",
    feature = "numbers-numeric",
    feature = "numbers-quad",
    feature = "numbers-rk"
))]
#[test]
fn simpson_integrates_quadratic_close_to_one_third() {
    let mut cx = eval_cx();
    let out = cx
        .eval_expr(Expr::Call {
            operator: Box::new(Expr::Symbol(Symbol::new("integrate"))),
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
                f64_number("0.0"),
                f64_number("1.0"),
                Expr::Symbol(Symbol::new(":method")),
                quoted("simpson"),
                Expr::Symbol(Symbol::new(":n")),
                f64_number("100"),
            ],
        })
        .unwrap();
    let value = out
        .object()
        .display(&mut cx)
        .unwrap()
        .parse::<f64>()
        .unwrap();
    assert!((value - (1.0 / 3.0)).abs() < 1.0e-8);
}

#[cfg(all(
    feature = "numbers-f64",
    feature = "numbers-func",
    feature = "numbers-numeric",
    feature = "numbers-quad",
    feature = "numbers-rk"
))]
#[test]
fn central_five_numeric_diff_handles_native_sine() {
    let mut cx = eval_cx();
    let func = cx
        .factory()
        .opaque(Arc::new(Func::native(
            vec![Symbol::new("x")],
            Arc::new(|cx, args| {
                let [x] = args else {
                    return Err(Error::Eval("expected one arg".to_owned()));
                };
                let value = x
                    .object()
                    .display(cx)
                    .unwrap()
                    .parse::<f64>()
                    .unwrap()
                    .sin();
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), value.to_string())
            }),
        )))
        .unwrap();
    let out = cx
        .call_function(
            &Symbol::new("numeric-diff"),
            Args::new(vec![
                func,
                cx.factory().symbol(Symbol::new("x")).unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "0.0".to_owned())
                    .unwrap(),
                cx.factory()
                    .table(vec![
                        (
                            Symbol::new(":method"),
                            cx.factory().symbol(Symbol::new("central-5")).unwrap(),
                        ),
                        (
                            Symbol::new(":h"),
                            cx.factory()
                                .number_literal(
                                    Symbol::qualified("numbers", "f64"),
                                    "1e-4".to_owned(),
                                )
                                .unwrap(),
                        ),
                    ])
                    .unwrap(),
            ]),
        )
        .unwrap();
    let value = out
        .object()
        .display(&mut cx)
        .unwrap()
        .parse::<f64>()
        .unwrap();
    assert!((value - 1.0).abs() < 1.0e-8);
}

#[cfg(all(
    feature = "numbers-f64",
    feature = "numbers-func",
    feature = "numbers-numeric",
    feature = "numbers-quad",
    feature = "numbers-rk"
))]
#[test]
fn rkf45_solves_exp_growth_close_to_e() {
    let mut cx = eval_cx();
    let out = cx
        .eval_expr(Expr::Call {
            operator: Box::new(Expr::Symbol(Symbol::new("ode-solve"))),
            args: vec![
                Expr::Call {
                    operator: Box::new(Expr::Symbol(Symbol::new("fn"))),
                    args: vec![
                        Expr::List(vec![
                            Expr::Symbol(Symbol::new("x")),
                            Expr::Symbol(Symbol::new("y")),
                        ]),
                        quoted("y"),
                    ],
                },
                quoted("x"),
                quoted("y"),
                f64_number("0.0"),
                f64_number("1.0"),
                f64_number("1.0"),
                Expr::Symbol(Symbol::new(":method")),
                quoted("rkf45"),
                Expr::Symbol(Symbol::new(":tol")),
                f64_number("1e-8"),
            ],
        })
        .unwrap();
    let expr = out.object().as_expr(&mut cx).unwrap();
    let last_y = match expr {
        Expr::List(points) => match points.last().unwrap() {
            Expr::List(pair) => match &pair[1] {
                Expr::Number(number) => number.canonical.parse::<f64>().unwrap(),
                other => cx
                    .eval_expr(other.clone())
                    .unwrap()
                    .object()
                    .display(&mut cx)
                    .unwrap()
                    .parse::<f64>()
                    .unwrap(),
            },
            _ => panic!("expected [x y] point"),
        },
        _ => panic!("expected ode-solve to return a list"),
    };
    assert!((last_y - std::f64::consts::E).abs() < 1.0e-6);
}
