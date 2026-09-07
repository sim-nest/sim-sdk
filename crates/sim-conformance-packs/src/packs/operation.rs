//! Durable-operation conformance pack.

use crate::{CheckObservation, PackRequest, PackVerdict, harness};

/// Checks the requested durable operation scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-op", request, |request| match request.scope {
        "operation/log" => operation_log(request),
        "operation/reconcile" => operation_reconcile(request),
        "operation/local" => operation_local(request),
        _ => unreachable!("availability was checked before dispatch"),
    })
}

fn operation_reconcile(
    request: &PackRequest<'_>,
) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    let mut observations = Vec::new();
    for key in [
        "operation.lease-is-bounded",
        "operation.lease-binds-holder-and-writer-fence",
        "operation.observe-before-resume",
        "operation.observer-independent-of-performer",
        "operation.already-true-reduced",
        "operation.verified-reduced",
        "operation.diverged-reduced",
        "operation.uncertain-reduced",
        "operation.idempotent-retry-requires-expiry-and-absence",
        "operation.exactly-once-unknown-ack-not-repeated",
        "operation.lost-receipt-reconciles-or-uncertain",
        "operation.observer-disagreement-uncertain",
        "operation.performer-identity-stable-on-recovery",
        "operation.all-crash-cuts-reconstruct",
    ] {
        observations.push(harness::expect_true(request, key)?);
    }
    observations.push(harness::expect_eq(
        request,
        "operation.crash-boundaries",
        "8",
    )?);
    Ok(observations)
}

fn operation_local(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    let mut observations = Vec::new();
    for key in [
        "operation.local-port-is-portable",
        "operation.local-command-is-installed-and-allowlisted",
        "operation.local-command-id-binds-complete-spec",
        "operation.local-manifest-script-bytes-exact",
        "operation.local-model-interpolation-refused",
        "operation.local-environment-sealed",
        "operation.local-writable-roots-confined",
        "operation.local-network-absent",
        "operation.local-network-grant-separate",
        "operation.local-release-credentials-absent",
        "operation.local-formatter-mutation-observed",
        "operation.local-test-success-observed",
        "operation.local-test-failure-observed",
        "operation.local-validation-command-exact",
        "operation.local-docs-command-exact",
        "operation.local-timeout-terminates-group",
        "operation.local-cancellation-terminates-group",
        "operation.local-signal-escalation-bounded",
        "operation.local-descendants-zero",
        "operation.local-scratch-zero",
        "operation.local-independent-postcondition",
        "operation.local-later-owner-gates-reachable",
    ] {
        observations.push(harness::expect_true(request, key)?);
    }
    Ok(observations)
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
