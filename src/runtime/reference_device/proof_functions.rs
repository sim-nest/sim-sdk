use sim_kernel::{Args, Callable, Cx, Error, Expr, Object, ObjectCompat, Result, Symbol, Value};

use super::{prove_consent_without_kernel_grant, prove_route_swap, prove_two_rate};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ProofKind {
    TwoRate,
    Consent,
    RouteSwap,
}

impl ProofKind {
    pub(super) const ALL: [Self; 3] = [Self::TwoRate, Self::Consent, Self::RouteSwap];

    fn token(self) -> &'static str {
        match self {
            Self::TwoRate => "two-rate",
            Self::Consent => "consent",
            Self::RouteSwap => "route-swap",
        }
    }
}

pub(super) fn proof_function_symbol(kind: ProofKind) -> Symbol {
    Symbol::qualified("device/reference", kind.token())
}

pub(super) struct ReferenceProofFunction {
    pub(super) kind: ProofKind,
}

impl Callable for ReferenceProofFunction {
    fn call(&self, cx: &mut Cx, args: Args) -> Result<Value> {
        let symbol = proof_function_symbol(self.kind);
        accept_proof_args(cx, &args, &symbol)?;
        let expr = match self.kind {
            ProofKind::TwoRate => prove_two_rate()?.to_expr(),
            ProofKind::Consent => prove_consent_without_kernel_grant(cx).to_expr(),
            ProofKind::RouteSwap => prove_route_swap()?.to_expr(),
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

impl Object for ReferenceProofFunction {
    fn display(&self, _cx: &mut Cx) -> Result<String> {
        Ok(proof_function_symbol(self.kind).to_string())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ObjectCompat for ReferenceProofFunction {
    fn as_callable(&self) -> Option<&dyn Callable> {
        Some(self)
    }
}
