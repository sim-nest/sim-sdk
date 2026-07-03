use std::sync::Arc;
use std::{panic::AssertUnwindSafe, panic::catch_unwind};

use sim_codec::{CodecRuntime, Input, Output, decode_with_codec, encode_with_codec};
use sim_kernel::{
    DefaultFactory, EagerPolicy, EncodeOptions, Expr, NumberLiteral, QuoteMode, ReadPolicy, Symbol,
};

use crate::{
    codec_algol::AlgolCodecLib, codec_binary::BinaryCodecLib,
    codec_binary_base64::BinaryBase64CodecLib, codec_json::JsonCodecLib, codec_lisp::LispCodecLib,
    runtime::install_core_runtime,
};

pub fn cx() -> sim_kernel::Cx {
    let mut cx = sim_kernel::Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    #[cfg(feature = "numbers-f64")]
    if cx
        .registry()
        .number_domain_by_symbol(&Symbol::qualified("numbers", "f64"))
        .is_none()
    {
        cx.load_lib(&crate::numbers_f64::F64NumbersLib::new())
            .unwrap();
    }

    let lisp = LispCodecLib::new(cx.registry_mut().fresh_codec_id()).unwrap();
    cx.load_lib(&lisp).unwrap();
    let json = JsonCodecLib::new(cx.registry_mut().fresh_codec_id());
    cx.load_lib(&json).unwrap();
    let binary = BinaryCodecLib::new(cx.registry_mut().fresh_codec_id());
    cx.load_lib(&binary).unwrap();
    let binary_base64 = BinaryBase64CodecLib::new(cx.registry_mut().fresh_codec_id());
    cx.load_lib(&binary_base64).unwrap();
    let algol = AlgolCodecLib::new(cx.registry_mut().fresh_codec_id());
    cx.load_lib(&algol).unwrap();
    #[cfg(feature = "codec-bitwise")]
    {
        let bitwise =
            crate::codec_bitwise::BitwiseCodecLib::new(cx.registry_mut().fresh_codec_id());
        cx.load_lib(&bitwise).unwrap();
    }
    #[cfg(feature = "codec-bitwise-base64")]
    {
        let bitwise_base64 =
            crate::codec_bitwise_base64::BitwiseBase64CodecLib::new(cx.registry_mut().fresh_codec_id());
        cx.load_lib(&bitwise_base64).unwrap();
    }

    cx
}

pub fn codec_symbols() -> Vec<Symbol> {
    #[allow(unused_mut)]
    let mut symbols = vec![
        Symbol::qualified("codec", "lisp"),
        Symbol::qualified("codec", "json"),
        Symbol::qualified("codec", "binary"),
        Symbol::qualified("codec", "binary-base64"),
        Symbol::qualified("codec", "algol"),
    ];
    #[cfg(feature = "codec-bitwise")]
    symbols.push(Symbol::qualified("codec", "bitwise"));
    #[cfg(feature = "codec-bitwise-base64")]
    symbols.push(Symbol::qualified("codec", "bitwise-base64"));
    symbols
}

pub fn corpus() -> Vec<Expr> {
    let mut corpus = vec![
        Expr::Nil,
        Expr::Bool(true),
        Expr::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "f64"),
            canonical: "42.5".to_owned(),
        }),
        Expr::Symbol(Symbol::qualified("math", "pi")),
        Expr::Local(Symbol::new("arg0")),
        Expr::String("line\n\"quoted\"".to_owned()),
        Expr::Bytes(vec![0, 1, 2, 0xff]),
        Expr::List(vec![
            Expr::Symbol(Symbol::new("f")),
            Expr::String("x".to_owned()),
            Expr::Bool(false),
        ]),
        Expr::Vector(vec![Expr::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "f64"),
            canonical: "1.5".to_owned(),
        })]),
        Expr::Map(vec![
            (Expr::Symbol(Symbol::new("b")), Expr::Bool(false)),
            (Expr::Symbol(Symbol::new("a")), Expr::Bool(true)),
        ]),
        Expr::Set(vec![
            Expr::String("z".to_owned()),
            Expr::String("a".to_owned()),
        ]),
        Expr::Call {
            operator: Box::new(Expr::Symbol(Symbol::qualified("math", "add"))),
            args: vec![
                Expr::Number(NumberLiteral {
                    domain: Symbol::qualified("numbers", "f64"),
                    canonical: "1.25".to_owned(),
                }),
                Expr::Number(NumberLiteral {
                    domain: Symbol::qualified("numbers", "f64"),
                    canonical: "2.5".to_owned(),
                }),
            ],
        },
        Expr::Infix {
            operator: Symbol::new("+"),
            left: Box::new(Expr::Number(NumberLiteral {
                domain: Symbol::qualified("numbers", "f64"),
                canonical: "1.25".to_owned(),
            })),
            right: Box::new(Expr::Prefix {
                operator: Symbol::new("-"),
                arg: Box::new(Expr::Postfix {
                    operator: Symbol::new("!"),
                    arg: Box::new(Expr::Symbol(Symbol::new("n"))),
                }),
            }),
        },
        Expr::Block(vec![
            Expr::Symbol(Symbol::new("x")),
            Expr::String("done".to_owned()),
        ]),
    ];
    corpus.extend(quote_modes().into_iter().map(|mode| Expr::Quote {
        mode,
        expr: Box::new(Expr::Symbol(Symbol::new(format!(
            "quote-target-{}",
            quote_mode_name(mode)
        )))),
    }));
    corpus.push(
        Expr::Quote {
            mode: sim_kernel::QuoteMode::Syntax,
            expr: Box::new(Expr::Extension {
                tag: Symbol::qualified("demo", "escape"),
                payload: Box::new(Expr::Annotated {
                    expr: Box::new(Expr::Map(vec![(
                        Expr::String("k".to_owned()),
                        Expr::Vector(vec![Expr::Bool(true)]),
                    )])),
                    annotations: vec![(
                        Symbol::qualified("meta", "origin"),
                        Expr::String("matrix".to_owned()),
                    )],
                }),
            }),
        },
    );
    corpus
}

pub fn generated_expr_corpus() -> Vec<Expr> {
    let mut exprs = (0..76).map(|seed| generated_expr(seed, 3)).collect::<Vec<_>>();
    exprs.extend(quote_modes().into_iter().map(|mode| Expr::Quote {
        mode,
        expr: Box::new(generated_expr(100 + mode as u64, 2)),
    }));
    exprs
}

pub fn quote_modes() -> [QuoteMode; 5] {
    [
        QuoteMode::Quote,
        QuoteMode::QuasiQuote,
        QuoteMode::Unquote,
        QuoteMode::Splice,
        QuoteMode::Syntax,
    ]
}

pub fn variant_name(expr: &Expr) -> &'static str {
    match expr {
        Expr::Nil => "Nil",
        Expr::Bool(_) => "Bool",
        Expr::Number(_) => "Number",
        Expr::Symbol(_) => "Symbol",
        Expr::Local(_) => "Local",
        Expr::String(_) => "String",
        Expr::Bytes(_) => "Bytes",
        Expr::List(_) => "List",
        Expr::Vector(_) => "Vector",
        Expr::Map(_) => "Map",
        Expr::Set(_) => "Set",
        Expr::Call { .. } => "Call",
        Expr::Infix { .. } => "Infix",
        Expr::Prefix { .. } => "Prefix",
        Expr::Postfix { .. } => "Postfix",
        Expr::Block(_) => "Block",
        Expr::Quote { .. } => "Quote",
        Expr::Annotated { .. } => "Annotated",
        Expr::Extension { .. } => "Extension",
    }
}

pub fn quote_mode_name(mode: QuoteMode) -> &'static str {
    match mode {
        QuoteMode::Quote => "Quote",
        QuoteMode::QuasiQuote => "QuasiQuote",
        QuoteMode::Unquote => "Unquote",
        QuoteMode::Splice => "Splice",
        QuoteMode::Syntax => "Syntax",
    }
}

fn generated_expr(seed: u64, depth: u8) -> Expr {
    if depth == 0 {
        return generated_leaf(seed);
    }

    match seed % 19 {
        0 => Expr::Nil,
        1 => Expr::Bool(seed.is_multiple_of(2)),
        2 => Expr::Number(generated_number(seed)),
        3 => Expr::Symbol(generated_symbol(seed)),
        4 => Expr::Local(Symbol::new(format!("local_{}", seed % 7))),
        5 => Expr::String(format!("text:{seed}\n\"quoted\"")),
        6 => Expr::Bytes(vec![seed as u8, seed.wrapping_mul(3) as u8, 0xff]),
        7 => Expr::List(vec![
            generated_expr(seed + 1, depth - 1),
            generated_expr(seed + 2, depth - 1),
        ]),
        8 => Expr::Vector(vec![
            generated_expr(seed + 3, depth - 1),
            generated_expr(seed + 4, depth - 1),
        ]),
        9 => Expr::Map(vec![
            (
                Expr::Symbol(generated_symbol(seed + 5)),
                generated_expr(seed + 6, depth - 1),
            ),
            (
                Expr::String(format!("key-{seed}")),
                generated_expr(seed + 7, depth - 1),
            ),
        ]),
        10 => Expr::Set(vec![
            generated_expr(seed + 8, depth - 1),
            generated_expr(seed + 9, depth - 1),
        ]),
        11 => Expr::Call {
            operator: Box::new(Expr::Symbol(Symbol::qualified("test", "apply"))),
            args: vec![
                generated_expr(seed + 10, depth - 1),
                generated_expr(seed + 11, depth - 1),
            ],
        },
        12 => Expr::Infix {
            operator: Symbol::new("+"),
            left: Box::new(generated_expr(seed + 12, depth - 1)),
            right: Box::new(generated_expr(seed + 13, depth - 1)),
        },
        13 => Expr::Prefix {
            operator: Symbol::new("-"),
            arg: Box::new(generated_expr(seed + 14, depth - 1)),
        },
        14 => Expr::Postfix {
            operator: Symbol::new("!"),
            arg: Box::new(generated_expr(seed + 15, depth - 1)),
        },
        15 => Expr::Block(vec![
            generated_expr(seed + 16, depth - 1),
            generated_expr(seed + 17, depth - 1),
        ]),
        16 => Expr::Quote {
            mode: quote_modes()[(seed as usize) % quote_modes().len()],
            expr: Box::new(generated_expr(seed + 18, depth - 1)),
        },
        17 => Expr::Annotated {
            expr: Box::new(generated_expr(seed + 19, depth - 1)),
            annotations: vec![(
                Symbol::qualified("meta", format!("a{}", seed % 5)),
                generated_expr(seed + 20, depth - 1),
            )],
        },
        _ => Expr::Extension {
            tag: Symbol::qualified("generated", format!("tag{}", seed % 5)),
            payload: Box::new(generated_expr(seed + 21, depth - 1)),
        },
    }
}

fn generated_leaf(seed: u64) -> Expr {
    match seed % 7 {
        0 => Expr::Nil,
        1 => Expr::Bool(seed.is_multiple_of(2)),
        2 => Expr::Number(generated_number(seed)),
        3 => Expr::Symbol(generated_symbol(seed)),
        4 => Expr::Local(Symbol::new(format!("local_{}", seed % 7))),
        5 => Expr::String(format!("leaf-{seed}")),
        _ => Expr::Bytes(vec![seed as u8, 0, 255]),
    }
}

fn generated_number(seed: u64) -> NumberLiteral {
    match seed % 4 {
        0 => NumberLiteral {
            domain: Symbol::qualified("numbers", "f64"),
            canonical: format!("{}.5", seed % 17),
        },
        1 => NumberLiteral {
            domain: Symbol::qualified("numbers", "f64"),
            canonical: "0".to_owned(),
        },
        2 => NumberLiteral {
            domain: Symbol::qualified("numbers", "i64"),
            canonical: format!("{}", (seed % 23) as i64 - 11),
        },
        _ => NumberLiteral {
            domain: Symbol::qualified("numbers", "rational"),
            canonical: format!("{}/{}", seed % 11 + 1, seed % 5 + 2),
        },
    }
}

fn generated_symbol(seed: u64) -> Symbol {
    match seed % 6 {
        0 => Symbol::new("alpha"),
        1 => Symbol::qualified("demo", format!("beta{}", seed % 5)),
        2 => Symbol::new("nil"),
        3 => Symbol::new("+"),
        4 => Symbol::new("foo-bar"),
        _ => Symbol::qualified("expr", "lisp"),
    }
}

pub fn encode_once(cx: &mut sim_kernel::Cx, codec: &Symbol, expr: &Expr) -> Output {
    encode_with_codec(cx, codec, expr, EncodeOptions::default())
        .unwrap_or_else(|err| panic!("codec {codec} failed to encode {expr:?}: {err:?}"))
}

pub fn decode_once(cx: &mut sim_kernel::Cx, codec: &Symbol, output: Output) -> Expr {
    let input = match output {
        Output::Text(text) => Input::Text(text),
        Output::Bytes(bytes) => Input::Bytes(bytes),
    };
    decode_with_codec(cx, codec, input.clone(), ReadPolicy::default())
        .unwrap_or_else(|err| panic!("codec {codec} failed to decode {input:?}: {err:?}"))
}

#[test]
fn malformed_public_codec_inputs_return_errors_instead_of_panicking() {
    let mut cx = cx();
    let cases = vec![
        (
            Symbol::qualified("codec", "lisp"),
            Input::Text("#wat".to_owned()),
        ),
        (
            Symbol::qualified("codec", "json"),
            Input::Text("{".to_owned()),
        ),
        (
            Symbol::qualified("codec", "binary"),
            Input::Bytes(vec![0, 1, 2, 3]),
        ),
        (
            Symbol::qualified("codec", "algol"),
            Input::Text("1 + )".to_owned()),
        ),
    ];

    for (codec, input) in cases {
        let result = catch_unwind(AssertUnwindSafe(|| {
            decode_with_codec(&mut cx, &codec, input, ReadPolicy::default())
        }));
        let outcome =
            result.unwrap_or_else(|_| panic!("codec {codec} panicked on malformed input"));
        assert!(
            outcome.is_err(),
            "codec {codec} accepted malformed input unexpectedly"
        );
    }
}

#[test]
fn codec_metadata_uses_specific_shape_objects() {
    let mut cx = cx();
    let expected = [
        (
            Symbol::qualified("codec", "lisp"),
            Symbol::qualified("codec", "LispSurface"),
        ),
        (
            Symbol::qualified("codec", "json"),
            Symbol::qualified("codec", "JsonTaggedExpr"),
        ),
        (
            Symbol::qualified("codec", "binary"),
            Symbol::qualified("codec", "BinaryFrame"),
        ),
        (
            Symbol::qualified("codec", "binary-base64"),
            Symbol::qualified("codec", "BinaryBase64Text"),
        ),
        (
            Symbol::qualified("codec", "algol"),
            Symbol::qualified("codec", "AlgolSurface"),
        ),
    ];

    for (codec_symbol, expr_shape_symbol) in expected {
        let codec_value = cx.resolve_codec(&codec_symbol).unwrap();
        let codec = codec_value.object().downcast_ref::<CodecRuntime>().unwrap();
        let expr_shape = codec.expr_shape.clone();
        let options_shape = codec.options_shape.clone();

        assert_eq!(
            expr_shape.object().as_expr(&mut cx).unwrap(),
            Expr::Symbol(expr_shape_symbol.clone())
        );
        assert_eq!(
            options_shape.object().as_expr(&mut cx).unwrap(),
            Expr::Symbol(Symbol::qualified("core", "EncodeOptions"))
        );
        assert_ne!(
            expr_shape.object().as_expr(&mut cx).unwrap(),
            Expr::Symbol(Symbol::qualified("core", "Any"))
        );
    }
}
