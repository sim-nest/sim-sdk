mod registry;
#[cfg(test)]
mod tests;

#[cfg(all(feature = "dynamic-native", not(target_arch = "wasm32")))]
pub use native::{NativeDylibLoader, encode_native_manifest_response};
#[cfg(feature = "codec-binary")]
pub use sim_run_loaders::{
    BinaryLibPack, BinaryPackLoader, decode_binary_lib_pack, encode_binary_lib_pack,
};
/// Native dynamic-library loader compatibility exports.
#[cfg(all(feature = "dynamic-native", not(target_arch = "wasm32")))]
pub mod native {
    pub use sim_run_loaders::{
        NativeDylibLoader, encode_native_manifest_response, validate_native_abi_header,
    };
}
pub use registry::{HostLoader, standard_loader_registry, standard_loader_registry_with_sources};
#[cfg(feature = "wasm")]
pub use registry::{
    standard_loader_registry_with_wasm, standard_loader_registry_with_wasm_and_sources,
};
#[cfg(feature = "codec-lisp")]
pub use sim_run_loaders::LispSourceLoader;
pub use sim_run_loaders::{ReexportKind, ReexportSpec};
#[cfg(feature = "wasm")]
pub use sim_run_loaders::{WasmLoader, wasm_load_capability};
#[cfg(any(
    feature = "codec-binary",
    feature = "codec-lisp",
    feature = "dynamic-native",
    feature = "wasm"
))]
pub use sim_run_loaders::{
    bytes_from_payload, bytes_from_source, bytes_source, bytes_source_kind, bytes_source_spec,
    catalog_bytes_source, catalog_content_address_source, catalog_path_source, catalog_url_source,
    content_address_payload, content_address_source, content_address_source_kind,
    content_address_source_spec, is_bytes_source, is_path_source, is_url_source, path_from_payload,
    path_from_source, path_payload, path_source, path_source_kind, path_source_spec,
    url_from_payload, url_from_source, url_source, url_source_kind, url_source_spec,
};
#[cfg(all(feature = "codec-lisp", feature = "codec-binary"))]
pub use sim_run_loaders::{
    compile_lisp_source_pack, compile_lisp_source_text_to_pack,
    encode_lisp_source_text_to_binary_pack, export_lisp_source_file_to_binary_pack,
};
