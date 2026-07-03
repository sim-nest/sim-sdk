use std::sync::Arc;

use sim_codec::encode_value_with_codec;
use sim_kernel::{Cx, EncodeOptions, Error, Expr, LengthResult, Result, Symbol, Value, VecList};

use super::support::{codec_symbols, cx as test_cx, decode_once};

#[cfg(feature = "list-cell")]
use crate::list_cell::install_cons_list_lib;
#[cfg(feature = "list-lazy")]
use crate::list_lazy::{LazyConsList, LazyIterList, install_lazy_list_lib};

fn number_expr(text: &str) -> Expr {
    Expr::Number(sim_kernel::NumberLiteral {
        domain: Symbol::qualified("numbers", "f64"),
        canonical: text.to_owned(),
    })
}

fn number_value(cx: &mut Cx, text: &str) -> Value {
    cx.factory()
        .number_literal(Symbol::qualified("numbers", "f64"), text.to_owned())
        .unwrap()
}

fn list_expr() -> Expr {
    Expr::List(vec![
        number_expr("1.25"),
        number_expr("2.5"),
        number_expr("3.75"),
    ])
}

fn list_values(cx: &mut Cx) -> Vec<Value> {
    vec![
        number_value(cx, "1.25"),
        number_value(cx, "2.5"),
        number_value(cx, "3.75"),
    ]
}

fn install_list_backends(cx: &mut Cx) {
    install_cons_list_lib(cx).unwrap();
    install_lazy_list_lib(cx).unwrap();
}

fn assert_backend_encodes_like_dense(cx: &mut Cx, backend: &str) {
    cx.list_registry_mut().set_active(backend).unwrap();
    let items = list_values(cx);
    let value = cx.new_list(items).unwrap();
    let dense = list_expr();
    for codec in codec_symbols() {
        let encoded =
            encode_value_with_codec(cx, &codec, &value, EncodeOptions::default()).unwrap();
        let baseline =
            sim_codec::encode_with_codec(cx, &codec, &dense, EncodeOptions::default()).unwrap();
        assert_eq!(
            encoded, baseline,
            "backend {backend} changed wire form for {codec}"
        );
        let decoded = decode_once(cx, &codec, encoded);
        assert!(
            decoded.canonical_eq(&dense),
            "backend {backend} via {codec} decoded {decoded:?} instead of {dense:?}"
        );
        let Expr::List(_) = decoded else {
            panic!("codec {codec} did not decode the encoded list back to Expr::List");
        };
    }
}

#[test]
fn vec_and_cons_lists_roundtrip_identically() {
    let mut cx = test_cx();
    install_list_backends(&mut cx);
    assert_backend_encodes_like_dense(&mut cx, "vec");
    assert_backend_encodes_like_dense(&mut cx, "cons");
}

#[test]
fn finite_lazy_backends_roundtrip_identically() {
    let mut cx = test_cx();
    install_list_backends(&mut cx);
    assert_backend_encodes_like_dense(&mut cx, "lazy");
    assert_backend_encodes_like_dense(&mut cx, "iter");
}

#[test]
fn decode_target_remains_dense_vec_list() {
    let mut cx = test_cx();
    install_list_backends(&mut cx);
    cx.list_registry_mut().set_active("iter").unwrap();
    let items = list_values(&mut cx);
    let value = cx.new_list(items).unwrap();

    for codec in codec_symbols() {
        let encoded =
            encode_value_with_codec(&mut cx, &codec, &value, EncodeOptions::default()).unwrap();
        let decoded = decode_once(&mut cx, &codec, encoded);
        let rebuilt = expr_to_dense_value(&mut cx, &decoded).unwrap();
        let list = rebuilt.object().as_list().unwrap();
        assert_eq!(list.len(&mut cx).unwrap(), LengthResult::Known(3));
        assert!(rebuilt.object().downcast_ref::<VecList>().is_some());
    }
}

#[test]
fn endless_lazy_cons_encode_returns_bounded_error() {
    let mut cx = test_cx();
    install_list_backends(&mut cx);
    let looped = Arc::<LazyConsList>::new_cyclic(|weak| {
        let weak = weak.clone();
        LazyConsList::new(
            move |cx| Ok(number_value(cx, "1.25")),
            move |cx| Ok(Some(cx.factory().opaque(weak.upgrade().unwrap())?)),
        )
    });
    let value = cx.factory().opaque(looped).unwrap();
    let err = encode_value_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "lisp"),
        &value,
        EncodeOptions::default(),
    )
    .unwrap_err();
    assert_force_bound_error(err);
}

#[test]
fn endless_lazy_iter_encode_returns_bounded_error() {
    let mut cx = test_cx();
    install_list_backends(&mut cx);
    let item = number_value(&mut cx, "1.25");
    let value = cx
        .factory()
        .opaque(Arc::new(LazyIterList::new(Box::new(
            std::iter::repeat_with(move || Ok(item.clone())),
        ))))
        .unwrap();
    let err = encode_value_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "json"),
        &value,
        EncodeOptions::default(),
    )
    .unwrap_err();
    assert_force_bound_error(err);
}

fn assert_force_bound_error(err: Error) {
    match err {
        Error::Eval(message) => {
            assert!(message.contains("force bound"));
            assert!(message.contains("lazy/endless list"));
        }
        other => panic!("expected bounded encode error, found {other:?}"),
    }
}

fn expr_to_dense_value(cx: &mut Cx, expr: &Expr) -> Result<Value> {
    match expr {
        Expr::Nil => cx.factory().nil(),
        Expr::Bool(value) => cx.factory().bool(*value),
        Expr::Number(number) => cx
            .factory()
            .number_literal(number.domain.clone(), number.canonical.clone()),
        Expr::Symbol(symbol) => cx.factory().symbol(symbol.clone()),
        Expr::String(text) => cx.factory().string(text.clone()),
        Expr::Bytes(bytes) => cx.factory().bytes(bytes.clone()),
        Expr::List(items) | Expr::Vector(items) => {
            let values = items
                .iter()
                .map(|item| expr_to_dense_value(cx, item))
                .collect::<Result<Vec<_>>>()?;
            cx.factory().list(values)
        }
        Expr::Map(entries) => {
            let values = entries
                .iter()
                .map(|(key, value)| {
                    let Expr::Symbol(symbol) = key else {
                        return Err(Error::TypeMismatch {
                            expected: "symbol table key",
                            found: "non-symbol",
                        });
                    };
                    Ok((symbol.clone(), expr_to_dense_value(cx, value)?))
                })
                .collect::<Result<Vec<_>>>()?;
            cx.factory().table(values)
        }
        _ => cx.factory().expr(expr.clone()),
    }
}
