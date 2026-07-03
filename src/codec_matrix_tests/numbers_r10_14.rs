#[cfg(feature = "numbers-prelude")]
use sim_codec::{Input, Output, decode_with_codec, encode_value_with_codec};

#[cfg(feature = "numbers-prelude")]
use sim_kernel::{EncodeOptions, Expr, NumberLiteral, ReadPolicy, Symbol, Value};

#[cfg(feature = "numbers-prelude")]
use crate::numbers_prelude::NumbersPreludeLib;

#[cfg(feature = "numbers-prelude")]
use super::support::{codec_symbols, cx as test_cx};

#[cfg(feature = "numbers-prelude")]
#[test]
fn numeric_value_corpus_roundtrips_through_every_codec() {
    let mut cx = test_cx();
    NumbersPreludeLib::new().install_all(&mut cx).unwrap();

    for value in numeric_values(&mut cx) {
        let expected = value.object().as_expr(&mut cx).unwrap();
        for codec in codec_symbols() {
            let encoded =
                encode_value_with_codec(&mut cx, &codec, &value, EncodeOptions::default()).unwrap();
            let input = match encoded {
                Output::Text(text) => Input::Text(text),
                Output::Bytes(bytes) => Input::Bytes(bytes),
            };
            let decoded = decode_with_codec(&mut cx, &codec, input, ReadPolicy::default())
                .unwrap_or_else(|err| {
                    panic!(
                        "codec {codec} failed to decode numeric surface {:?}: {err:?}",
                        expected
                    )
                });
            assert!(
                decoded.canonical_eq(&expected),
                "codec {codec} changed numeric surface {expected:?} into {decoded:?}"
            );
        }
    }
}

#[cfg(feature = "numbers-prelude")]
fn numeric_values(cx: &mut sim_kernel::Cx) -> Vec<Value> {
    vec![
        cx.factory().bool(true).unwrap(),
        literal(
            cx,
            "numbers",
            "i128",
            "170141183460469231731687303715884105727",
        ),
        literal(cx, "numbers", "f32", "1.25"),
        literal(cx, "numbers", "bigint", "123456789012345678901234567890"),
        literal(cx, "numbers", "rational", "1/3"),
        literal(cx, "numbers", "complex", "1+2i"),
        cx.factory().symbol(Symbol::new("cf-sqrt2")).unwrap(),
        cx.eval_expr(Expr::Call {
            operator: Box::new(Expr::Symbol(Symbol::new("vec"))),
            args: vec![i64_expr("0"), i64_expr("1"), i64_expr("2"), i64_expr("3")],
        })
        .unwrap(),
        cx.resolve_value(&Symbol::qualified("numbers/quad", "simpson"))
            .unwrap(),
        cx.resolve_value(&Symbol::qualified("numbers/rk", "rkf45"))
            .unwrap(),
    ]
}

#[cfg(feature = "numbers-prelude")]
fn literal(cx: &mut sim_kernel::Cx, namespace: &str, name: &str, canonical: &str) -> Value {
    cx.factory()
        .number_literal(Symbol::qualified(namespace, name), canonical.to_owned())
        .unwrap()
}

#[cfg(feature = "numbers-prelude")]
fn i64_expr(text: &str) -> Expr {
    Expr::Number(NumberLiteral {
        domain: Symbol::qualified("numbers", "i64"),
        canonical: text.to_owned(),
    })
}
