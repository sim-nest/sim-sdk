use std::path::PathBuf;
#[cfg(feature = "shape")]
use std::sync::Arc;
#[cfg(feature = "shape")]
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "shape")]
use sim_kernel::{Args, LibTarget, Symbol};
use sim_kernel::{LibLoader, LibSource};

#[cfg(all(feature = "codec-binary", feature = "shape"))]
use super::support::pack_output_file;
#[cfg(feature = "shape")]
use super::support::{
    TickCallable, cx_with_lisp_codec, register_truthy_macro, source_defmacro_fixture,
    source_fixture, source_macro_fixture, write_source_file,
};

#[test]
fn lisp_source_loader_accepts_lisp_paths() {
    let loader = crate::loaders::LispSourceLoader::default();
    assert!(loader.can_load(&LibSource::Path(PathBuf::from("lib.lisp"))));
    assert!(!loader.can_load(&LibSource::Bytes(Vec::new())));
}

#[cfg(feature = "shape")]
#[test]
fn lisp_source_loader_reexports_existing_runtime_objects() {
    let mut cx = cx_with_lisp_codec();
    let counter = Arc::new(AtomicUsize::new(0));
    let tick = cx
        .factory()
        .opaque(Arc::new(TickCallable {
            counter: counter.clone(),
        }))
        .unwrap();
    cx.registry_mut()
        .register_function_value(Symbol::new("tick"), tick)
        .unwrap();

    let path = write_source_file("reexports", source_fixture());

    let registry = crate::loaders::standard_loader_registry();
    registry
        .load_and_register(&mut cx, LibSource::Path(path.clone()))
        .unwrap();

    let value = cx
        .call_function(&Symbol::qualified("loader", "tick"), Args::new(Vec::new()))
        .unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 1);
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        sim_kernel::Expr::Number(sim_kernel::NumberLiteral {
            domain: Symbol::qualified("numbers", "f64"),
            canonical: "1".to_owned(),
        })
    );
    assert!(
        cx.resolve_shape(&Symbol::qualified("loader", "Expr"))
            .is_ok()
    );
    assert!(
        cx.resolve_codec(&Symbol::qualified("loader", "lisp"))
            .is_ok()
    );
    assert!(
        cx.resolve_number_domain(&Symbol::qualified("loader", "f64"))
            .is_ok()
    );
    assert_eq!(
        cx.registry()
            .lib(&Symbol::qualified("loader", "source-demo"))
            .unwrap()
            .manifest
            .target,
        LibTarget::CodecSource(Symbol::qualified("codec", "lisp"))
    );

    let _ = std::fs::remove_file(path);
}

#[cfg(feature = "shape")]
#[test]
fn registry_can_inspect_lisp_source_manifest_before_load() {
    let mut cx = cx_with_lisp_codec();
    let path = write_source_file("inspect-source", source_fixture());
    let registry = crate::loaders::standard_loader_registry();

    let manifest = registry
        .inspect_manifest(&mut cx, LibSource::Path(path.clone()))
        .unwrap();

    assert_eq!(manifest.id, Symbol::qualified("loader", "source-demo"));
    assert!(manifest.capabilities.is_empty());

    let _ = std::fs::remove_file(path);
}

#[cfg(feature = "shape")]
#[test]
fn lisp_source_loader_reexports_existing_macros() {
    let mut cx = cx_with_lisp_codec();
    cx.grant(sim_kernel::macro_expand_capability());
    register_truthy_macro(&mut cx);

    let path = write_source_file("macro-reexports", source_macro_fixture());

    let registry = crate::loaders::standard_loader_registry();
    registry
        .load_and_register(&mut cx, LibSource::Path(path.clone()))
        .unwrap();

    assert!(
        cx.registry()
            .macro_by_symbol(&Symbol::qualified("loader", "truthy"))
            .is_some()
    );
    let expanded = cx
        .expand_macros(
            sim_kernel::Phase::Expand,
            sim_kernel::Expr::List(vec![sim_kernel::Expr::Symbol(Symbol::qualified(
                "loader", "truthy",
            ))]),
        )
        .unwrap();
    assert_eq!(expanded, sim_kernel::Expr::Bool(true));

    let _ = std::fs::remove_file(path);
}

#[cfg(feature = "shape")]
#[test]
fn lisp_source_loader_authors_defmacro_from_source() {
    let mut cx = cx_with_lisp_codec();
    cx.grant(sim_kernel::macro_expand_capability());

    let path = write_source_file("macro-authored", source_defmacro_fixture());

    let registry = crate::loaders::standard_loader_registry();
    registry
        .load_and_register(&mut cx, LibSource::Path(path.clone()))
        .unwrap();

    assert!(
        cx.registry()
            .macro_by_symbol(&Symbol::qualified("loader", "when"))
            .is_some()
    );
    let expanded = cx
        .expand_macros(
            sim_kernel::Phase::Expand,
            sim_kernel::Expr::List(vec![
                sim_kernel::Expr::Symbol(Symbol::qualified("loader", "when")),
                sim_kernel::Expr::Symbol(Symbol::new("ready")),
                sim_kernel::Expr::List(vec![
                    sim_kernel::Expr::Symbol(Symbol::new("send")),
                    sim_kernel::Expr::Symbol(Symbol::new("report")),
                ]),
            ]),
        )
        .unwrap();
    assert_eq!(
        expanded,
        sim_kernel::Expr::List(vec![
            sim_kernel::Expr::Symbol(Symbol::new("if")),
            sim_kernel::Expr::Symbol(Symbol::new("ready")),
            sim_kernel::Expr::List(vec![
                sim_kernel::Expr::Symbol(Symbol::new("do")),
                sim_kernel::Expr::List(vec![
                    sim_kernel::Expr::Symbol(Symbol::new("send")),
                    sim_kernel::Expr::Symbol(Symbol::new("report")),
                ]),
            ]),
            sim_kernel::Expr::Nil,
        ])
    );

    let _ = std::fs::remove_file(path);
}

#[cfg(feature = "shape")]
#[test]
fn lisp_source_loader_reports_invalid_manifest_shape() {
    let mut cx = cx_with_lisp_codec();
    let path = write_source_file("invalid", "\"not a lib form\"");
    let registry = crate::loaders::standard_loader_registry();
    let err = registry
        .load_lib(&mut cx, LibSource::Path(path.clone()))
        .err()
        .unwrap();
    assert!(matches!(err, sim_kernel::Error::Lib(_)));
    let _ = std::fs::remove_file(path);
}

#[cfg(feature = "shape")]
#[test]
fn lisp_source_loader_rejects_arbitrary_top_level_eval() {
    let mut cx = cx_with_lisp_codec();
    let path = write_source_file("non-manifest", "(print \"hi\")");
    let registry = crate::loaders::standard_loader_registry();
    let err = registry
        .load_lib(&mut cx, LibSource::Path(path.clone()))
        .err()
        .unwrap();
    assert!(matches!(
        err,
        sim_kernel::Error::Lib(message)
            if message.contains("sim_lib manifest dialect form")
                || message.contains("expected symbol sim_lib")
    ));
    let _ = std::fs::remove_file(path);
}

#[cfg(all(feature = "codec-binary", feature = "shape"))]
#[test]
fn lisp_source_compiles_to_binary_pack_shape() {
    let mut cx = cx_with_lisp_codec();
    let expr = sim_codec::decode_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "lisp"),
        sim_codec::Input::Text(source_fixture().to_owned()),
        sim_kernel::ReadPolicy::default(),
    )
    .unwrap();
    let pack = crate::loaders::compile_lisp_source_pack(PathBuf::from("demo.lisp"), expr).unwrap();
    let bytes = crate::loaders::encode_binary_lib_pack(&pack).unwrap();
    let decoded = crate::loaders::decode_binary_lib_pack(&bytes).unwrap();

    assert_eq!(
        decoded.manifest.id,
        Symbol::qualified("loader", "source-demo")
    );
    assert_eq!(
        decoded.manifest.target,
        LibTarget::CodecSource(Symbol::qualified("codec", "lisp"))
    );
    assert_eq!(decoded.exports, pack.exports);
}

#[cfg(all(feature = "codec-binary", feature = "shape"))]
#[test]
fn binary_pack_compile_rejects_authored_defmacro() {
    let mut cx = cx_with_lisp_codec();
    let expr = sim_codec::decode_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "lisp"),
        sim_codec::Input::Text(source_defmacro_fixture().to_owned()),
        sim_kernel::ReadPolicy::default(),
    )
    .unwrap();
    let error =
        crate::loaders::compile_lisp_source_pack(PathBuf::from("demo.lisp"), expr).unwrap_err();
    assert!(matches!(error, sim_kernel::Error::Lib(message) if message.contains("defmacro")));
}

#[cfg(all(feature = "codec-binary", feature = "shape"))]
#[test]
fn lisp_source_text_can_encode_directly_to_pack_bytes() {
    let mut cx = cx_with_lisp_codec();
    let bytes = crate::loaders::encode_lisp_source_text_to_binary_pack(
        &mut cx,
        &Symbol::qualified("codec", "lisp"),
        PathBuf::from("inline-demo.lisp"),
        source_fixture(),
    )
    .unwrap();
    let decoded = crate::loaders::decode_binary_lib_pack(&bytes).unwrap();
    assert_eq!(
        decoded.manifest.id,
        Symbol::qualified("loader", "source-demo")
    );
    assert_eq!(
        decoded.manifest.target,
        LibTarget::CodecSource(Symbol::qualified("codec", "lisp"))
    );
}

#[cfg(all(feature = "codec-binary", feature = "shape"))]
#[test]
fn lisp_source_file_can_export_to_binary_pack_file() {
    let mut cx = cx_with_lisp_codec();
    let source = write_source_file("export-pack", source_fixture());
    let output = pack_output_file("export-pack");

    crate::loaders::export_lisp_source_file_to_binary_pack(
        &mut cx,
        &Symbol::qualified("codec", "lisp"),
        &source,
        &output,
    )
    .unwrap();

    let bytes = std::fs::read(&output).unwrap();
    let decoded = crate::loaders::decode_binary_lib_pack(&bytes).unwrap();
    assert_eq!(
        decoded.manifest.id,
        Symbol::qualified("loader", "source-demo")
    );
    assert_eq!(
        decoded.manifest.target,
        LibTarget::CodecSource(Symbol::qualified("codec", "lisp"))
    );

    let _ = std::fs::remove_file(source);
    let _ = std::fs::remove_file(output);
}
