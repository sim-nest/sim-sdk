//! Evidence conformance pack.

use crate::{CheckObservation, PackRequest, PackVerdict, harness};

/// Exact pre-materialized facts required by the stage-one closure assay checker.
pub const PREDICTED_CLOSURE_FACTS: [&str; 21] = [
    "evidence.oracle-frozen-before-implementation",
    "evidence.predicted-affected-sets-exact",
    "evidence.declared-denominators-explicit",
    "evidence.predicted-decisions-measured",
    "evidence.changed-facts-measured",
    "evidence.explanations-measured",
    "evidence.journal-records-measured",
    "evidence.planned-starts-measured",
    "evidence.d1-all-unaffected-no-work",
    "evidence.d2-unaffected-at-least-95-percent",
    "evidence.d3-exact-public-consumer-cone",
    "evidence.d4-unaffected-at-least-70-percent",
    "evidence.d5-matching-proof-checker-keys-only",
    "evidence.d6-no-semantic-work-or-carried-state",
    "evidence.every-declaration-mutant-killed",
    "evidence.every-edge-mutant-killed",
    "evidence.proof-not-executed",
    "evidence.receipt-reuse-not-claimed",
    "evidence.projection-repair-set-bounded",
    "evidence.third-unchanged-repair-needs-direction",
    "evidence.failed-assay-yields-no-qualification",
];

/// Checks the exact funded evidence scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-evidence", request, |request| {
        match request.scope {
            "evidence/projection" => projection(request),
            "evidence/predicted-closure" => predicted_closure(request),
            _ => unreachable!("availability was checked before dispatch"),
        }
    })
}

fn predicted_closure(
    request: &PackRequest<'_>,
) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    PREDICTED_CLOSURE_FACTS
        .into_iter()
        .map(|key| harness::expect_true(request, key))
        .collect()
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
