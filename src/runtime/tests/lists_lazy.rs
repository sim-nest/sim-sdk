use sim_kernel::{Expr, NumberLiteral, QuoteMode, Symbol};

use super::support::{call_expr, eval_cx};
use crate::runtime::config_list_impl_capability;

fn number(text: &str) -> Expr {
    Expr::Number(NumberLiteral {
        domain: Symbol::qualified("numbers", "f64"),
        canonical: text.to_owned(),
    })
}

#[cfg(feature = "list-lazy")]
#[test]
fn list_impl_can_switch_to_lazy_backend() {
    let mut cx = eval_cx();
    cx.grant(config_list_impl_capability());

    let switched = cx
        .eval_expr(call_expr(
            Symbol::new("list-impl"),
            vec![Expr::Quote {
                mode: QuoteMode::Quote,
                expr: Box::new(Expr::Symbol(Symbol::new("lazy"))),
            }],
        ))
        .unwrap();
    assert_eq!(
        switched.object().as_expr(&mut cx).unwrap(),
        Expr::Symbol(Symbol::new("lazy"))
    );

    let len = cx
        .eval_expr(call_expr(
            Symbol::new("len"),
            vec![call_expr(
                Symbol::new("list"),
                vec![number("1"), number("2"), number("3")],
            )],
        ))
        .unwrap();
    assert_eq!(
        len.object().as_expr(&mut cx).unwrap(),
        Expr::Symbol(Symbol::new("unknown"))
    );

    let nth = cx
        .eval_expr(call_expr(
            Symbol::new("nth"),
            vec![
                call_expr(
                    Symbol::new("list"),
                    vec![number("7"), number("8"), number("9")],
                ),
                number("2"),
            ],
        ))
        .unwrap();
    assert_eq!(nth.object().as_expr(&mut cx).unwrap(), number("9"));

    let len_gte = cx
        .eval_expr(call_expr(
            Symbol::new("len>="),
            vec![
                call_expr(
                    Symbol::new("list"),
                    vec![number("7"), number("8"), number("9")],
                ),
                number("2"),
            ],
        ))
        .unwrap();
    assert_eq!(len_gte.object().as_expr(&mut cx).unwrap(), Expr::Bool(true));

    let take = cx
        .eval_expr(call_expr(
            Symbol::new("take"),
            vec![
                call_expr(
                    Symbol::new("list"),
                    vec![number("7"), number("8"), number("9")],
                ),
                number("2"),
            ],
        ))
        .unwrap();
    assert_eq!(
        take.object().as_expr(&mut cx).unwrap(),
        Expr::List(vec![number("7"), number("8")])
    );
}

#[cfg(feature = "list-lazy")]
#[test]
fn list_impl_can_switch_to_iter_backend() {
    let mut cx = eval_cx();
    cx.grant(config_list_impl_capability());

    let switched = cx
        .eval_expr(call_expr(
            Symbol::new("list-impl"),
            vec![Expr::Quote {
                mode: QuoteMode::Quote,
                expr: Box::new(Expr::Symbol(Symbol::new("iter"))),
            }],
        ))
        .unwrap();
    assert_eq!(
        switched.object().as_expr(&mut cx).unwrap(),
        Expr::Symbol(Symbol::new("iter"))
    );

    let head = cx
        .eval_expr(call_expr(
            Symbol::new("car"),
            vec![call_expr(
                Symbol::new("cons"),
                vec![
                    number("5"),
                    call_expr(Symbol::new("list"), vec![number("6"), number("7")]),
                ],
            )],
        ))
        .unwrap();
    assert_eq!(head.object().as_expr(&mut cx).unwrap(), number("5"));

    let third = cx
        .eval_expr(call_expr(
            Symbol::new("nth"),
            vec![
                call_expr(
                    Symbol::new("cons"),
                    vec![
                        number("5"),
                        call_expr(Symbol::new("list"), vec![number("6"), number("7")]),
                    ],
                ),
                number("2"),
            ],
        ))
        .unwrap();
    assert_eq!(third.object().as_expr(&mut cx).unwrap(), number("7"));
}
