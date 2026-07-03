//! A tiny REPL for SIM: read a line, evaluate it, print the result.
//!
//! Boots a runtime with the number domains and the Lisp codec, then loops:
//! decode each line in *eval position* (so a list becomes a call, not data),
//! evaluate it, and encode the result back to source. This is `sim repl` in
//! miniature -- the published binary will load these same pieces as libraries
//! through the bootloader; here they are linked in.
//!
//! Try it: type `(math/add (math/mul 6 7) 0)` and get `42`. Ctrl-D to exit.
//! It is pipe-friendly too: `echo '(math/add 1 2)' | <this example>` prints `3`.

use std::io::{self, BufRead, Write};
use std::sync::Arc;

use sim::codec::{
    DecodePosition, DecodedForm, Input, Output, decode_default_with_codec, encode_with_codec,
};
use sim::kernel::{Cx, DefaultFactory, EagerPolicy, EncodeOptions, Expr, ReadPolicy, Symbol};

fn boot() -> Cx {
    let mut cx = Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    sim::runtime::install_core_runtime(&mut cx);
    sim::numbers_prelude::NumbersPreludeLib::new()
        .install_all(&mut cx)
        .unwrap();
    let lisp = sim::codec_lisp::LispCodecLib::new(cx.registry_mut().fresh_codec_id()).unwrap();
    cx.load_lib(&lisp).unwrap();
    cx
}

fn eval_line(cx: &mut Cx, codec: &Symbol, line: &str) -> Result<String, String> {
    let decoded = match decode_default_with_codec(
        cx,
        codec,
        Input::Text(line.to_owned()),
        ReadPolicy::default(),
        DecodePosition::Eval,
    )
    .map_err(|err| format!("{err:?}"))?
    {
        DecodedForm::Term(term) => Expr::from(term),
        DecodedForm::Datum(datum) => Expr::from(datum),
    };
    let value = cx.eval_expr(decoded).map_err(|err| format!("{err:?}"))?;
    let expr = value
        .object()
        .as_expr(cx)
        .map_err(|err| format!("{err:?}"))?;
    match encode_with_codec(cx, codec, &expr, EncodeOptions::default())
        .map_err(|err| format!("{err:?}"))?
    {
        Output::Text(text) => Ok(text),
        Output::Bytes(_) => Ok("<bytes>".to_owned()),
    }
}

fn main() {
    let mut cx = boot();
    let codec = Symbol::qualified("codec", "lisp");

    // The prompt and banner are UI, not output: keep them on stderr so a piped
    // run leaves only results on stdout.
    eprintln!("sim repl (example) -- type a Lisp expression, Ctrl-D to exit");
    eprint!("sim> ");
    let _ = io::stderr().flush();

    for line in io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            match eval_line(&mut cx, &codec, trimmed) {
                Ok(result) => println!("{result}"),
                Err(err) => println!("error: {err}"),
            }
        }
        eprint!("sim> ");
        let _ = io::stderr().flush();
    }
    eprintln!();
}
