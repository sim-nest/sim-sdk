//! Architecture boundary inventory checks.

use crate::{CheckObservation, PackRequest, PackVerdict, harness};

/// Checks the exact requested boundary scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-boundary", request, |request| {
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
        .collect::<Result<Vec<CheckObservation>, _>>()
    })
}
