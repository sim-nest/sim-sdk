use std::rc::Rc;

use sim_kernel::{Expr, Result, Symbol};
use sim_lib_stream_device::ModeledDeviceCapsSource;
use sim_lib_view_device::{
    DeviceProfile, DeviceProfileParts, EncodedScene, LocalAdapter, RateClass,
};
use sim_value::build;

/// Symbol naming the rich reference-device profile export.
pub fn reference_rich_profile_symbol() -> Symbol {
    Symbol::qualified("device/reference", "rich-profile")
}

/// Symbol naming the compact glance reference profile export.
pub fn reference_glance_profile_symbol() -> Symbol {
    Symbol::qualified("device/reference", "glance-profile")
}

/// Builds the rich, pose-coupled reference device profile.
pub fn reference_rich_profile() -> DeviceProfile {
    DeviceProfile::new(DeviceProfileParts {
        kind: Symbol::qualified("device", "reference-device"),
        display: symbols(&["stereo", "hud"]),
        input: symbols(&["tap"]),
        output: symbols(&["hud", "haptic"]),
        links: symbols(&["local"]),
        streams: symbols(&["pose"]),
        rate: RateClass::stereo(),
        policy: build::map(vec![
            ("consent", build::sym("required")),
            ("retention-ms", build::uint(100)),
        ]),
    })
}

/// Builds the actuator-tier glance reference profile.
pub fn reference_glance_profile() -> DeviceProfile {
    DeviceProfile::new(DeviceProfileParts {
        kind: Symbol::qualified("device", "reference-glance"),
        display: symbols(&["round"]),
        input: symbols(&["tap"]),
        output: symbols(&["haptic"]),
        links: symbols(&["local"]),
        streams: Vec::new(),
        rate: RateClass::watch(),
        policy: build::map(vec![
            ("consent", build::sym("visible")),
            ("retention-ms", build::uint(50)),
        ]),
    })
}

/// Builds the deterministic modeled stream-facing capability source.
pub fn reference_caps_source() -> ModeledDeviceCapsSource {
    ModeledDeviceCapsSource::new(
        Symbol::qualified("device", "reference-device"),
        vec![Symbol::qualified("device/stream", "pose")],
        vec![Symbol::qualified("device/input", "tap")],
        vec![
            Symbol::qualified("device/output", "hud"),
            Symbol::qualified("device/output", "haptic"),
        ],
    )
    .with_seq_base(10)
}

/// Device-local pose state consumed by the rich reference adapter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferencePose {
    /// Modeled pose sample sequence.
    pub seq: u64,
    /// Yaw in millidegrees.
    pub yaw_mdeg: i32,
}

impl ReferencePose {
    /// Builds one deterministic pose sample.
    pub fn new(seq: u64, yaw_mdeg: i32) -> Self {
        Self { seq, yaw_mdeg }
    }
}

/// Bespoke rich-tier adapter for the modeled reference device.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReferenceRichAdapter;

impl LocalAdapter for ReferenceRichAdapter {
    type State = ReferencePose;

    fn adapt(
        &self,
        scene: &EncodedScene,
        state: &Self::State,
        profile: &DeviceProfile,
    ) -> Result<Rc<Expr>> {
        Ok(Rc::new(build::map(vec![
            ("kind", build::qsym("device/reference", "rich-frame")),
            ("tier", Expr::Symbol(profile.tier.to_symbol())),
            ("pose-seq", build::uint(state.seq)),
            ("yaw-mdeg", build::int(i64::from(state.yaw_mdeg))),
            ("scene", scene.expr().clone()),
        ])))
    }
}

/// Deterministic content encoder used by the two-rate proof.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReferenceSceneEncoder {
    calls: u64,
}

impl ReferenceSceneEncoder {
    /// Builds a fresh encoder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Encodes the reference scene once for any number of local adapters.
    pub fn encode(&mut self) -> EncodedScene {
        self.calls = self.calls.saturating_add(1);
        EncodedScene::new(reference_scene())
    }

    /// Number of content encoding calls performed.
    pub fn calls(&self) -> u64 {
        self.calls
    }
}

/// Builds the portable scene used by both reference tiers.
pub fn reference_scene() -> Expr {
    build::map(vec![
        ("kind", build::qsym("scene", "glance")),
        ("title", build::text("Reference pose")),
        (
            "metric",
            build::map(vec![
                ("label", build::text("yaw")),
                ("value", build::text("12 deg")),
            ]),
        ),
        (
            "action",
            build::map(vec![
                ("label", build::text("Acknowledge")),
                ("target", build::sym("tap")),
            ]),
        ),
        ("urgency", build::sym("info")),
        ("cells", build::uint(4)),
        ("bypass-budget", Expr::Bool(false)),
    ])
}

fn symbols(names: &[&str]) -> Vec<Symbol> {
    names.iter().map(|name| Symbol::new(*name)).collect()
}
