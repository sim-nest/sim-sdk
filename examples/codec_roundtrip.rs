//! Showcase demo: one expression, every codec, the same `Expr`.
//!
//! Decodes a Lisp source string into the shared expression graph, then encodes
//! and decodes it through the JSON and binary codecs and checks that all three
//! decode back to the identical `Expr`. This is the "codecs are first-class,
//! reversible objects over one expression graph" claim, made runnable.

use std::sync::Arc;

use sim::codec::{Input, Output, decode_with_codec, encode_with_codec};
use sim::kernel::{Cx, DefaultFactory, EagerPolicy, EncodeOptions, Expr, ReadPolicy, Symbol};

fn boot() -> Cx {
    let mut cx = Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    sim::runtime::install_core_runtime(&mut cx);
    sim::numbers_prelude::NumbersPreludeLib::new()
        .install_all(&mut cx)
        .unwrap();
    let lisp = sim::codec_lisp::LispCodecLib::new(cx.registry_mut().fresh_codec_id()).unwrap();
    cx.load_lib(&lisp).unwrap();
    let json = sim::codec_json::JsonCodecLib::new(cx.registry_mut().fresh_codec_id());
    cx.load_lib(&json).unwrap();
    let binary = sim::codec_binary::BinaryCodecLib::new(cx.registry_mut().fresh_codec_id());
    cx.load_lib(&binary).unwrap();
    cx
}

fn codec(namespace: &str, name: &str) -> Symbol {
    Symbol::qualified(namespace, name)
}

fn decode(cx: &mut Cx, ns: &str, name: &str, input: Input) -> Expr {
    decode_with_codec(cx, &codec(ns, name), input, ReadPolicy::default()).expect("decode")
}

fn encode(cx: &mut Cx, ns: &str, name: &str, expr: &Expr) -> Output {
    encode_with_codec(cx, &codec(ns, name), expr, EncodeOptions::default()).expect("encode")
}

fn main() {
    let mut cx = boot();
    let source = "(+ 1 2)";

    let from_lisp = decode(&mut cx, "codec", "lisp", Input::Text(source.to_owned()));

    // Through JSON (text) and back.
    let json_text = match encode(&mut cx, "codec", "json", &from_lisp) {
        Output::Text(text) => text,
        Output::Bytes(_) => unreachable!("json encodes to text"),
    };
    let from_json = decode(&mut cx, "codec", "json", Input::Text(json_text.clone()));

    // Through the binary frame format (bytes) and back.
    let from_binary = match encode(&mut cx, "codec", "binary", &from_lisp) {
        Output::Bytes(bytes) => decode(&mut cx, "codec", "binary", Input::Bytes(bytes)),
        Output::Text(_) => unreachable!("binary encodes to bytes"),
    };

    let identical = from_lisp == from_json && from_lisp == from_binary;

    println!("lisp source : {source}");
    println!("as json     : {json_text}");
    println!("decoded back identical across lisp / json / binary: {identical}");
}
