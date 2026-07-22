use sim_kernel::{Cx, Expr, Result};
use sim_lib_view_device::{
    ConsentReceipt, DeviceCapability, DeviceSampleStore, EdgeId, FrameClock, RateClass,
    RetentionReaper, StoreKey, StoredSample, require_with_consent,
};
use sim_value::build;

/// Result of the hardware-free consent and retention proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsentProof {
    /// Whether visible consent without a kernel grant fails closed.
    pub denied_without_kernel_grant: bool,
    /// Whether missing visible consent fails closed.
    pub denied_without_visible_grant: bool,
    /// Whether the sample is absent after the retention sweep.
    pub sample_evicted: bool,
    /// Whether content referenced only by the sample is absent after the sweep.
    pub content_evicted: bool,
}

impl ConsentProof {
    /// Encodes the proof as expression data for cookbook recipes.
    pub fn to_expr(&self) -> Expr {
        build::map(vec![
            ("kind", build::qsym("device/reference", "consent-proof")),
            (
                "denied-without-kernel-grant",
                Expr::Bool(self.denied_without_kernel_grant),
            ),
            (
                "denied-without-visible-grant",
                Expr::Bool(self.denied_without_visible_grant),
            ),
            ("sample-evicted", Expr::Bool(self.sample_evicted)),
            ("content-evicted", Expr::Bool(self.content_evicted)),
        ])
    }
}

/// Builds the stable reference device edge id.
pub fn reference_edge_id() -> EdgeId {
    EdgeId::named("reference-device")
}

/// Builds a visible pose-consent receipt for the reference edge.
pub fn reference_pose_receipt(seq: u64, retain_ms: u64) -> ConsentReceipt {
    ConsentReceipt::new(
        vec![DeviceCapability::Pose.grant_symbol()],
        retain_ms,
        Vec::new(),
        reference_edge_id(),
        seq,
    )
}

/// Requires a pose read against both kernel capability and visible consent.
pub fn require_reference_pose(cx: &Cx, receipt: &ConsentReceipt) -> Result<()> {
    require_with_consent(
        cx,
        DeviceCapability::Pose.as_str(),
        receipt,
        &reference_edge_id(),
    )
}

/// Runs the consent and retention portions that do not need test-only grants.
pub fn prove_consent_without_kernel_grant(cx: &Cx) -> ConsentProof {
    let receipt = reference_pose_receipt(7, 5);
    let empty_receipt = ConsentReceipt::new(Vec::new(), 5, Vec::new(), reference_edge_id(), 8);
    let denied_without_kernel_grant = require_reference_pose(cx, &receipt).is_err();
    let denied_without_visible_grant = require_reference_pose(cx, &empty_receipt).is_err();
    let retention = prove_retention_reaper(&receipt);
    ConsentProof {
        denied_without_kernel_grant,
        denied_without_visible_grant,
        sample_evicted: retention.sample_evicted,
        content_evicted: retention.content_evicted,
    }
}

/// Result of the deterministic retention sweep.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetentionProof {
    /// Whether the sample is absent after the sweep.
    pub sample_evicted: bool,
    /// Whether its referenced content is absent after the sweep.
    pub content_evicted: bool,
}

/// Runs the retention reaper over a modeled sample store.
pub fn prove_retention_reaper(receipt: &ConsentReceipt) -> RetentionProof {
    let sample_key = StoreKey::named("pose-sample");
    let content_key = StoreKey::named("pose-content");
    let mut store = DeviceSampleStore::new();
    store.insert_content(content_key.clone(), build::text("pose payload"));
    store.insert_sample(StoredSample::new(
        sample_key.clone(),
        receipt.seq,
        0,
        vec![content_key.clone()],
        build::map(vec![("pose", build::uint(1))]),
    ));
    let _evicted = RetentionReaper::new().sweep(
        &mut store,
        std::slice::from_ref(receipt),
        FrameClock::new(
            receipt.retain_ms.saturating_add(2),
            RateClass::safe_default(),
        ),
    );
    RetentionProof {
        sample_evicted: !store.contains_sample(&sample_key),
        content_evicted: !store.contains_content(&content_key),
    }
}
