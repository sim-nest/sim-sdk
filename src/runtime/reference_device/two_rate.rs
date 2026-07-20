use sim_kernel::{Expr, Result, Symbol};
use sim_lib_stream_device::seq_is_monotone;
use sim_lib_view_device::{
    AdapterInput, AdapterLoop, FrameClock, GlanceAdapter, GlanceBudget, GlanceInput, GlanceState,
    StalePolicy,
};
use sim_value::build;

use super::{
    ReferencePose, ReferenceRichAdapter, ReferenceSceneEncoder, reference_caps_source,
    reference_glance_profile, reference_rich_profile,
};

/// Result of the hardware-free two-rate proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TwoRateProof {
    /// Number of content encoder calls.
    pub encoder_calls: u64,
    /// Rich loop drop count from coalesced modeled samples.
    pub rich_dropped: u32,
    /// Whether the rich loop marks a stale sample.
    pub rich_stale: bool,
    /// Compact glance budget cells.
    pub glance_cells: u8,
    /// Compact glance ack channel token.
    pub glance_ack: Symbol,
    /// Whether the modeled capability stream is monotone.
    pub modeled_stream_monotone: bool,
}

impl TwoRateProof {
    /// Encodes the proof as expression data for cookbook recipes.
    pub fn to_expr(&self) -> Expr {
        build::map(vec![
            ("kind", build::qsym("device/reference", "two-rate-proof")),
            ("encoder-calls", build::uint(self.encoder_calls)),
            ("rich-dropped", build::uint(u64::from(self.rich_dropped))),
            ("rich-stale", Expr::Bool(self.rich_stale)),
            ("glance-cells", build::uint(u64::from(self.glance_cells))),
            ("glance-ack", Expr::Symbol(self.glance_ack.clone())),
            (
                "modeled-stream-monotone",
                Expr::Bool(self.modeled_stream_monotone),
            ),
        ])
    }
}

/// Runs the two-rate modeled timing proof.
pub fn prove_two_rate() -> Result<TwoRateProof> {
    let rich_profile = reference_rich_profile();
    let glance_profile = reference_glance_profile();
    let mut encoder = ReferenceSceneEncoder::new();
    let encoded = encoder.encode();
    let shared_scene = encoded.shared();

    let mut rich_loop = AdapterLoop::new(ReferenceRichAdapter, StalePolicy::Predict);
    rich_loop.offer(&ReferencePose::new(1, 100));
    rich_loop.offer(&ReferencePose::new(2, 250));
    rich_loop.offer(&ReferencePose::new(3, 500));
    let rich_input = AdapterInput::new(encoded.clone(), 1, ReferencePose::new(3, 500), 3);
    let fresh = rich_loop.step(
        &FrameClock::new(3, rich_profile.rate),
        &rich_input,
        &rich_profile,
    )?;

    let stale_input =
        AdapterInput::from_shared_scene(shared_scene, 1, ReferencePose::new(3, 500), 3);
    let stale = rich_loop.step(
        &FrameClock::new(200, rich_profile.rate),
        &stale_input,
        &rich_profile,
    )?;

    let budget = GlanceBudget::mono_hud();
    let glance_adapter = GlanceAdapter::new(budget, 25);
    let mut glance_loop = AdapterLoop::new(glance_adapter, StalePolicy::HoldLast);
    glance_loop.offer(&GlanceState::with_input(GlanceInput::Tap, 4));
    let glance_input =
        AdapterInput::new(encoded, 1, GlanceState::with_input(GlanceInput::Tap, 4), 4);
    let _glance = glance_loop.step(
        &FrameClock::new(4, glance_profile.rate),
        &glance_input,
        &glance_profile,
    )?;

    Ok(TwoRateProof {
        encoder_calls: encoder.calls(),
        rich_dropped: fresh.dropped,
        rich_stale: stale.stale,
        glance_cells: budget.cells,
        glance_ack: budget.ack.to_symbol(),
        modeled_stream_monotone: seq_is_monotone(&reference_caps_source(), 0, 4),
    })
}
