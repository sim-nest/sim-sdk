//! Permanent retired-runtime checks.

use crate::{CheckObservation, PackRequest, PackVerdict, harness};

/// Checks the exact requested retirement scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-v3", request, |request| {
        [
            "retirement.guard-passed",
            "retirement.production-entrypoints-fail-closed",
            "retirement.production-readers-absent",
            "retirement.production-renderers-absent",
            "retirement.production-shadow-state-absent",
            "retirement.inventory-complete",
        ]
        .into_iter()
        .map(|key| harness::expect_true(request, key))
        .collect::<Result<Vec<CheckObservation>, _>>()
    })
}
