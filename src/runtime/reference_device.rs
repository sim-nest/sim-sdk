//! Hardware-free reference device profiles, adapters, and cookbook proofs.
//!
//! The reference device is a reusable modeled target for device bootstrapping:
//! it exposes one rich pose-coupled profile, one compact glance profile, a
//! deterministic sample source, and proof callables that exercise timing,
//! consent, retention, and route rebinding without touching real hardware.

use std::sync::Arc;

use sim_kernel::{AbiVersion, Cx, Export, Lib, LibManifest, LibTarget, Linker, Result, Version};

mod consent;
#[cfg(feature = "cookbook")]
mod cookbook;
mod profiles;
mod proof_functions;
mod route;
mod two_rate;

pub use consent::{
    ConsentProof, RetentionProof, prove_consent_without_kernel_grant, prove_retention_reaper,
    reference_edge_id, reference_pose_receipt, require_reference_pose,
};
#[cfg(feature = "cookbook")]
pub use cookbook::RECIPES;
pub use profiles::{
    ReferencePose, ReferenceRichAdapter, ReferenceSceneEncoder, reference_caps_source,
    reference_glance_profile, reference_glance_profile_symbol, reference_rich_profile,
    reference_rich_profile_symbol, reference_scene,
};
use proof_functions::{ProofKind, ReferenceProofFunction, proof_function_symbol};
pub use route::{RouteSwapProof, prove_route_swap};
pub use two_rate::{TwoRateProof, prove_two_rate};

/// Host-registered lib that exposes reference profiles and proof callables.
pub struct ReferenceDeviceLib;

impl Lib for ReferenceDeviceLib {
    fn manifest(&self) -> LibManifest {
        LibManifest {
            id: reference_device_manifest_symbol(),
            version: Version(env!("CARGO_PKG_VERSION").to_owned()),
            abi: AbiVersion { major: 0, minor: 1 },
            target: LibTarget::HostRegistered,
            requires: Vec::new(),
            capabilities: Vec::new(),
            exports: vec![
                Export::Value {
                    symbol: reference_rich_profile_symbol(),
                },
                Export::Value {
                    symbol: reference_glance_profile_symbol(),
                },
                Export::Value {
                    symbol: proof_function_symbol(ProofKind::TwoRate),
                },
                Export::Value {
                    symbol: proof_function_symbol(ProofKind::Consent),
                },
                Export::Value {
                    symbol: proof_function_symbol(ProofKind::RouteSwap),
                },
            ],
        }
    }

    fn load(&self, cx: &mut sim_kernel::LoadCx, linker: &mut Linker<'_>) -> Result<()> {
        linker.value(
            reference_rich_profile_symbol(),
            cx.factory().expr(reference_rich_profile().to_expr())?,
        )?;
        linker.value(
            reference_glance_profile_symbol(),
            cx.factory().expr(reference_glance_profile().to_expr())?,
        )?;
        for kind in ProofKind::ALL {
            linker.value(
                proof_function_symbol(kind),
                cx.factory()
                    .opaque(Arc::new(ReferenceProofFunction { kind }))?,
            )?;
        }
        Ok(())
    }
}

/// Installs the device base and the SDK reference-device exports.
pub fn install_device_base(cx: &mut Cx) -> Result<()> {
    sim_lib_stream_device::install_device_stream_base(cx)?;
    sim_lib_core::install_once(cx, &ReferenceDeviceLib)?;
    Ok(())
}

/// Installs the modeled reference device into a context.
pub fn install_reference_device(cx: &mut Cx) -> Result<()> {
    install_device_base(cx)
}

/// Returns the manifest id for the reference-device facade.
pub fn reference_device_manifest_symbol() -> sim_kernel::Symbol {
    sim_kernel::Symbol::qualified("device", "reference")
}

/// Reads a boolean field from a proof expression.
#[cfg(test)]
pub(crate) fn bool_field(expr: &sim_kernel::Expr, field: &'static str) -> bool {
    sim_value::access::field_bool(expr, field).unwrap_or(false)
}
