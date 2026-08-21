// conformance: the standard JVM facade exports the complete public front door.

#![cfg(feature = "standard-jvm")]

#[test]
fn sdk_exports_caller_selected_jvm_execution() {
    use sim::kernel::{Cx, DefaultFactory, NoopEvalPolicy};
    use std::sync::Arc;
    let mut cx = Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    cx.grant(sim::lib_lang_jvm::class_load_capability());
    cx.grant(sim::lib_lang_jvm::jvm_invoke_capability());
    let request = sim::lib_lang_jvm::JvmExecutionRequest {
        classfile: include_str!("../recipes/jvm/StaticInt.hex")
            .trim()
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect(),
        class: "StaticInt".into(),
        member: "wholePipeline".into(),
        descriptor: "(II)I".into(),
        arguments: vec![8, 2],
    };
    assert!(matches!(
        sim::lib_lang_jvm::JvmSurface::new(1 << 20).execute_i32(&mut cx, request),
        sim::lib_lang_jvm::JvmExecutionOutcome::Value(20)
    ));
}
