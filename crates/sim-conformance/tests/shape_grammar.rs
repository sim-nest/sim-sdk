use std::sync::Arc;

use sim::{
    codec::{
        DecodePosition, DecodedForm, Input, Output, decode_default_with_codec, encode_with_codec,
    },
    kernel::{Cx, Diagnostic, EncodeOptions, Expr, ReadPolicy, Result, ShapeMatch, Symbol},
    shape::{ExprKind, ExprKindShape, ListShape, Shape},
};

#[path = "conformance_support/mod.rs"]
mod conformance_support;
#[allow(dead_code)]
#[path = "spec/support.rs"]
mod support;

#[test]
fn matching_text_passes_public_decode_shape_path() {
    for codec in codec_cases() {
        let mut cx = support::cx();
        let shape = flat_shape();
        let expr = Expr::List(vec![text("alpha"), Expr::Bool(true)]);
        let report = check_encoded(&mut cx, shape.as_ref(), &codec, &expr);

        assert!(report.accepted, "{:?}", report.diagnostics);
        assert!(report.decoded.is_some());
    }
}

#[test]
fn malformed_text_fails_before_shape_checking() {
    for (codec, malformed) in [(q("codec", "json"), "{"), (q("codec", "lisp"), "(")] {
        let mut cx = support::cx();
        let shape = flat_shape();
        let report = decode_shape_check(&mut cx, shape.as_ref(), &codec, malformed);

        assert!(!report.accepted);
        assert!(report.decoded.is_none());
        assert!(!report.diagnostics.is_empty());
    }
}

#[test]
fn well_formed_wrong_shape_fails_shape_checking() {
    for codec in codec_cases() {
        let mut cx = support::cx();
        let shape = flat_shape();
        let expr = Expr::List(vec![text("alpha"), text("not-bool")]);
        let report = check_encoded(&mut cx, shape.as_ref(), &codec, &expr);

        assert!(!report.accepted);
        assert!(report.decoded.is_some());
        assert!(!report.diagnostics.is_empty());
    }
}

#[derive(Debug)]
struct CheckReport {
    accepted: bool,
    decoded: Option<DecodedForm>,
    diagnostics: Vec<Diagnostic>,
}

fn check_encoded(cx: &mut Cx, shape: &dyn Shape, codec: &Symbol, expr: &Expr) -> CheckReport {
    let text = encode_text(cx, codec, expr);
    decode_shape_check(cx, shape, codec, &text)
}

fn decode_shape_check(cx: &mut Cx, shape: &dyn Shape, codec: &Symbol, text: &str) -> CheckReport {
    let decoded = match decode_default_with_codec(
        cx,
        codec,
        Input::Text(text.to_owned()),
        ReadPolicy::default(),
        DecodePosition::Data,
    ) {
        Ok(decoded) => decoded,
        Err(err) => {
            return CheckReport {
                accepted: false,
                decoded: None,
                diagnostics: vec![Diagnostic::error(format!(
                    "decode with {codec} failed: {err}"
                ))],
            };
        }
    };
    let matched = check_decoded(cx, shape, &decoded).unwrap();
    CheckReport {
        accepted: matched.accepted,
        decoded: Some(decoded),
        diagnostics: matched.diagnostics,
    }
}

fn check_decoded(cx: &mut Cx, shape: &dyn Shape, decoded: &DecodedForm) -> Result<ShapeMatch> {
    let expr = match decoded {
        DecodedForm::Datum(datum) => Expr::from(datum.clone()),
        DecodedForm::Term(term) => Expr::from(term.clone()),
    };
    shape.check_expr(cx, &expr)
}

fn encode_text(cx: &mut Cx, codec: &Symbol, expr: &Expr) -> String {
    match encode_with_codec(cx, codec, expr, EncodeOptions::default()).unwrap() {
        Output::Text(text) => text,
        Output::Bytes(bytes) => String::from_utf8(bytes).unwrap(),
    }
}

fn flat_shape() -> Arc<dyn Shape> {
    Arc::new(ListShape::new(vec![string_shape(), bool_shape()]))
}

fn string_shape() -> Arc<dyn Shape> {
    Arc::new(ExprKindShape::new(ExprKind::String))
}

fn bool_shape() -> Arc<dyn Shape> {
    Arc::new(ExprKindShape::new(ExprKind::Bool))
}

fn codec_cases() -> [Symbol; 2] {
    [q("codec", "json"), q("codec", "lisp")]
}

fn text(value: &str) -> Expr {
    Expr::String(value.to_owned())
}

fn q(namespace: &str, name: &str) -> Symbol {
    Symbol::qualified(namespace, name)
}
