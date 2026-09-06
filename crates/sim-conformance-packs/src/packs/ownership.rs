//! Activation ownership checks.

use crate::{CheckObservation, PackRequest, PackVerdict, harness};

/// Checks the exact requested ownership scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-own", request, |request| {
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
    })
}
