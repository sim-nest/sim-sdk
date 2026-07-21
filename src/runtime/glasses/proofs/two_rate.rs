use std::rc::Rc;

use sim_kernel::{Error, Expr, Result};
use sim_lib_scene::{Anchor, AnchorSpace, Transform3};
use sim_lib_stream_device::ModeledSource;
use sim_lib_stream_xr::{ModeledViturePoseSource, XrPoseSample};
use sim_lib_view::{SurfaceCaps, SurfaceCodec};
use sim_lib_view_device::{AdapterInput, DeviceSurfaceCapsExt, EncodedScene, FrameClock};
use sim_lib_view_spatial::{PoseView, SpatialSurfaceCodec, viture_loop};
use sim_value::build;

/// Result of the modeled Viture two-rate reprojection proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TwoRateProof {
    /// Number of content-rate encodes performed.
    pub content_encodes: u64,
    /// Number of coalesced pose updates reported as drops.
    pub dropped: u32,
    /// Whether the stale pose frame is marked stale.
    pub stale: bool,
    /// Prediction lead after the reprojector clamp.
    pub clamped_predict_ms: u64,
    /// Whether a pose beyond the clamp holds the prior frame.
    pub held_after_clamp: bool,
}

impl TwoRateProof {
    /// Encodes the proof as expression data for cookbook recipes.
    pub fn to_expr(&self) -> Expr {
        build::map(vec![
            ("kind", build::qsym("glasses/sdk", "two-rate-proof")),
            ("content-encodes", build::uint(self.content_encodes)),
            ("dropped", build::uint(u64::from(self.dropped))),
            ("stale", Expr::Bool(self.stale)),
            ("clamped-predict-ms", build::uint(self.clamped_predict_ms)),
            ("held-after-clamp", Expr::Bool(self.held_after_clamp)),
        ])
    }
}

/// Encodes once and reprojects a modeled Viture pose track at device rate.
pub fn prove_two_rate() -> Result<TwoRateProof> {
    let caps = SurfaceCaps::from_preset("glasses-luma-ultra", "sdk.viture")
        .ok_or_else(|| Error::HostError("Viture surface preset missing".to_owned()))?;
    let profile = caps.device_profile();
    let mut cx = sim_kernel::testing::bare_cx();
    let scene = SpatialSurfaceCodec::new().encode(&mut cx, &workspace_scene(), &caps)?;
    let encoded = EncodedScene::new(scene);
    let content_encodes = 1;
    let source = ModeledViturePoseSource;
    let (mut adapter, clock) = viture_loop(&profile, 12);

    let mut newest = pose_view(&source.at(0), 1, 4_000_000);
    for index in 0..4 {
        newest = pose_view(&source.at(index), 1, 4_000_000);
        adapter.offer(&newest);
    }
    let fresh = adapter.step(
        &clock,
        &AdapterInput::new(encoded.clone(), 41, newest, clock.tick),
        &profile,
    )?;

    let stale_clock = FrameClock::new(4, profile.rate);
    let stale_pose = pose_view(&source.at(5), 12, 40_000_000);
    adapter.offer(&stale_pose);
    let stale = adapter.step(
        &stale_clock,
        &AdapterInput::new(encoded.clone(), 41, stale_pose, 0),
        &profile,
    )?;

    let beyond = pose_view(&source.at(6), 13, 80_000_000);
    adapter.offer(&beyond);
    let held = adapter.step(
        &stale_clock,
        &AdapterInput::new(encoded, 41, beyond, 0),
        &profile,
    )?;

    Ok(TwoRateProof {
        content_encodes,
        dropped: fresh.dropped,
        stale: stale.stale,
        clamped_predict_ms: field_u64(stale.out.as_ref(), "predict-ms").unwrap_or(0),
        held_after_clamp: Rc::ptr_eq(&stale.out, &held.out),
    })
}

fn pose_view(sample: &XrPoseSample, age_ms: u64, predict_ns: u64) -> PoseView {
    let mut pose = PoseView::identity(sample.seq()).with_timing(age_ms, predict_ns);
    if let Some(position) = sample.position_m() {
        pose = pose.with_translation(position);
    }
    pose
}

fn workspace_scene() -> Expr {
    sim_lib_scene::spatial(vec![sim_lib_scene::panel(
        "review",
        sim_lib_scene::text_node("Modeled review"),
        Anchor::new(AnchorSpace::World, "desk"),
        Transform3::new([0.0, 0.0, -1.6], [0.0, 0.0, 0.0, 1.0], [1.0, 1.0, 1.0]),
    )])
}

fn field_u64(expr: &Expr, name: &str) -> Option<u64> {
    let Expr::Number(number) = sim_value::access::field(expr, name)? else {
        return None;
    };
    number.canonical.parse().ok()
}
