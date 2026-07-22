use sim_kernel::{Error, Expr, Result};
use sim_lib_scene::GlanceMetric;
use sim_lib_stream_device::ModeledSource;
use sim_lib_stream_halo::{LuaFrameBudget, diff_glance};
use sim_lib_stream_xr::ModeledHaloMotionSource;
use sim_lib_view::SurfaceCaps;
use sim_lib_view_device::{
    DeviceSurfaceCapsExt, EncodedScene, GlanceInput, GlanceState, LocalAdapter,
};
use sim_lib_view_spatial::{halo_glance_config, halo_glance_scene};
use sim_value::{access, build};

/// Result of the modeled Halo glance-diff and local-ack proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HaloGlanceProof {
    /// Sequence from the modeled Halo motion source.
    pub modeled_seq: u64,
    /// Whether the source reduced to `scene/glance`.
    pub glance_scene: bool,
    /// Changed Lua cells emitted for the small update.
    pub delta_cells: usize,
    /// Encoded Lua bytes emitted for the small update.
    pub delta_bytes: u32,
    /// Per-tick Lua byte ceiling.
    pub budget_bytes: u32,
    /// Whether the small content change remained a small delta.
    pub small_delta: bool,
    /// Whether tap acknowledgement uses `GlyphFlash`.
    pub glyph_flash_ack: bool,
}

impl HaloGlanceProof {
    /// Encodes the proof as expression data for cookbook recipes.
    pub fn to_expr(&self) -> Expr {
        build::map(vec![
            ("kind", build::qsym("glasses/sdk", "halo-glance-proof")),
            ("modeled-seq", build::uint(self.modeled_seq)),
            ("glance-scene", Expr::Bool(self.glance_scene)),
            ("delta-cells", build::uint(self.delta_cells as u64)),
            ("delta-bytes", build::uint(u64::from(self.delta_bytes))),
            ("budget-bytes", build::uint(u64::from(self.budget_bytes))),
            ("small-delta", Expr::Bool(self.small_delta)),
            ("glyph-flash-ack", Expr::Bool(self.glyph_flash_ack)),
        ])
    }
}

/// Reduces modeled Halo content, diffs one glyph, and acknowledges a tap locally.
pub fn prove_halo_glance() -> Result<HaloGlanceProof> {
    let motion = ModeledHaloMotionSource.at(7);
    let caps = SurfaceCaps::from_preset("glasses-hud", "sdk.halo")
        .ok_or_else(|| Error::HostError("Halo surface preset missing".to_owned()))?;
    let profile = caps.device_profile();
    let previous = halo_glance_scene(&source_scene("21"), &profile)?;
    let next = halo_glance_scene(&source_scene("22"), &profile)?;
    let budget = LuaFrameBudget::new(96)?;
    let delta = diff_glance(&previous, &next, &budget)?;
    let acknowledged = halo_glance_config().adapt(
        &EncodedScene::new(next.clone()),
        &GlanceState::with_input(GlanceInput::Tap, 8),
        &profile,
    )?;

    Ok(HaloGlanceProof {
        modeled_seq: motion.seq(),
        glance_scene: scene_kind(&next).as_deref() == Some("glance"),
        delta_cells: delta.cells.len(),
        delta_bytes: delta.bytes,
        budget_bytes: budget.max_bytes_per_tick,
        small_delta: delta.is_complete()
            && delta.cells.len() <= 2
            && delta.bytes < budget.max_bytes_per_tick,
        glyph_flash_ack: access::field_sym(acknowledged.as_ref(), "ack-channel")
            .is_some_and(|symbol| symbol.name.as_ref() == "glyph-flash"),
    })
}

fn source_scene(value: &str) -> Expr {
    sim_lib_scene::node(
        "stack",
        vec![
            ("title", build::text(format!("Temperature {value}"))),
            (
                "metric",
                sim_lib_scene::glance_card(
                    "Temperature",
                    Some(GlanceMetric::new("C", value)),
                    None,
                    "info",
                    1,
                ),
            ),
            ("children", build::list(Vec::new())),
        ],
    )
}

fn scene_kind(expr: &Expr) -> Option<String> {
    let kind = sim_lib_scene::node_kind(expr)?;
    Some(kind.name.to_string())
}
