use std::sync::Arc;
use std::{fs, path::PathBuf};

use sim_kernel::{AbiVersion, Args, LibLoader, LibSource, LibTarget, Symbol};
use sim_wasm_abi::{
    AbiValue, Frame, InMemoryWasmRuntime, WasmExport, WasmGuestModule, WasmManifest,
    encode_exports_frame, encode_manifest_frame, encode_value_frame,
};

use super::support::cx;

struct LoaderWasmModule {
    manifest: WasmManifest,
    exports: Vec<WasmExport>,
}

impl WasmGuestModule for LoaderWasmModule {
    fn manifest_frame(&self) -> sim_kernel::Result<Frame> {
        encode_manifest_frame(&self.manifest)
    }

    fn exports_frame(&self) -> sim_kernel::Result<Frame> {
        encode_exports_frame(&self.exports)
    }

    fn call(&self, _function: &Symbol, _args: Frame) -> sim_kernel::Result<Frame> {
        encode_value_frame(&AbiValue::Expr(sim_kernel::Expr::Nil))
    }
}

#[test]
fn standard_registry_with_wasm_accepts_wasm_source_shape() {
    let runtime = Arc::new(crate::wasm_abi::InMemoryWasmRuntime::new());
    let registry = crate::loaders::standard_loader_registry_with_wasm(runtime);
    let mut cx = cx();
    cx.grant(crate::loaders::wasm_load_capability());
    let err = registry
        .load_lib(
            &mut cx,
            sim_kernel::LibSource::Path(PathBuf::from("x.wasm")),
        )
        .err()
        .unwrap();
    assert!(matches!(err, sim_kernel::Error::HostError(_)));
}

#[test]
fn wasm_loader_accepts_wasm_paths_and_bytes() {
    let runtime = Arc::new(crate::wasm_abi::InMemoryWasmRuntime::new());
    let loader = crate::loaders::WasmLoader::new(runtime);
    assert!(loader.can_load(&sim_kernel::LibSource::Path(PathBuf::from("lib.wasm"))));
    assert!(loader.can_load(&sim_kernel::LibSource::Bytes(b"\0asm....".to_vec())));
    assert!(!loader.can_load(&sim_kernel::LibSource::Bytes(b"L8PK".to_vec())));
    assert!(!loader.can_load(&sim_kernel::LibSource::Url(
        "https://example.com/lib.wasm".to_owned()
    )));
}

#[test]
fn wasm_loader_loads_registered_module_bytes() {
    let runtime = Arc::new(InMemoryWasmRuntime::new());
    let exports = vec![WasmExport::Function {
        symbol: Symbol::qualified("loader", "wasm-tick"),
    }];
    let manifest = WasmManifest {
        id: Symbol::qualified("loader", "wasm-demo"),
        version: "0.1.0".to_owned(),
        abi: AbiVersion { major: 0, minor: 1 },
        target: LibTarget::WasmComponent,
        requires: Vec::new(),
        capabilities: Vec::new(),
        exports: exports.clone(),
    };
    let bytes = b"\0asmloader-demo".to_vec();
    runtime
        .register_module(
            bytes.clone(),
            Arc::new(LoaderWasmModule {
                manifest: manifest.clone(),
                exports,
            }),
        )
        .unwrap();

    let registry = crate::loaders::standard_loader_registry_with_wasm(runtime);
    let mut cx = cx();
    cx.grant(crate::loaders::wasm_load_capability());
    registry
        .load_and_register(&mut cx, LibSource::Bytes(bytes))
        .unwrap();

    assert!(
        cx.registry()
            .lib(&Symbol::qualified("loader", "wasm-demo"))
            .is_some()
    );
    let value = cx
        .call_function(
            &Symbol::qualified("loader", "wasm-tick"),
            Args::new(Vec::new()),
        )
        .unwrap();
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        sim_kernel::Expr::Nil
    );
}

#[test]
fn standard_registry_with_wasm_loads_real_fixture_path() {
    let path = write_wasm_fixture();
    let registry = crate::loaders::standard_loader_registry_with_wasm(Arc::new(
        crate::wasm_abi::WasmiRuntime::new(),
    ));
    let mut cx = cx();
    cx.grant(crate::loaders::wasm_load_capability());

    registry
        .load_and_register(&mut cx, LibSource::Path(path.clone()))
        .unwrap();

    assert!(
        cx.registry()
            .lib(&Symbol::qualified("loader", "wasm-file-demo"))
            .is_some()
    );
    let value = cx
        .call_function(
            &Symbol::qualified("loader", "wasm-file-tick"),
            Args::new(Vec::new()),
        )
        .unwrap();
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        sim_kernel::Expr::Nil
    );

    let _ = fs::remove_file(path);
}

fn write_wasm_fixture() -> PathBuf {
    let path = unique_wasm_path();
    fs::write(&path, wasm_fixture_bytes()).expect("write wasm fixture");
    path
}

fn wasm_fixture_bytes() -> Vec<u8> {
    let exports = vec![WasmExport::Function {
        symbol: Symbol::qualified("loader", "wasm-file-tick"),
    }];
    let manifest = WasmManifest {
        id: Symbol::qualified("loader", "wasm-file-demo"),
        version: "0.1.0".to_owned(),
        abi: AbiVersion { major: 0, minor: 1 },
        target: LibTarget::WasmComponent,
        requires: Vec::new(),
        capabilities: Vec::new(),
        exports: exports.clone(),
    };
    let manifest_frame = encode_manifest_frame(&manifest).unwrap();
    let exports_frame = encode_exports_frame(&exports).unwrap();
    let nil_frame = encode_value_frame(&AbiValue::Expr(sim_kernel::Expr::Nil)).unwrap();
    wat::parse_str(format!(
        r#"(module
            (memory (export "memory") 1)
            (global $heap (mut i32) (i32.const 3072))
            (data (i32.const 0) "{}")
            (data (i32.const 1024) "{}")
            (data (i32.const 2048) "{}")
            (func (export "sim_alloc") (param $len i32) (result i32)
                (local $ptr i32)
                global.get $heap
                local.tee $ptr
                local.get $len
                i32.add
                global.set $heap
                local.get $ptr)
            (func (export "sim_manifest") (result i64)
                i64.const {})
            (func (export "sim_exports") (result i64)
                i64.const {})
            (func (export "sim_call") (param i32) (param i32) (param i32) (param i32) (result i64)
                i64.const {})
        )"#,
        wat_bytes(manifest_frame.bytes()),
        wat_bytes(exports_frame.bytes()),
        wat_bytes(nil_frame.bytes()),
        pack_frame_ref(0, manifest_frame.bytes().len()),
        pack_frame_ref(1024, exports_frame.bytes().len()),
        pack_frame_ref(2048, nil_frame.bytes().len()),
    ))
    .expect("hand-written wasm fixture should assemble")
}

fn pack_frame_ref(ptr: u32, len: usize) -> u64 {
    ((len as u64) << 32) | ptr as u64
}

fn wat_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("\\{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

fn unique_wasm_path() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "sim-sdk-wasm-fixture-{}-{nanos}.wasm",
        std::process::id()
    ))
}
