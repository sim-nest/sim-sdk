use std::sync::Arc;

use sim::{kernel::{Cx, DefaultFactory, NoopEvalPolicy}, lib_lang_jvm};

fn main() {
    let mut cx = Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    cx.grant(lib_lang_jvm::class_load_capability());
    cx.grant(lib_lang_jvm::jvm_invoke_capability());
    let outcome = lib_lang_jvm::JvmSurface::new(1 << 20).execute_i32(
        &mut cx,
        lib_lang_jvm::JvmExecutionRequest {
            classfile: decode_hex(include_str!("../StaticInt.hex")),
            class: "StaticInt".into(), member: "wholePipeline".into(),
            descriptor: "(II)I".into(), arguments: vec![5, 6],
        },
    );
    assert!(matches!(outcome, lib_lang_jvm::JvmExecutionOutcome::Value(22)));
}

fn decode_hex(input: &str) -> Vec<u8> {
    input.trim().as_bytes().chunks_exact(2).map(|pair| {
        u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap()
    }).collect()
}
