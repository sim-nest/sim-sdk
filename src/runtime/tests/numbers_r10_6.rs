#[cfg(all(
    feature = "codec-lisp",
    feature = "numbers-cas",
    feature = "numbers-i64"
))]
use std::sync::Arc;

#[cfg(all(
    feature = "codec-lisp",
    feature = "numbers-cas",
    feature = "numbers-i64"
))]
use sim_codec::encode_value_with_codec;
#[cfg(all(
    feature = "codec-lisp",
    feature = "numbers-cas",
    feature = "numbers-i64"
))]
use sim_codec_lisp::LispCodecLib;
#[cfg(all(
    feature = "codec-lisp",
    feature = "numbers-cas",
    feature = "numbers-i64"
))]
use sim_kernel::{
    DefaultFactory, EagerPolicy, EncodeOptions, Expr, NumberLiteral, QuoteMode, Symbol,
    macro_expand_eval_capability,
};

#[cfg(all(
    feature = "codec-lisp",
    feature = "numbers-cas",
    feature = "numbers-i64"
))]
use crate::runtime::install_core_runtime;

#[cfg(all(
    feature = "codec-lisp",
    feature = "numbers-cas",
    feature = "numbers-i64"
))]
fn runtime() -> sim_kernel::Cx {
    let mut cx = sim_kernel::Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    cx.grant(macro_expand_eval_capability());
    let lisp = LispCodecLib::new(cx.registry_mut().fresh_codec_id()).unwrap();
    cx.load_lib(&lisp).unwrap();
    cx
}

#[cfg(all(
    feature = "codec-lisp",
    feature = "numbers-cas",
    feature = "numbers-i64"
))]
#[test]
fn quoted_symbol_addition_builds_canonical_cas() {
    let mut cx = runtime();
    let expr = Expr::Call {
        operator: Box::new(Expr::Symbol(Symbol::new("+"))),
        args: vec![
            Expr::Number(NumberLiteral {
                domain: Symbol::qualified("numbers", "i64"),
                canonical: "1".to_owned(),
            }),
            Expr::Quote {
                mode: QuoteMode::Quote,
                expr: Box::new(Expr::Symbol(Symbol::new("a"))),
            },
        ],
    };
    let value = cx.eval_expr(expr).unwrap();
    let encoded = encode_value_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "lisp"),
        &value,
        EncodeOptions::default(),
    )
    .unwrap()
    .into_text()
    .unwrap();
    assert_eq!(encoded, "(+ 1 a)");
}

#[cfg(all(
    feature = "codec-lisp",
    feature = "numbers-cas",
    feature = "numbers-i64"
))]
#[test]
fn nested_addition_folds_constants_inside_cas() {
    let mut cx = runtime();
    let expr = Expr::Call {
        operator: Box::new(Expr::Symbol(Symbol::new("+"))),
        args: vec![
            Expr::Call {
                operator: Box::new(Expr::Symbol(Symbol::new("+"))),
                args: vec![
                    Expr::Number(NumberLiteral {
                        domain: Symbol::qualified("numbers", "i64"),
                        canonical: "1".to_owned(),
                    }),
                    Expr::Quote {
                        mode: QuoteMode::Quote,
                        expr: Box::new(Expr::Symbol(Symbol::new("a"))),
                    },
                ],
            },
            Expr::Number(NumberLiteral {
                domain: Symbol::qualified("numbers", "i64"),
                canonical: "2".to_owned(),
            }),
        ],
    };
    let value = cx.eval_expr(expr).unwrap();
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::List(vec![
            Expr::Symbol(Symbol::new("+")),
            Expr::Number(NumberLiteral {
                domain: Symbol::qualified("numbers", "i64"),
                canonical: "3".to_owned(),
            }),
            Expr::Symbol(Symbol::new("a")),
        ])
    );
}

#[cfg(all(
    feature = "codec-lisp",
    feature = "numbers-cas",
    feature = "numbers-i64"
))]
#[test]
fn zero_product_simplifies_to_plain_number() {
    let mut cx = runtime();
    let expr = Expr::Call {
        operator: Box::new(Expr::Symbol(Symbol::new("*"))),
        args: vec![
            Expr::Number(NumberLiteral {
                domain: Symbol::qualified("numbers", "i64"),
                canonical: "0".to_owned(),
            }),
            Expr::Quote {
                mode: QuoteMode::Quote,
                expr: Box::new(Expr::Symbol(Symbol::new("x"))),
            },
        ],
    };
    let value = cx.eval_expr(expr).unwrap();
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "i64"),
            canonical: "0".to_owned(),
        })
    );
}

#[cfg(all(
    feature = "codec-lisp",
    feature = "numbers-cas",
    feature = "numbers-exotic",
    feature = "numbers-i64"
))]
#[test]
fn continued_fraction_addition_prefers_cas_when_available() {
    let mut cx = runtime();
    let expr = Expr::Call {
        operator: Box::new(Expr::Symbol(Symbol::new("+"))),
        args: vec![
            Expr::Symbol(Symbol::new("cf-sqrt2")),
            Expr::Number(NumberLiteral {
                domain: Symbol::qualified("numbers", "i64"),
                canonical: "1".to_owned(),
            }),
        ],
    };
    let value = cx.eval_expr(expr).unwrap();
    let encoded = encode_value_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "lisp"),
        &value,
        EncodeOptions::default(),
    )
    .unwrap()
    .into_text()
    .unwrap();
    assert_eq!(encoded, "(+ 1 cf-sqrt2)");
}
