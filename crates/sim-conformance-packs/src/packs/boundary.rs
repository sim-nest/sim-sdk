//! Architecture boundary inventory checks.

use crate::{CheckObservation, PackRequest, PackVerdict, harness};

/// Checks the exact requested boundary scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-boundary", request, |request| {
        match request.scope {
            "boundary/inventory" => inventory(request),
            "boundary/identity-closure" => identity_closure(request),
            _ => unreachable!("availability was checked before dispatch"),
        }
    })
}

fn inventory(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    [
        "boundary.kernel-unchanged",
        "boundary.behavior-remains-loaded",
        "boundary.singular-graphs",
        "boundary.singular-stores",
        "boundary.fake-effects-only",
        "boundary.ambient-observation-refused",
        "boundary.inventory-complete",
    ]
    .into_iter()
    .map(|key| harness::expect_true(request, key))
    .collect()
}

fn identity_closure(
    request: &PackRequest<'_>,
) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    [
        "boundary.kernel-unchanged",
        "boundary.kernel-policy-absent",
        "boundary.behavior-remains-loaded",
        "boundary.semantic-byte-types-distinct",
        "boundary.loader-membrane-retained",
        "boundary.tooling-delegates-codec",
        "boundary.compatibility-reader-read-only",
        "boundary.ambient-identity-authority-refused",
        "boundary.cache-reuse-requires-exact-semantics",
        "boundary.identity-closure-complete",
    ]
    .into_iter()
    .map(|key| harness::expect_true(request, key))
    .collect()
}
