//! Exact release-closure evidence checks.

use crate::{CheckObservation, PackRequest, PackVerdict, harness};

/// Checks the exact requested release scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-release", request, |request| {
        [
            "release.owner-validation-passed",
            "release.owner-docs-passed",
            "release.packages-assembled",
            "release.audit-passed",
            "release.authorship-passed",
            "release.tags-exact",
            "release.pins-exact",
            "release.mirrors-current",
            "release.generated-converged",
            "release.boot-smoke-passed",
            "release.publication-confirmed",
            "release.standalone-ci-green",
        ]
        .into_iter()
        .map(|key| harness::expect_true(request, key))
        .collect::<Result<Vec<CheckObservation>, _>>()
    })
}
