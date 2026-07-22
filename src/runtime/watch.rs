//! Watch SDK facade, modeled install helper, and cookbook proof callables.
//!
//! The facade stays thin: it installs the shared device stream base, the worn
//! stream library, and a small host-registered SDK lib that exposes
//! hardware-free proofs over the landed watch contracts.

use std::sync::Arc;

use sim_kernel::{
    AbiVersion, Cx, Dependency, Export, Lib, LibManifest, LibTarget, Linker, Result, Symbol,
    Version,
};

#[cfg(feature = "cookbook")]
mod cookbook;
mod proof_functions;
mod proofs;

#[cfg(feature = "cookbook")]
pub use cookbook::RECIPES;
use proof_functions::{ProofKind, WatchProofFunction, proof_function_symbol};
pub use proofs::{
    DualQuorumProof, GlancePagerProof, HoldLastProof, PrivacyReaperProof, prove_dual_quorum,
    prove_glance_pager, prove_hold_last, prove_privacy_reaper,
};

/// SDK install mode for the watch stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WatchInstallMode {
    /// Deterministic modeled sources only.
    Modeled,
    /// Include the hardware provider bridge when the feature is enabled.
    Hardware,
}

/// Host-registered lib that exposes watch cookbook proof callables.
pub struct WatchStackLib;

impl Lib for WatchStackLib {
    fn manifest(&self) -> LibManifest {
        LibManifest {
            id: watch_stack_manifest_symbol(),
            version: Version(env!("CARGO_PKG_VERSION").to_owned()),
            abi: AbiVersion { major: 0, minor: 1 },
            target: LibTarget::HostRegistered,
            requires: vec![Dependency {
                id: sim_lib_stream_wrist::wrist_stream_manifest_symbol(),
                minimum_version: None,
            }],
            capabilities: Vec::new(),
            exports: ProofKind::ALL
                .into_iter()
                .map(|kind| Export::Value {
                    symbol: proof_function_symbol(kind),
                })
                .collect(),
        }
    }

    fn load(&self, cx: &mut sim_kernel::LoadCx, linker: &mut Linker<'_>) -> Result<()> {
        for kind in ProofKind::ALL {
            linker.value(
                proof_function_symbol(kind),
                cx.factory().opaque(Arc::new(WatchProofFunction { kind }))?,
            )?;
        }
        Ok(())
    }
}

/// Installs the modeled watch SDK stack into a context.
pub fn install_watch_stack(cx: &mut Cx, mode: WatchInstallMode) -> Result<()> {
    sim_lib_stream_wrist::install_wrist_stream_lib(cx)?;
    if mode == WatchInstallMode::Hardware {
        ensure_hardware_feature()?;
    }
    sim_lib_core::install_once(cx, &WatchStackLib)?;
    Ok(())
}

/// Returns the manifest id for the watch SDK facade.
pub fn watch_stack_manifest_symbol() -> Symbol {
    Symbol::qualified("watch", "sdk")
}

/// Reads a boolean field from a proof expression.
#[cfg(test)]
pub(crate) fn bool_field(expr: &sim_kernel::Expr, field: &'static str) -> bool {
    sim_value::access::field_bool(expr, field).unwrap_or(false)
}

#[cfg(feature = "watch-hardware")]
fn ensure_hardware_feature() -> Result<()> {
    let _provider = sim_lib_stream_wristbridge::watch_stub_provider();
    Ok(())
}

#[cfg(not(feature = "watch-hardware"))]
fn ensure_hardware_feature() -> Result<()> {
    Err(sim_kernel::Error::Eval(
        "watch hardware install requires the watch-hardware feature".to_owned(),
    ))
}
