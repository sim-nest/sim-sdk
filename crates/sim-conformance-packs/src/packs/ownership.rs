//! Activation ownership checks.

use crate::{CheckObservation, PackRequest, PackVerdict, harness};

/// Checks the exact requested ownership scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-own", request, |request| match request.scope {
        "ownership/activation" => activation(request),
        "ownership/dependencies" => dependencies(request),
        "ownership/bootstrap" => bootstrap(request),
        "ownership/produced" => produced(request),
        _ => unreachable!("availability was checked before dispatch"),
    })
}

fn produced(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    let mut observations = harness::expect_same_u64(
        request,
        "ownership.produced-target-surfaces",
        "ownership.qualified-target-surfaces",
    )?;
    for key in [
        "ownership.target-was-planned-before-construction",
        "ownership.packet-dependencies-released",
        "ownership.unimplemented-external-dependency-refused",
        "ownership.target-source-qualified",
        "ownership.target-api-qualified",
        "ownership.target-release-qualified",
        "ownership.owner-validation-and-docs-exact",
        "ownership.index-route-present",
        "ownership.crate-admission-complete",
        "ownership.support-graph-acyclic",
        "ownership.overlaps-zero",
        "ownership.kernel-head-unchanged",
    ] {
        observations.push(harness::expect_true(request, key)?);
    }
    Ok(observations)
}

fn bootstrap(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    let mut observations = harness::expect_same_u64(
        request,
        "ownership.bootstrap-reached-surfaces",
        "ownership.bootstrap-qualified-surfaces",
    )?;
    for key in [
        "ownership.bootstrap-closure-complete",
        "ownership.bootstrap-transitive-dependencies-released",
        "ownership.bootstrap-produced-source-qualified",
        "ownership.bootstrap-produced-api-qualified",
        "ownership.bootstrap-produced-release-qualified",
        "ownership.bootstrap-binding-receipts-verified",
        "ownership.bootstrap-dependency-use-sets-qualified",
        "ownership.bootstrap-later-surfaces-planned",
        "ownership.bootstrap-roadmap-final-unclaimed",
        "ownership.bootstrap-handoff-boundary-recorded",
        "ownership.bootstrap-support-graph-acyclic",
        "ownership.bootstrap-overlaps-zero",
    ] {
        observations.push(harness::expect_true(request, key)?);
    }
    Ok(observations)
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
