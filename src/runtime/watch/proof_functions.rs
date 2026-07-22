use sim_kernel::{Args, Callable, Cx, Error, Expr, Object, ObjectCompat, Result, Symbol, Value};

use super::{prove_dual_quorum, prove_glance_pager, prove_hold_last, prove_privacy_reaper};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ProofKind {
    GlancePager,
    HoldLast,
    PrivacyReaper,
    DualQuorum,
}

impl ProofKind {
    pub(super) const ALL: [Self; 4] = [
        Self::GlancePager,
        Self::HoldLast,
        Self::PrivacyReaper,
        Self::DualQuorum,
    ];

    fn token(self) -> &'static str {
        match self {
            Self::GlancePager => "glance-pager",
            Self::HoldLast => "hold-last",
            Self::PrivacyReaper => "privacy-reaper",
            Self::DualQuorum => "dual-quorum",
        }
    }
}

pub(super) fn proof_function_symbol(kind: ProofKind) -> Symbol {
    Symbol::qualified("watch/sdk", kind.token())
}

pub(super) struct WatchProofFunction {
    pub(super) kind: ProofKind,
}

impl Callable for WatchProofFunction {
    fn call(&self, cx: &mut Cx, args: Args) -> Result<Value> {
        let symbol = proof_function_symbol(self.kind);
        accept_proof_args(cx, &args, &symbol)?;
        let expr = match self.kind {
            ProofKind::GlancePager => prove_glance_pager()?.to_expr(),
            ProofKind::HoldLast => prove_hold_last()?.to_expr(),
            ProofKind::PrivacyReaper => prove_privacy_reaper()?.to_expr(),
            ProofKind::DualQuorum => prove_dual_quorum()?.to_expr(),
        };
        cx.factory().expr(expr)
    }
}

fn accept_proof_args(cx: &mut Cx, args: &Args, symbol: &Symbol) -> Result<()> {
    match args.values() {
        [] => Ok(()),
        [marker] if matches!(marker.object().as_expr(cx)?, Expr::Nil) => Ok(()),
        _ => Err(Error::Eval(format!(
            "{symbol} expects no arguments or one nil call marker"
        ))),
    }
}

impl Object for WatchProofFunction {
    fn display(&self, _cx: &mut Cx) -> Result<String> {
        Ok(proof_function_symbol(self.kind).to_string())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ObjectCompat for WatchProofFunction {
    fn as_callable(&self) -> Option<&dyn Callable> {
        Some(self)
    }
}
