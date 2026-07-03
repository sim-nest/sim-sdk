use sim_kernel::{Error, Expr, NumberLiteral, QuoteMode, Symbol, config_list_impl_capability};

use super::support::{call_expr, eval_cx};

fn number(text: &str) -> Expr {
    Expr::Number(NumberLiteral {
        domain: Symbol::qualified("numbers", "f64"),
        canonical: text.to_owned(),
    })
}

#[test]
fn cons_and_walk_default_vec() {
    let mut cx = eval_cx();
    let value = cx
        .eval_expr(call_expr(
            Symbol::new("car"),
            vec![call_expr(
                Symbol::new("cons"),
                vec![
                    number("1"),
                    call_expr(Symbol::new("list"), vec![number("2"), number("3")]),
                ],
            )],
        ))
        .unwrap();
    assert_eq!(value.object().as_expr(&mut cx).unwrap(), number("1"));

    let len = cx
        .eval_expr(call_expr(
            Symbol::new("len"),
            vec![call_expr(
                Symbol::new("cons"),
                vec![
                    number("1"),
                    call_expr(Symbol::new("list"), vec![number("2"), number("3")]),
                ],
            )],
        ))
        .unwrap();
    assert_eq!(len.object().as_expr(&mut cx).unwrap(), number("3"));

    let nth = cx
        .eval_expr(call_expr(
            Symbol::new("nth"),
            vec![
                call_expr(
                    Symbol::new("list"),
                    vec![number("7"), number("8"), number("9")],
                ),
                number("1"),
            ],
        ))
        .unwrap();
    assert_eq!(nth.object().as_expr(&mut cx).unwrap(), number("8"));

    let len_cmp = cx
        .eval_expr(call_expr(
            Symbol::new("len-cmp"),
            vec![
                call_expr(
                    Symbol::new("list"),
                    vec![number("1"), number("2"), number("3")],
                ),
                number("3"),
            ],
        ))
        .unwrap();
    assert_eq!(
        len_cmp.object().as_expr(&mut cx).unwrap(),
        Expr::Symbol(Symbol::new("eq"))
    );

    let len_lt = cx
        .eval_expr(call_expr(
            Symbol::new("len<"),
            vec![
                call_expr(
                    Symbol::new("list"),
                    vec![number("1"), number("2"), number("3")],
                ),
                number("5"),
            ],
        ))
        .unwrap();
    assert_eq!(len_lt.object().as_expr(&mut cx).unwrap(), Expr::Bool(true));

    let len_eq = cx
        .eval_expr(call_expr(
            Symbol::new("len="),
            vec![
                call_expr(
                    Symbol::new("list"),
                    vec![number("1"), number("2"), number("3")],
                ),
                number("3"),
            ],
        ))
        .unwrap();
    assert_eq!(len_eq.object().as_expr(&mut cx).unwrap(), Expr::Bool(true));

    let take = cx
        .eval_expr(call_expr(
            Symbol::new("take"),
            vec![
                call_expr(
                    Symbol::new("list"),
                    vec![number("4"), number("5"), number("6")],
                ),
                number("2"),
            ],
        ))
        .unwrap();
    assert_eq!(
        take.object().as_expr(&mut cx).unwrap(),
        Expr::List(vec![number("4"), number("5")])
    );

    let drop = cx
        .eval_expr(call_expr(
            Symbol::new("drop"),
            vec![
                call_expr(
                    Symbol::new("list"),
                    vec![number("4"), number("5"), number("6")],
                ),
                number("2"),
            ],
        ))
        .unwrap();
    assert_eq!(
        drop.object().as_expr(&mut cx).unwrap(),
        Expr::List(vec![number("6")])
    );
}

#[test]
fn cons_terminates_with_empty_list_not_nil() {
    let mut cx = eval_cx();
    let single = cx
        .eval_expr(call_expr(
            Symbol::new("cons"),
            vec![number("1"), Expr::List(Vec::new())],
        ))
        .unwrap();
    assert_eq!(
        single.object().as_expr(&mut cx).unwrap(),
        Expr::List(vec![number("1")])
    );

    let tail_is_empty = cx
        .eval_expr(call_expr(
            Symbol::new("empty?"),
            vec![call_expr(
                Symbol::new("cdr"),
                vec![call_expr(
                    Symbol::new("cons"),
                    vec![number("1"), Expr::List(Vec::new())],
                )],
            )],
        ))
        .unwrap();
    assert_eq!(
        tail_is_empty.object().as_expr(&mut cx).unwrap(),
        Expr::Bool(true)
    );

    let nil_tail = cx.eval_expr(call_expr(Symbol::new("cons"), vec![number("1"), Expr::Nil]));
    assert!(matches!(
        nil_tail,
        Err(Error::TypeMismatch {
            expected: "list",
            found: "non-list"
        })
    ));
}

#[test]
fn list_impl_switch_is_capability_gated() {
    let mut cx = eval_cx();
    let denied = cx.eval_expr(call_expr(
        Symbol::new("list-impl"),
        vec![Expr::Quote {
            mode: QuoteMode::Quote,
            expr: Box::new(Expr::Symbol(Symbol::new("vec"))),
        }],
    ));
    assert!(matches!(
        denied,
        Err(Error::CapabilityDenied { capability })
            if capability == config_list_impl_capability()
    ));

    cx.grant(config_list_impl_capability());
    let current = cx
        .eval_expr(call_expr(Symbol::new("list-impl"), Vec::new()))
        .unwrap();
    assert_eq!(
        current.object().as_expr(&mut cx).unwrap(),
        Expr::Symbol(Symbol::new("vec"))
    );

    let switched = cx
        .eval_expr(call_expr(
            Symbol::new("list-impl"),
            vec![Expr::Quote {
                mode: QuoteMode::Quote,
                expr: Box::new(Expr::Symbol(Symbol::new("vec"))),
            }],
        ))
        .unwrap();
    assert_eq!(
        switched.object().as_expr(&mut cx).unwrap(),
        Expr::Symbol(Symbol::new("vec"))
    );
}

#[cfg(feature = "list-cell")]
#[test]
fn list_impl_can_switch_to_cons_backend() {
    let mut cx = eval_cx();
    cx.grant(config_list_impl_capability());

    let switched = cx
        .eval_expr(call_expr(
            Symbol::new("list-impl"),
            vec![Expr::Quote {
                mode: QuoteMode::Quote,
                expr: Box::new(Expr::Symbol(Symbol::new("cons"))),
            }],
        ))
        .unwrap();
    assert_eq!(
        switched.object().as_expr(&mut cx).unwrap(),
        Expr::Symbol(Symbol::new("cons"))
    );

    let len = cx
        .eval_expr(call_expr(
            Symbol::new("len"),
            vec![call_expr(
                Symbol::new("cons"),
                vec![
                    number("1"),
                    call_expr(Symbol::new("list"), vec![number("2"), number("3")]),
                ],
            )],
        ))
        .unwrap();
    assert_eq!(len.object().as_expr(&mut cx).unwrap(), number("3"));
}
