use sim_kernel::{CatalogSource, LoaderRegistry, Symbol};
use std::sync::Arc;

use sim_kernel::{Cx, Lib, LibLoader, LibSource, Result};

#[cfg(feature = "codec-binary")]
use super::BinaryPackLoader;
#[cfg(feature = "codec-lisp")]
use super::LispSourceLoader;
#[cfg(all(feature = "dynamic-native", not(target_arch = "wasm32")))]
use super::NativeDylibLoader;
#[cfg(feature = "wasm")]
use super::WasmLoader;

/// Builds the portable SDK registry over an installed platform loader service.
///
/// The SDK knows only canonical loader kinds and source shapes. Concrete native,
/// wasm, source, and package mechanisms remain behind [`super::LoaderPort`] in
/// the selected platform capsule.
pub fn platform_loader_registry(port: Arc<dyn super::LoaderPort>) -> LoaderRegistry {
    let mut registry = LoaderRegistry::new().with_loader(HostLoader);
    for kind in port.loader_kinds() {
        let accepts: Option<fn(&LibSource) -> bool> = if kind.symbol()
            == &Symbol::qualified("loader", "native-v1")
        {
            Some(sim_run_loaders::is_path_source)
        } else if kind.symbol() == &Symbol::qualified("loader", "wasm-v1") {
            Some(|source| {
                sim_run_loaders::is_path_source(source) || sim_run_loaders::is_bytes_source(source)
            })
        } else if kind.symbol() == &Symbol::qualified("loader", "source-v1") {
            Some(|source| {
                sim_run_loaders::is_path_source(source) || sim_run_loaders::is_bytes_source(source)
            })
        } else if kind.symbol() == &Symbol::qualified("loader", "static-v1") {
            Some(sim_run_loaders::is_static_source)
        } else {
            None
        };
        if let Some(accepts) = accepts {
            registry.add_loader(super::PortLoader::new(port.clone(), kind, accepts));
        }
    }
    registry
}

/// Builds a loader registry with the standard loaders for the enabled features.
pub fn standard_loader_registry() -> LoaderRegistry {
    standard_loader_registry_with_sources(std::iter::empty::<(Symbol, CatalogSource)>())
}

/// Builds the standard loader registry and seeds it with catalog sources.
pub fn standard_loader_registry_with_sources(
    sources: impl IntoIterator<Item = (Symbol, CatalogSource)>,
) -> LoaderRegistry {
    let mut registry = LoaderRegistry::new().with_loader(HostLoader);
    #[cfg(all(feature = "dynamic-native", not(target_arch = "wasm32")))]
    {
        registry.add_loader(NativeDylibLoader);
    }
    #[cfg(feature = "codec-binary")]
    {
        registry.add_loader(BinaryPackLoader);
    }
    #[cfg(feature = "codec-lisp")]
    {
        registry.add_loader(LispSourceLoader::default());
    }
    for (symbol, source) in sources {
        registry.add_source(symbol, source);
    }
    registry
}

/// Builds the standard loader registry and adds a wasm loader backed by
/// `runtime`.
#[cfg(feature = "wasm")]
pub fn standard_loader_registry_with_wasm(
    runtime: Arc<dyn crate::wasm_abi::WasmRuntime>,
) -> LoaderRegistry {
    standard_loader_registry_with_wasm_and_sources(
        runtime,
        std::iter::empty::<(Symbol, CatalogSource)>(),
    )
}

/// Builds the standard loader registry with a wasm loader and catalog sources.
#[cfg(feature = "wasm")]
pub fn standard_loader_registry_with_wasm_and_sources(
    runtime: Arc<dyn crate::wasm_abi::WasmRuntime>,
    sources: impl IntoIterator<Item = (Symbol, CatalogSource)>,
) -> LoaderRegistry {
    let mut registry = standard_loader_registry_with_sources(sources);
    registry.add_loader(WasmLoader::new(runtime));
    registry
}

/// Loader for libs supplied directly as in-process host objects.
#[derive(Default)]
pub struct HostLoader;

impl LibLoader for HostLoader {
    fn can_load(&self, source: &LibSource) -> bool {
        matches!(source, LibSource::Host(_))
    }

    fn load(&self, _cx: &mut Cx, source: LibSource) -> Result<Box<dyn Lib>> {
        match source {
            LibSource::Host(lib) => Ok(lib),
            _ => Err(sim_kernel::Error::HostError(
                "host loader received non-host source".to_owned(),
            )),
        }
    }

    fn inspect_manifest(
        &self,
        _cx: &mut Cx,
        source: &LibSource,
    ) -> Result<Option<sim_kernel::LibManifest>> {
        match source {
            LibSource::Host(lib) => Ok(Some(lib.manifest())),
            _ => Ok(None),
        }
    }
}
