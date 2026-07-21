//! Glasses SDK facade, modeled install helper, and cookbook proof callables.
//!
//! The facade installs the shared DEVICE_3 base and XR stream lib, then exposes
//! deterministic proofs over the spatial, Halo, co-use, voice-site, and BRIDGE
//! review contracts. Provider modes only select already-linked host providers.

use std::sync::Arc;

use sim_kernel::{
    AbiVersion, Cx, Dependency, Export, Lib, LibManifest, LibTarget, Linker, Result, Symbol,
    Version,
};

mod asr_site;
#[cfg(feature = "cookbook")]
mod cookbook;
mod proof_functions;
mod proofs;

pub use asr_site::modeled_asr_site_symbol;
#[cfg(feature = "cookbook")]
pub use cookbook::RECIPES;
use proof_functions::{GlassesProofFunction, ProofKind, proof_function_symbol};
pub use proofs::{
    CoUseProof, HaloGlanceProof, ReviewInSpaceProof, TwoRateProof, VoiceSiteProof, prove_co_use,
    prove_halo_glance, prove_review_in_space, prove_two_rate, prove_voice_site,
};

/// SDK install mode for the glasses stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlassesInstallMode {
    /// Deterministic modeled sources and surfaces only.
    Modeled,
    /// Enable the Viture provider lane.
    Viture,
    /// Enable the Halo provider lane.
    Halo,
    /// Enable both provider lanes.
    Both,
}

/// Host-registered lib that exposes glasses cookbook proofs and modeled ASR.
pub struct GlassesStackLib;

impl Lib for GlassesStackLib {
    fn manifest(&self) -> LibManifest {
        let mut exports = ProofKind::ALL
            .into_iter()
            .map(|kind| Export::Value {
                symbol: proof_function_symbol(kind),
            })
            .collect::<Vec<_>>();
        exports.push(Export::Site {
            symbol: modeled_asr_site_symbol(),
            runtime_id: None,
        });
        LibManifest {
            id: glasses_stack_manifest_symbol(),
            version: Version(env!("CARGO_PKG_VERSION").to_owned()),
            abi: AbiVersion { major: 0, minor: 1 },
            target: LibTarget::HostRegistered,
            requires: vec![Dependency {
                id: sim_lib_stream_xr::xr_stream_manifest_symbol(),
                minimum_version: None,
            }],
            capabilities: Vec::new(),
            exports,
        }
    }

    fn load(&self, cx: &mut sim_kernel::LoadCx, linker: &mut Linker<'_>) -> Result<()> {
        for kind in ProofKind::ALL {
            linker.value(
                proof_function_symbol(kind),
                cx.factory()
                    .opaque(Arc::new(GlassesProofFunction { kind }))?,
            )?;
        }
        linker.site_value(
            modeled_asr_site_symbol(),
            cx.factory().opaque(Arc::new(asr_site::ModeledAsrSite))?,
        )?;
        Ok(())
    }
}

/// Installs the shared device base, XR stream lib, and glasses SDK facade.
pub fn install_glasses_stack(cx: &mut Cx, mode: GlassesInstallMode) -> Result<()> {
    super::reference_device::install_device_base(cx)?;
    sim_lib_stream_xr::install_xr_stream_lib(cx)?;
    if matches!(mode, GlassesInstallMode::Viture | GlassesInstallMode::Both) {
        ensure_viture_feature()?;
    }
    if matches!(mode, GlassesInstallMode::Halo | GlassesInstallMode::Both) {
        ensure_halo_feature()?;
    }
    sim_lib_core::install_once(cx, &GlassesStackLib)?;
    Ok(())
}

/// Returns the manifest id for the glasses SDK facade.
pub fn glasses_stack_manifest_symbol() -> Symbol {
    Symbol::qualified("glasses", "sdk")
}

#[cfg(feature = "glasses-viture")]
fn ensure_viture_feature() -> Result<()> {
    let _provider = sim_lib_stream_viture::VitureProvider::stub();
    Ok(())
}

#[cfg(not(feature = "glasses-viture"))]
fn ensure_viture_feature() -> Result<()> {
    Err(sim_kernel::Error::Eval(
        "Viture install requires the glasses-viture feature".to_owned(),
    ))
}

#[cfg(feature = "glasses-halo")]
fn ensure_halo_feature() -> Result<()> {
    let _provider = sim_lib_stream_halo::halo_stub_provider();
    Ok(())
}

#[cfg(not(feature = "glasses-halo"))]
fn ensure_halo_feature() -> Result<()> {
    Err(sim_kernel::Error::Eval(
        "Halo install requires the glasses-halo feature".to_owned(),
    ))
}
