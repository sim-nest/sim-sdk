use sim_kernel::{Error, Expr, Result, Symbol};
use sim_lib_intent::{Origin, intent};
use sim_lib_view::SurfaceCaps;
use sim_lib_view_device::{ConsentReceipt, DeviceCapability, EdgeId};
use sim_lib_web_bridge::GlassesCoUseSession;
use sim_value::{access, build};

/// Result of the modeled Viture and Halo co-use proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoUseProof {
    /// Number of attached surface peers.
    pub peers: usize,
    /// Number of projections receiving the Halo-driven update.
    pub broadcasts: usize,
    /// Number of canonical edit-ledger rows appended.
    pub ledger_rows: usize,
    /// Whether the Viture review panel records the Halo acknowledgement.
    pub viture_panel_acked: bool,
}

impl CoUseProof {
    /// Encodes the proof as expression data for cookbook recipes.
    pub fn to_expr(&self) -> Expr {
        build::map(vec![
            ("kind", build::qsym("glasses/sdk", "co-use-proof")),
            ("peers", build::uint(self.peers as u64)),
            ("broadcasts", build::uint(self.broadcasts as u64)),
            ("ledger-rows", build::uint(self.ledger_rows as u64)),
            ("viture-panel-acked", Expr::Bool(self.viture_panel_acked)),
        ])
    }
}

/// Attaches modeled Viture and Halo peers and drives the main panel from Halo.
pub fn prove_co_use() -> Result<CoUseProof> {
    let edge = EdgeId::named("sdk-glasses-co-use");
    let receipt = ConsentReceipt::new(
        vec![
            DeviceCapability::Pose.grant_symbol(),
            DeviceCapability::Mic.grant_symbol(),
        ],
        60_000,
        Vec::new(),
        edge.clone(),
        7,
    );
    let mut session =
        GlassesCoUseSession::new(edge, receipt, build::keyword("workspace"), workspace())?;
    session.attach_viture(
        SurfaceCaps::from_preset("glasses-luma-ultra", "sdk.co-use.viture")
            .ok_or_else(|| Error::HostError("Viture surface preset missing".to_owned()))?,
    )?;
    session.attach_halo(
        SurfaceCaps::from_preset("glasses-hud", "sdk.co-use.halo")
            .ok_or_else(|| Error::HostError("Halo surface preset missing".to_owned()))?,
    )?;
    let tap = intent(
        "invoke",
        Origin::human(8),
        vec![
            ("target", build::sym("workspace")),
            (
                "op",
                Expr::Symbol(Symbol::qualified("glasses/input", "double-tap")),
            ),
            ("args", build::list(Vec::new())),
        ],
    );
    let broadcasts =
        session.acknowledge_review_from_halo(&tap, Symbol::qualified("bridge", "packet-review"))?;
    let review = access::field(session.workspace()?, "review");

    Ok(CoUseProof {
        peers: session.live_bindings().len(),
        broadcasts: broadcasts.len(),
        ledger_rows: session.ledger().len(),
        viture_panel_acked: review
            .and_then(|review| access::field_sym(review, "status"))
            .is_some_and(|status| status.name.as_ref() == "acked"),
    })
}

fn workspace() -> Expr {
    build::map(vec![
        ("title", build::text("Bridge review")),
        (
            "review",
            build::map(vec![
                (
                    "mission",
                    Expr::Symbol(Symbol::qualified("bridge", "packet-review")),
                ),
                ("status", build::sym("pending")),
            ]),
        ),
    ])
}
