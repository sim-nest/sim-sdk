//! Durable-operation conformance pack.

use crate::{CheckObservation, PackRequest, PackVerdict, harness};

/// Checks durable log evidence while later reconciliation scopes stay unavailable.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-op", request, |request| match request.scope {
        "operation/log" => operation_log(request),
        _ => unreachable!("availability was checked before dispatch"),
    })
}

fn operation_log(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    let mut observations = Vec::new();
    for key in [
        "operation.intent-is-canonical-datum",
        "operation.operation-id-binds-target",
        "operation.operation-id-binds-intended-result",
        "operation.operation-id-binds-replay-policy",
        "operation.grant-recorded-separately",
        "operation.attempt-recorded-separately",
        "operation.lease-excluded-from-operation-id",
        "operation.contradictory-intent-refused",
        "operation.intent-before-dispatch",
        "operation.dispatch-before-performance",
        "operation.raw-receipt-after-performance",
        "operation.all-crash-cuts-reconstruct",
        "operation.recorded-dispatch-not-repeated",
        "operation.external-counter-survives-replay",
        "operation.missing-acknowledgement-not-failure",
        "operation.fake-performers-only",
        "operation.real-process-network-disabled",
        "operation.reconciliation-unimplemented",
    ] {
        observations.push(harness::expect_true(request, key)?);
    }
    observations.push(harness::expect_eq(request, "operation.crash-cuts", "6")?);
    observations.push(harness::expect_eq(
        request,
        "operation.durable-states",
        "3",
    )?);
    Ok(observations)
}
