#[cfg(feature = "codec-binary")]
mod binary_pack;
mod reexport;
mod registry;
mod shared;
#[cfg(feature = "codec-lisp")]
mod source;
#[cfg(test)]
mod tests;

#[cfg(feature = "codec-binary")]
pub use binary_pack::{
    BinaryLibPack, BinaryPackLoader, decode_binary_lib_pack, encode_binary_lib_pack,
};
#[cfg(all(feature = "dynamic-native", not(target_arch = "wasm32")))]
pub use native::{NativeDylibLoader, encode_native_manifest_response};
/// Native dynamic-library loader compatibility exports.
#[cfg(all(feature = "dynamic-native", not(target_arch = "wasm32")))]
pub mod native {
    pub use sim_cli_loaders::{
        NativeDylibLoader, encode_native_manifest_response, validate_native_abi_header,
    };
}
pub use reexport::ReexportSpec;
pub use registry::{HostLoader, standard_loader_registry, standard_loader_registry_with_sources};
#[cfg(feature = "wasm")]
pub use registry::{
    standard_loader_registry_with_wasm, standard_loader_registry_with_wasm_and_sources,
};
#[cfg(feature = "wasm")]
pub use sim_cli_loaders::{WasmLoader, wasm_load_capability};
#[cfg(feature = "codec-lisp")]
pub use source::LispSourceLoader;
#[cfg(all(feature = "codec-lisp", feature = "codec-binary"))]
pub use source::{
    compile_lisp_source_pack, compile_lisp_source_text_to_pack,
    encode_lisp_source_text_to_binary_pack, export_lisp_source_file_to_binary_pack,
};
