use sim_kernel::{Error, EventKind, EventLedger, Expr, Ref, Result, Symbol};
use sim_value::build;

use super::{reference_edge_id, reference_glance_profile, reference_pose_receipt};

/// Result of the route-swap survival proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteSwapProof {
    /// Whether the session id remains unchanged.
    pub same_session_id: bool,
    /// Whether the ledger reference remains unchanged.
    pub same_ledger: bool,
    /// Whether bound visible consent remains attached.
    pub consent_survived: bool,
    /// Whether the event ledger records the rebind.
    pub events_advanced: bool,
    /// Whether the device peer surface keeps the reference namespace.
    pub peer_surface_registered: bool,
}

impl RouteSwapProof {
    /// Encodes the proof as expression data for cookbook recipes.
    pub fn to_expr(&self) -> Expr {
        build::map(vec![
            ("kind", build::qsym("device/reference", "route-swap-proof")),
            ("same-session-id", Expr::Bool(self.same_session_id)),
            ("same-ledger", Expr::Bool(self.same_ledger)),
            ("consent-survived", Expr::Bool(self.consent_survived)),
            ("events-advanced", Expr::Bool(self.events_advanced)),
            (
                "peer-surface-registered",
                Expr::Bool(self.peer_surface_registered),
            ),
        ])
    }
}

/// Runs the route-swap survival proof.
pub fn prove_route_swap() -> Result<RouteSwapProof> {
    let edge = reference_edge_id();
    let id = edge.as_symbol().clone();
    let receipt = reference_pose_receipt(9, 100);
    let consent_expr = receipt.to_expr();
    let mut session = ReferenceRouteSession::new(id.clone(), reference_link_symbol("direct"))?;
    session.bind_consent(consent_expr.clone())?;
    let ledger = session.ledger.clone();
    let events_before = session.events.len_for_run(&ledger);

    let mut hub = sim_lib_web_bridge::SurfaceHub::new();
    let peer =
        sim_lib_web_bridge::register_device_peer(&mut hub, &edge, &reference_glance_profile());

    session.rebind(reference_link_symbol("relay"))?;

    Ok(RouteSwapProof {
        same_session_id: session.id == id,
        same_ledger: session.ledger == ledger,
        consent_survived: session.consent.as_ref() == Some(&consent_expr),
        events_advanced: session.events.len_for_run(&ledger) == events_before + 1,
        peer_surface_registered: peer.namespace.as_deref() == Some("device/peer"),
    })
}

#[derive(Clone, Debug)]
struct ReferenceRouteSession {
    id: Symbol,
    link: Symbol,
    ledger: Ref,
    events: EventLedger,
    consent: Option<Expr>,
}

impl ReferenceRouteSession {
    fn new(id: Symbol, link: Symbol) -> Result<Self> {
        let ledger = Ref::Symbol(id.clone());
        let mut session = Self {
            id,
            link,
            ledger,
            events: EventLedger::new(),
            consent: None,
        };
        session.record_event("open")?;
        Ok(session)
    }

    fn bind_consent(&mut self, consent: Expr) -> Result<()> {
        match sim_value::access::field_sym(&consent, "session") {
            Some(session) if session == self.id => {}
            Some(session) => {
                return Err(Error::HostError(format!(
                    "consent session '{session}' does not match device edge session '{}'",
                    self.id
                )));
            }
            None => {
                return Err(Error::HostError(
                    "device edge consent is missing session".to_owned(),
                ));
            }
        }
        self.consent = Some(consent);
        self.record_event("consent-bound")
    }

    fn rebind(&mut self, link: Symbol) -> Result<()> {
        self.link = link;
        self.record_event("rebind")
    }

    fn record_event(&mut self, name: &str) -> Result<()> {
        self.events.push(
            self.ledger.clone(),
            EventKind::Trace(Ref::Symbol(Symbol::qualified("device/edge", name))),
        )?;
        Ok(())
    }
}

fn reference_link_symbol(name: &str) -> Symbol {
    Symbol::qualified("device/link", name.to_owned())
}
