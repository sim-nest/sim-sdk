use sim_codec_bridge::{
    BridgeBook, BridgeFramePayload, BridgeHeader, BridgePacket, BridgePart, BridgeProvenance,
    stamp_packet_cid, warrant_for_packet,
};
use sim_kernel::{Error, Expr, Result, Symbol};
use sim_lib_intent::{Origin, intent_kind_of};
use sim_lib_scene::GlanceCard;
use sim_lib_view::SurfaceCaps;
use sim_lib_view_bridge::{
    BridgeGlassesReviewInput, halo_warrant_glance_pager, warrant_review_intent_from_glasses_input,
};
use sim_lib_view_device::DeviceSurfaceCapsExt;
use sim_value::build::{self, entry, qsym};

/// Result of the modeled Halo BRIDGE warrant-review proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewInSpaceProof {
    /// Whether the pending packet becomes a budget-bypassing Halo pager.
    pub warrant_pager: bool,
    /// Whether a modeled Halo double tap emits `intent/approve`.
    pub approved: bool,
    /// Whether the approval remains bound to the stamped packet id.
    pub packet_bound: bool,
}

impl ReviewInSpaceProof {
    /// Encodes the proof as expression data for cookbook recipes.
    pub fn to_expr(&self) -> Expr {
        build::map(vec![
            ("kind", build::qsym("glasses/sdk", "review-in-space-proof")),
            ("warrant-pager", Expr::Bool(self.warrant_pager)),
            ("approved", Expr::Bool(self.approved)),
            ("packet-bound", Expr::Bool(self.packet_bound)),
        ])
    }
}

/// Projects a pending BRIDGE packet and approves it with a modeled Halo double tap.
pub fn prove_review_in_space() -> Result<ReviewInSpaceProof> {
    let packet = packet_with_warrant()?;
    let profile = SurfaceCaps::from_preset("glasses-hud", "sdk.review")
        .ok_or_else(|| Error::HostError("Halo surface preset missing".to_owned()))?
        .device_profile();
    let pager = halo_warrant_glance_pager(&packet, &profile)?;
    let card = GlanceCard::from_scene(&pager)?;
    let approval = warrant_review_intent_from_glasses_input(
        &packet,
        BridgeGlassesReviewInput::HaloDoubleTap,
        Origin::human(9),
    )?;
    let packet_cid = packet.header.cid.as_deref().unwrap_or_default();

    Ok(ReviewInSpaceProof {
        warrant_pager: card.bypass_budget,
        approved: intent_kind_of(&approval).is_some_and(|kind| kind.name.as_ref() == "approve"),
        packet_bound: sim_lib_intent::field(&approval, "packet-cid")
            == Some(&Expr::String(packet_cid.to_owned())),
    })
}

fn packet_with_warrant() -> Result<BridgePacket> {
    let mut packet = BridgePacket {
        header: BridgeHeader {
            cid: None,
            move_kind: Symbol::new("reply"),
            from: "model:drafter".to_owned(),
            to: vec!["human:reviewer".to_owned()],
            role: Symbol::new("implementer"),
            parents: vec!["core/sha256-bridge-v1:root".to_owned()],
            task: Symbol::new("T1"),
            output: Symbol::new("O1"),
            ceiling: Vec::new(),
            context: Vec::new(),
            provenance: BridgeProvenance::default(),
        },
        body: vec![
            BridgePart {
                id: Symbol::new("T1"),
                kind: Symbol::qualified("bridge", "Frame"),
                payload: BridgeFramePayload::new(Symbol::qualified("bridge", "answer")).to_expr(),
            },
            BridgePart {
                id: Symbol::new("O1"),
                kind: Symbol::qualified("bridge", "Return"),
                payload: Expr::Map(vec![
                    entry("codec", qsym("codec", "bridge")),
                    entry("shape", qsym("core", "Map")),
                ]),
            },
        ],
        warrant: None,
    };
    packet.warrant = Some(warrant_for_packet(&BridgeBook::standard(), &packet)?);
    stamp_packet_cid(&packet)
}
