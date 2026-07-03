#[cfg(feature = "numbers-prelude")]
use sim_kernel::{Args, Expr, NumberLiteral, QuoteMode, Symbol};

#[cfg(feature = "numbers-prelude")]
use crate::numbers_prelude::NumbersPreludeLib;

#[cfg(feature = "numbers-prelude")]
use super::support::eval_cx;

#[cfg(feature = "numbers-prelude")]
#[test]
fn numbers_prelude_loads_stable_stack_idempotently() {
    let mut cx = eval_cx();
    NumbersPreludeLib::new().install_all(&mut cx).unwrap();
    NumbersPreludeLib::new().install_all(&mut cx).unwrap();

    for symbol in [
        Symbol::new("+"),
        Symbol::new("fn"),
        Symbol::new("diff"),
        Symbol::new("integrate-sym"),
        Symbol::new("eval-cas"),
        Symbol::new("numeric-diff"),
        Symbol::new("integrate"),
        Symbol::new("ode-solve"),
        Symbol::new("vec"),
        Symbol::new("matmul"),
    ] {
        assert!(cx.resolve_function(&symbol).is_ok(), "missing {symbol}");
    }
}

#[cfg(feature = "numbers-prelude")]
#[test]
fn numeric_worked_example_from_numeric_4_passes() {
    let mut cx = eval_cx();
    NumbersPreludeLib::new().install_all(&mut cx).unwrap();

    let xs = cx
        .eval_expr(Expr::Call {
            operator: Box::new(Expr::Symbol(Symbol::new("vec"))),
            args: vec![num("0"), num("1"), num("2"), num("3")],
        })
        .unwrap();
    cx.env_mut().define(Symbol::new("x"), xs);

    let polynomial = cx
        .eval_expr(Expr::Call {
            operator: Box::new(Expr::Symbol(Symbol::new("+"))),
            args: vec![
                Expr::Call {
                    operator: Box::new(Expr::Symbol(Symbol::new("*"))),
                    args: vec![
                        rat("1/2"),
                        Expr::Call {
                            operator: Box::new(Expr::Symbol(Symbol::new("^"))),
                            args: vec![quoted("x"), num("2")],
                        },
                    ],
                },
                quoted("a"),
            ],
        })
        .unwrap();
    let derivative = cx
        .call_function(
            &Symbol::new("diff"),
            Args::new(vec![
                polynomial,
                cx.factory().symbol(Symbol::new("x")).unwrap(),
            ]),
        )
        .unwrap();
    assert_eq!(
        cx.call_function(&Symbol::new("eval-cas"), Args::new(vec![derivative]))
            .unwrap()
            .object()
            .as_expr(&mut cx)
            .unwrap(),
        Expr::Vector(vec![num("0"), num("1"), num("2"), num("3")])
    );

    let func = cx
        .eval_expr(Expr::Call {
            operator: Box::new(Expr::Symbol(Symbol::new("fn"))),
            args: vec![
                Expr::List(vec![Expr::Symbol(Symbol::new("x"))]),
                Expr::Call {
                    operator: Box::new(Expr::Symbol(Symbol::new("+"))),
                    args: vec![
                        Expr::Call {
                            operator: Box::new(Expr::Symbol(Symbol::new("*"))),
                            args: vec![
                                rat("1/2"),
                                Expr::Call {
                                    operator: Box::new(Expr::Symbol(Symbol::new("^"))),
                                    args: vec![quoted("x"), num("2")],
                                },
                            ],
                        },
                        num("1"),
                    ],
                },
            ],
        })
        .unwrap();

    let func_expr = func.object().as_expr(&mut cx).unwrap();
    let approx = cx
        .eval_expr(Expr::Call {
            operator: Box::new(Expr::Symbol(Symbol::new("integrate"))),
            args: vec![
                func_expr,
                quoted("x"),
                num("0"),
                num("1"),
                Expr::Symbol(Symbol::new(":method")),
                quoted("simpson"),
                Expr::Symbol(Symbol::new(":n")),
                num("100"),
            ],
        })
        .unwrap();
    let exact = cx
        .call_function(
            &Symbol::new("integrate-sym"),
            Args::new(vec![func, cx.factory().symbol(Symbol::new("x")).unwrap()]),
        )
        .unwrap();
    let one = num_value(&mut cx, "1");
    let exact_at_one = cx.call_value(exact, Args::new(vec![one])).unwrap();

    assert_eq!(exact_at_one.object().display(&mut cx).unwrap(), "7/6");
    let approx_value = parse_numeric_display(&approx.object().display(&mut cx).unwrap());
    assert!((approx_value - (7.0 / 6.0)).abs() < 1.0e-6);
}

#[cfg(feature = "numbers-prelude")]
fn quoted(name: &str) -> Expr {
    Expr::Quote {
        mode: QuoteMode::Quote,
        expr: Box::new(Expr::Symbol(Symbol::new(name))),
    }
}

#[cfg(feature = "numbers-prelude")]
fn num(text: &str) -> Expr {
    Expr::Number(NumberLiteral {
        domain: Symbol::qualified("numbers", "i64"),
        canonical: text.to_owned(),
    })
}

#[cfg(feature = "numbers-prelude")]
fn rat(text: &str) -> Expr {
    Expr::Number(NumberLiteral {
        domain: Symbol::qualified("numbers", "rational"),
        canonical: text.to_owned(),
    })
}

#[cfg(feature = "numbers-prelude")]
fn num_value(cx: &mut sim_kernel::Cx, text: &str) -> sim_kernel::Value {
    cx.factory()
        .number_literal(Symbol::qualified("numbers", "i64"), text.to_owned())
        .unwrap()
}

#[cfg(feature = "numbers-prelude")]
fn parse_numeric_display(text: &str) -> f64 {
    if let Some((num, den)) = text.split_once('/') {
        let num = num.parse::<f64>().unwrap();
        let den = den.parse::<f64>().unwrap();
        return num / den;
    }
    text.parse::<f64>().unwrap()
}
