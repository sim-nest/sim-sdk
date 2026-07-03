use sim_kernel::shape_report::satisfies_shape_predicate;
use sim_kernel::{ClaimPattern, Cx, Ref, Result, Symbol, Value, value_from_ref};

pub(super) fn claim_scalar(cx: &mut Cx, subject: &Ref, predicate: Symbol) -> Result<Option<Value>> {
    let claims = cx.query_facts(ClaimPattern {
        subject: Some(subject.clone()),
        predicate: Some(predicate),
        object: None,
        include_revoked: false,
    })?;
    claims
        .first()
        .map(|claim| value_from_claim_object(cx, &claim.object))
        .transpose()
}

pub(super) fn shape_provenance(cx: &mut Cx, subject: &Ref) -> Result<Option<Value>> {
    let claims = cx.query_facts(ClaimPattern {
        subject: Some(subject.clone()),
        predicate: Some(satisfies_shape_predicate()),
        object: None,
        include_revoked: false,
    })?;
    let mut values = Vec::new();
    for evidence in claims.into_iter().flat_map(|claim| claim.evidence) {
        values.push(value_from_claim_object(cx, &evidence)?);
    }
    if values.is_empty() {
        Ok(None)
    } else {
        cx.factory().list(values).map(Some)
    }
}

fn value_from_claim_object(cx: &mut Cx, object: &Ref) -> Result<Value> {
    match object {
        Ref::Symbol(symbol) => cx.factory().symbol(symbol.clone()),
        Ref::Content(_) | Ref::Handle(_) => value_from_ref(cx, object),
        Ref::Coord(_) => cx
            .factory()
            .expr(sim_kernel::Expr::String(format!("{object:?}"))),
    }
}
