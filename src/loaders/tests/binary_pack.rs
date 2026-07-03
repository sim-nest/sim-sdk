use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use sim_kernel::{Args, Export, ExportState, LibLoader, LibSource, LibTarget, Symbol, Version};

use super::support::{TickCallable, cx, cx_with_lisp_codec, write_binary_pack_file};

#[test]
fn binary_pack_loader_accepts_pack_paths_and_bytes() {
    let loader = crate::loaders::BinaryPackLoader;
    assert!(loader.can_load(&LibSource::Path(std::path::PathBuf::from("lib.l8b"))));
    assert!(loader.can_load(&LibSource::Bytes(b"L8PKrest".to_vec())));
    assert!(!loader.can_load(&LibSource::Url("https://example.com/lib.l8b".to_owned())));
    assert!(!loader.can_load(&LibSource::Bytes(Vec::new())));
}

#[cfg(all(feature = "codec-lisp", feature = "shape"))]
#[test]
fn binary_pack_loader_reexports_existing_runtime_objects() {
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

    let pack = crate::loaders::BinaryLibPack {
        manifest: sim_kernel::LibManifest {
            id: Symbol::qualified("loader", "pack-demo"),
            version: Version("0.3.0".to_owned()),
            abi: sim_kernel::AbiVersion { major: 0, minor: 1 },
            target: LibTarget::DataOnly,
            requires: Vec::new(),
            capabilities: Vec::new(),
            exports: vec![
                Export::Function {
                    symbol: Symbol::qualified("loader", "tick-pack"),
                    function_id: None,
                },
                Export::Shape {
                    symbol: Symbol::qualified("loader", "ExprPack"),
                    shape_id: None,
                },
                Export::Codec {
                    symbol: Symbol::qualified("loader", "lisp-pack"),
                    codec_id: None,
                },
                Export::NumberDomain {
                    symbol: Symbol::qualified("loader", "f64-pack"),
                    number_domain_id: None,
                },
            ],
        },
        exports: vec![
            crate::loaders::ReexportSpec {
                kind: crate::loaders::reexport::ReexportKind::Function,
                export: Symbol::qualified("loader", "tick-pack"),
                target: Symbol::new("tick"),
            },
            crate::loaders::ReexportSpec {
                kind: crate::loaders::reexport::ReexportKind::Shape,
                export: Symbol::qualified("loader", "ExprPack"),
                target: Symbol::qualified("core", "Expr"),
            },
            crate::loaders::ReexportSpec {
                kind: crate::loaders::reexport::ReexportKind::Codec,
                export: Symbol::qualified("loader", "lisp-pack"),
                target: Symbol::qualified("codec", "lisp"),
            },
            crate::loaders::ReexportSpec {
                kind: crate::loaders::reexport::ReexportKind::NumberDomain,
                export: Symbol::qualified("loader", "f64-pack"),
                target: Symbol::qualified("numbers", "f64"),
            },
        ],
    };
    let bytes = crate::loaders::encode_binary_lib_pack(&pack).unwrap();
    let path = write_binary_pack_file("reexports", &bytes);

    let registry = crate::loaders::standard_loader_registry();
    registry
        .load_and_register(&mut cx, LibSource::Path(path.clone()))
        .unwrap();

    let value = cx
        .call_function(
            &Symbol::qualified("loader", "tick-pack"),
            Args::new(Vec::new()),
        )
        .unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 1);
    assert!(
        cx.resolve_shape(&Symbol::qualified("loader", "ExprPack"))
            .is_ok()
    );
    assert!(
        cx.resolve_codec(&Symbol::qualified("loader", "lisp-pack"))
            .is_ok()
    );
    assert!(
        cx.resolve_number_domain(&Symbol::qualified("loader", "f64-pack"))
            .is_ok()
    );
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        sim_kernel::Expr::Number(sim_kernel::NumberLiteral {
            domain: Symbol::qualified("numbers", "f64"),
            canonical: "1".to_owned(),
        })
    );
    assert_eq!(
        cx.registry()
            .lib(&Symbol::qualified("loader", "pack-demo"))
            .unwrap()
            .manifest
            .target,
        LibTarget::DataOnly
    );

    let _ = std::fs::remove_file(path);
}

#[cfg(all(feature = "codec-lisp", feature = "shape"))]
#[test]
fn registry_can_inspect_binary_pack_manifest_before_load() {
    let mut cx = cx_with_lisp_codec();
    let pack = crate::loaders::BinaryLibPack {
        manifest: sim_kernel::LibManifest {
            id: Symbol::qualified("loader", "inspect-pack"),
            version: Version("0.3.0".to_owned()),
            abi: sim_kernel::AbiVersion { major: 0, minor: 1 },
            target: LibTarget::DataOnly,
            requires: Vec::new(),
            capabilities: vec![sim_kernel::read_eval_capability()],
            exports: Vec::new(),
        },
        exports: Vec::new(),
    };
    let bytes = crate::loaders::encode_binary_lib_pack(&pack).unwrap();
    let registry = crate::loaders::standard_loader_registry();

    let manifest = registry
        .inspect_manifest(&mut cx, LibSource::Bytes(bytes))
        .unwrap();

    assert_eq!(manifest.id, Symbol::qualified("loader", "inspect-pack"));
    assert_eq!(
        manifest.capabilities,
        vec![sim_kernel::read_eval_capability()]
    );
}

#[cfg(all(feature = "codec-lisp", feature = "shape"))]
#[test]
fn binary_pack_number_domain_reexport_is_visible_in_loaded_exports() {
    let mut cx = cx_with_lisp_codec();
    let pack = crate::loaders::BinaryLibPack {
        manifest: sim_kernel::LibManifest {
            id: Symbol::qualified("loader", "browse-pack"),
            version: Version("0.3.0".to_owned()),
            abi: sim_kernel::AbiVersion { major: 0, minor: 1 },
            target: LibTarget::DataOnly,
            requires: Vec::new(),
            capabilities: Vec::new(),
            exports: vec![Export::NumberDomain {
                symbol: Symbol::qualified("loader", "f64-browse"),
                number_domain_id: None,
            }],
        },
        exports: vec![crate::loaders::ReexportSpec {
            kind: crate::loaders::reexport::ReexportKind::NumberDomain,
            export: Symbol::qualified("loader", "f64-browse"),
            target: Symbol::qualified("numbers", "f64"),
        }],
    };

    crate::loaders::standard_loader_registry()
        .load_and_register(
            &mut cx,
            LibSource::Bytes(crate::loaders::encode_binary_lib_pack(&pack).unwrap()),
        )
        .unwrap();

    let loaded = cx
        .registry()
        .lib(&Symbol::qualified("loader", "browse-pack"))
        .unwrap();
    assert!(loaded.exports.iter().any(|export| {
        export.symbol == Symbol::qualified("loader", "f64-browse")
            && matches!(export.state, ExportState::Resolved { .. })
    }));
}

#[test]
fn binary_pack_loader_rejects_bad_magic() {
    let loader = crate::loaders::BinaryPackLoader;
    let err = match loader.load(&mut cx(), LibSource::Bytes(b"nope".to_vec())) {
        Ok(_) => panic!("expected binary pack load failure"),
        Err(err) => err,
    };
    assert!(matches!(err, sim_kernel::Error::HostError(_)));
}
