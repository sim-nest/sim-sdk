//! Activation ownership checks.

use crate::{CheckObservation, PackRequest, PackVerdict, harness};

/// Checks the exact requested ownership scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-own", request, |request| match request.scope {
        "ownership/activation" => activation(request),
        "ownership/dependencies" => dependencies(request),
        _ => unreachable!("availability was checked before dispatch"),
    })
}

fn activation(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    let mut observations = Vec::<CheckObservation>::new();
    for (actual, expected) in [
        (
            "ownership.owner-bindings",
            "ownership.expected-owner-bindings",
        ),
        (
            "ownership.checker-bindings",
            "ownership.expected-checker-bindings",
        ),
        ("ownership.phase-gates", "ownership.expected-phase-gates"),
        (
            "ownership.dependency-use-sets",
            "ownership.expected-dependency-use-sets",
        ),
    ] {
        observations.extend(harness::expect_same_u64(request, actual, expected)?);
    }
    for key in [
        "ownership.unresolved-rows-zero",
        "ownership.bindings-contain-no-invocations",
        "ownership.planned-target-admitted",
        "ownership.unavailable-dependency-refused",
        "ownership.activation-as-final-refused",
        "ownership.support-graph-acyclic",
    ] {
        observations.push(harness::expect_true(request, key)?);
    }
    Ok(observations)
}

fn dependencies(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    let mut observations = harness::expect_same_u64(
        request,
        "ownership.reached-owner-surfaces",
        "ownership.qualified-owner-surfaces",
    )?;
    for key in [
        "ownership.actual-closure-complete",
        "ownership.loader-owner-qualified",
        "ownership.tooling-owner-qualified",
        "ownership.codec-owns-vault-identity",
        "ownership.dependency-use-sets-qualified",
        "ownership.unresolved-rows-zero",
        "ownership.singular-owner-per-construction",
        "ownership.support-graph-acyclic",
        "ownership.overlaps-zero",
        "ownership.unavailable-dependency-refused",
    ] {
        observations.push(harness::expect_true(request, key)?);
    }
    Ok(observations)
}
