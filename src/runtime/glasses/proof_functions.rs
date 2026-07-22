use sim_kernel::{Args, Callable, Cx, Error, Expr, Object, ObjectCompat, Result, Symbol, Value};

use super::{
    prove_co_use, prove_halo_glance, prove_review_in_space, prove_two_rate, prove_voice_site,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ProofKind {
    TwoRate,
    HaloGlancePager,
    VoiceSite,
    CoUse,
    ReviewInSpace,
}

impl ProofKind {
    pub(super) const ALL: [Self; 5] = [
        Self::TwoRate,
        Self::HaloGlancePager,
        Self::VoiceSite,
        Self::CoUse,
        Self::ReviewInSpace,
    ];

    fn token(self) -> &'static str {
        match self {
            Self::TwoRate => "viture-two-rate",
            Self::HaloGlancePager => "halo-glance-pager",
            Self::VoiceSite => "voice-site",
            Self::CoUse => "co-use",
            Self::ReviewInSpace => "review-in-space",
        }
    }
}

pub(super) fn proof_function_symbol(kind: ProofKind) -> Symbol {
    Symbol::qualified("glasses/sdk", kind.token())
}

pub(super) struct GlassesProofFunction {
    pub(super) kind: ProofKind,
}

impl Callable for GlassesProofFunction {
    fn call(&self, cx: &mut Cx, args: Args) -> Result<Value> {
        let symbol = proof_function_symbol(self.kind);
        accept_proof_args(cx, &args, &symbol)?;
        let expr = match self.kind {
            ProofKind::TwoRate => prove_two_rate()?.to_expr(),
            ProofKind::HaloGlancePager => prove_halo_glance()?.to_expr(),
            ProofKind::VoiceSite => prove_voice_site(cx)?.to_expr(),
            ProofKind::CoUse => prove_co_use()?.to_expr(),
            ProofKind::ReviewInSpace => prove_review_in_space()?.to_expr(),
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

impl Object for GlassesProofFunction {
    fn display(&self, _cx: &mut Cx) -> Result<String> {
        Ok(proof_function_symbol(self.kind).to_string())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ObjectCompat for GlassesProofFunction {
    fn as_callable(&self) -> Option<&dyn Callable> {
        Some(self)
    }
}
