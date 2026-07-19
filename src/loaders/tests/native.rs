use std::path::PathBuf;

use sim_kernel::LibLoader;

use super::support::cx;

#[test]
fn native_dylib_loader_accepts_platform_library_extensions() {
    let loader = crate::loaders::NativeDylibLoader;
    assert!(loader.can_load(&sim_run_loaders::path_source(PathBuf::from("libdemo.so"))));
    assert!(loader.can_load(&sim_run_loaders::path_source(PathBuf::from(
        "libdemo.dylib"
    ))));
    assert!(loader.can_load(&sim_run_loaders::path_source(PathBuf::from("demo.dll"))));
    assert!(!loader.can_load(&sim_run_loaders::path_source(PathBuf::from("demo.wasm"))));
    assert!(!loader.can_load(&sim_run_loaders::bytes_source(Vec::new())));
}

#[test]
fn native_dylib_loader_requires_capability_before_loading() {
    let mut cx = cx();
    let loader = crate::loaders::NativeDylibLoader;
    let err = match loader.load(
        &mut cx,
        sim_run_loaders::path_source(PathBuf::from("missing.so")),
    ) {
        Ok(_) => panic!("expected native dylib load to require a capability"),
        Err(err) => err,
    };
    assert!(matches!(
        err,
        sim_kernel::Error::CapabilityDenied { capability }
            if capability == sim_kernel::native_dynamic_load_capability()
    ));
}

#[test]
fn native_dylib_header_validation_rejects_truncated_header() {
    let path = PathBuf::from("truncated.so");
    let err = crate::loaders::native::validate_native_abi_header(
        &sim_kernel::NativeLibAbiHeaderV1::new(
            sim_kernel::NativeLibAbiV1::HEADER_SIZE - 1,
            sim_kernel::NATIVE_LIB_ABI_V1_MAJOR,
            sim_kernel::NATIVE_LIB_ABI_V1_MINOR,
        ),
        path.as_path(),
    )
    .unwrap_err();

    assert!(matches!(
        err,
        sim_kernel::Error::HostError(message)
            if message.contains("smaller than host header")
    ));
}

#[test]
fn native_dylib_header_validation_rejects_wrong_major() {
    let path = PathBuf::from("wrong-major.so");
    let err = crate::loaders::native::validate_native_abi_header(
        &sim_kernel::NativeLibAbiHeaderV1::new(
            sim_kernel::NativeLibAbiV1::HEADER_SIZE,
            sim_kernel::NATIVE_LIB_ABI_V1_MAJOR + 1,
            sim_kernel::NATIVE_LIB_ABI_V1_MINOR,
        ),
        path.as_path(),
    )
    .unwrap_err();

    assert!(matches!(
        err,
        sim_kernel::Error::HostError(message)
            if message.contains("unsupported native ABI")
    ));
}

#[test]
fn standard_registry_includes_native_loader_when_enabled() {
    let mut cx = cx();
    cx.grant(sim_kernel::native_dynamic_load_capability());
    let err = match crate::loaders::standard_loader_registry().load_lib(
        &mut cx,
        sim_run_loaders::path_source(PathBuf::from("missing.so")),
    ) {
        Ok(_) => panic!("expected missing native dylib path to fail"),
        Err(err) => err,
    };
    match err {
        sim_kernel::Error::HostError(message) => {
            assert!(message.contains("failed to open native dylib"));
        }
        other => panic!("expected host error, found {other:?}"),
    }
}
