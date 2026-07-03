use std::path::PathBuf;
#[cfg(all(feature = "codec-binary", feature = "codec-lisp", feature = "shape"))]
use std::sync::Arc;
#[cfg(all(feature = "codec-binary", feature = "codec-lisp", feature = "shape"))]
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(all(feature = "codec-binary", feature = "codec-lisp", feature = "shape"))]
use sim_kernel::{Args, Export, LibTarget};
use sim_kernel::{ClassId, ExportState, LibSource, Symbol, Version};

use super::support::{
    BrokenResolvedExportLib, CapabilityLib, DeclaredOnlyLib, FailingLib, ResolvingLib, StubLib,
    VersionedStubLib, cx,
};
#[cfg(all(feature = "codec-binary", feature = "codec-lisp", feature = "shape"))]
use super::support::{TickCallable, cx_with_lisp_codec};

#[test]
fn host_loader_accepts_host_source() {
    let mut cx = cx();
    let registry = crate::loaders::standard_loader_registry();
    let lib = registry
        .load_lib(
            &mut cx,
            LibSource::Host(Box::new(StubLib {
                symbol: Symbol::new("host-lib"),
            })),
        )
        .unwrap();
    assert_eq!(lib.manifest().id, Symbol::new("host-lib"));
}

#[test]
fn load_and_register_uses_atomic_runtime_load() {
    let mut cx = cx();
    let registry = crate::loaders::standard_loader_registry();
    registry
        .load_and_register(
            &mut cx,
            LibSource::Host(Box::new(StubLib {
                symbol: Symbol::new("host-lib"),
            })),
        )
        .unwrap();
    assert!(cx.registry().functions().contains_key(&Symbol::new("stub")));
    assert!(cx.registry().lib(&Symbol::new("host-lib")).is_some());
}

#[test]
fn registry_reports_when_no_loader_accepts_source() {
    let mut cx = cx();
    let registry = crate::loaders::standard_loader_registry();
    let err = registry
        .load_lib(&mut cx, LibSource::Path(PathBuf::from("x.wasm")))
        .err()
        .unwrap();
    assert!(matches!(err, sim_kernel::Error::HostError(_)));
}

#[cfg(all(feature = "codec-binary", feature = "codec-lisp", feature = "shape"))]
#[test]
fn registry_can_resolve_symbol_sources_from_catalog() {
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
            id: Symbol::qualified("loader", "catalog-demo"),
            version: sim_kernel::Version("0.4.0".to_owned()),
            abi: sim_kernel::AbiVersion { major: 0, minor: 1 },
            target: LibTarget::DataOnly,
            requires: Vec::new(),
            capabilities: Vec::new(),
            exports: vec![Export::Function {
                symbol: Symbol::qualified("loader", "tick-catalog"),
                function_id: None,
            }],
        },
        exports: vec![crate::loaders::ReexportSpec {
            kind: crate::loaders::reexport::ReexportKind::Function,
            export: Symbol::qualified("loader", "tick-catalog"),
            target: Symbol::new("tick"),
        }],
    };
    let bytes = crate::loaders::encode_binary_lib_pack(&pack).unwrap();

    let registry = crate::loaders::standard_loader_registry_with_sources([(
        Symbol::qualified("loader", "catalog-demo"),
        sim_kernel::CatalogSource::Bytes(bytes),
    )]);

    registry
        .load_and_register(
            &mut cx,
            LibSource::Symbol(Symbol::qualified("loader", "catalog-demo")),
        )
        .unwrap();

    let value = cx
        .call_function(
            &Symbol::qualified("loader", "tick-catalog"),
            Args::new(Vec::new()),
        )
        .unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 1);
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        sim_kernel::Expr::Number(sim_kernel::NumberLiteral {
            domain: Symbol::qualified("numbers", "f64"),
            canonical: "1".to_owned(),
        })
    );
}

#[test]
fn registry_reports_unknown_symbol_source() {
    let mut cx = cx();
    let registry = crate::loaders::standard_loader_registry();
    let err = registry
        .load_lib(
            &mut cx,
            LibSource::Symbol(Symbol::qualified("missing", "lib")),
        )
        .err()
        .unwrap();
    match err {
        sim_kernel::Error::HostError(message) => {
            assert!(message.contains("missing/lib"));
        }
        other => panic!("expected host error, found {other:?}"),
    }
}

#[test]
fn failed_load_does_not_partially_mutate_registry() {
    let mut cx = cx();
    let registry = crate::loaders::standard_loader_registry();
    let err = registry
        .load_and_register(&mut cx, LibSource::Host(Box::new(FailingLib)))
        .err()
        .unwrap();
    assert!(matches!(err, sim_kernel::Error::HostError(_)));
    assert!(
        cx.resolve_function(&Symbol::new("half-registered"))
            .is_err()
    );
    assert!(cx.registry().lib(&Symbol::new("failing-lib")).is_none());
}

#[test]
fn commit_rejects_resolved_export_without_value() {
    let mut cx = cx();
    let registry = crate::loaders::standard_loader_registry();
    let error = registry
        .load_and_register(&mut cx, LibSource::Host(Box::new(BrokenResolvedExportLib)))
        .unwrap_err();
    assert!(
        matches!(error, sim_kernel::Error::Lib(message) if message.contains("function export broken has no value"))
    );
    assert!(cx.registry().lib(&Symbol::new("broken-lib")).is_none());
    assert!(cx.resolve_function(&Symbol::new("broken")).is_err());
}

#[test]
fn declared_export_is_visible_as_declared_not_resolved() {
    let mut cx = cx();
    let registry = crate::loaders::standard_loader_registry();
    registry
        .load_and_register(&mut cx, LibSource::Host(Box::new(DeclaredOnlyLib)))
        .unwrap();
    let loaded = cx.registry().lib(&Symbol::new("declared-lib")).unwrap();
    let export = loaded
        .exports
        .iter()
        .find(|export| export.symbol == Symbol::new("declared-only"))
        .unwrap();
    assert!(matches!(export.state, ExportState::Declared));
    assert!(cx.resolve_function(&Symbol::new("declared-only")).is_err());
}

#[test]
fn load_cx_can_resolve_existing_symbols_during_load() {
    let mut cx = cx();
    let class = cx
        .factory()
        .class_stub(ClassId(33), Symbol::new("already-there"))
        .unwrap();
    cx.registry_mut()
        .register_class_value(Symbol::new("already-there"), class)
        .unwrap();

    let registry = crate::loaders::standard_loader_registry();
    registry
        .load_and_register(&mut cx, LibSource::Host(Box::new(ResolvingLib)))
        .unwrap();

    assert!(
        cx.resolve_function(&Symbol::new("resolved-during-load"))
            .is_ok()
    );
}

#[test]
fn load_and_register_requires_manifest_declared_capabilities() {
    let mut cx = cx();
    let registry = crate::loaders::standard_loader_registry();

    let error = registry
        .load_and_register(&mut cx, LibSource::Host(Box::new(CapabilityLib)))
        .unwrap_err();

    assert!(matches!(
        error,
        sim_kernel::Error::CapabilityDenied { capability }
            if capability == sim_kernel::read_eval_capability()
    ));

    cx.grant(sim_kernel::read_eval_capability());
    registry
        .load_and_register(&mut cx, LibSource::Host(Box::new(CapabilityLib)))
        .unwrap();
    assert!(cx.registry().lib(&Symbol::new("cap-lib")).is_some());
}

#[test]
fn load_and_register_rejects_loaded_dependency_below_minimum_version() {
    let mut cx = cx();
    let registry = crate::loaders::standard_loader_registry();
    registry
        .load_and_register(
            &mut cx,
            LibSource::Host(Box::new(VersionedStubLib {
                symbol: Symbol::new("dep"),
                version: "1.5.0",
                requires: Vec::new(),
            })),
        )
        .unwrap();

    let err = registry
        .load_and_register(
            &mut cx,
            LibSource::Host(Box::new(VersionedStubLib {
                symbol: Symbol::new("user"),
                version: "0.1.0",
                requires: vec![sim_kernel::Dependency {
                    id: Symbol::new("dep"),
                    minimum_version: Some(Version("2.0.0".to_owned())),
                }],
            })),
        )
        .unwrap_err();

    assert!(matches!(
        err,
        sim_kernel::Error::DependencyVersionMismatch {
            lib,
            dependency,
            required,
            loaded
        } if lib == Symbol::new("user")
            && dependency == Symbol::new("dep")
            && required == Version("2.0.0".to_owned())
            && loaded == Version("1.5.0".to_owned())
    ));
}

#[test]
fn load_libs_rejects_dependency_below_minimum_version() {
    let mut cx = cx();
    let dep = VersionedStubLib {
        symbol: Symbol::new("dep"),
        version: "1.5.0",
        requires: Vec::new(),
    };
    let user = VersionedStubLib {
        symbol: Symbol::new("user"),
        version: "0.1.0",
        requires: vec![sim_kernel::Dependency {
            id: Symbol::new("dep"),
            minimum_version: Some(Version("2.0.0".to_owned())),
        }],
    };

    let err = cx.load_libs(&[&user, &dep]).unwrap_err();
    assert!(matches!(
        err,
        sim_kernel::Error::DependencyVersionMismatch { .. }
    ));
}

#[test]
fn standard_registry_includes_host_loader() {
    let mut cx = cx();
    let lib = crate::loaders::standard_loader_registry()
        .load_lib(
            &mut cx,
            LibSource::Host(Box::new(StubLib {
                symbol: Symbol::new("host-lib"),
            })),
        )
        .unwrap();
    assert_eq!(lib.manifest().id, Symbol::new("host-lib"));
}
