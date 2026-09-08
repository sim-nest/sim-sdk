//! Evidence conformance pack.

use crate::{CheckObservation, PackRequest, PackVerdict, harness};

/// Checks the exact funded evidence scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-evidence", request, |request| {
        match request.scope {
            "evidence/projection" => projection(request),
            _ => unreachable!("availability was checked before dispatch"),
        }
    })
}

fn projection(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    [
        "evidence.projection-domain-tag-unique",
        "evidence.provider-package-version-bound",
        "evidence.provider-code-identity-bound",
        "evidence.projector-policy-bound",
        "evidence.checked-config-bound",
        "evidence.consumed-semantic-inputs-bound",
        "evidence.diagnostic-envelope-excluded",
        "evidence.equal-input-cross-host-stable",
        "evidence.semantic-delta-changes-digest",
        "evidence.affected-closure-independently-expected",
        "evidence.explanation-path-exact",
        "evidence.qualification-revocation-scoped",
    ]
    .into_iter()
    .map(|key| harness::expect_true(request, key))
    .collect()
}
