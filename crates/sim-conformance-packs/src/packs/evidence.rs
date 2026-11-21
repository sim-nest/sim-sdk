// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Evidence conformance pack.

use crate::{CheckObservation, PackRequest, PackVerdict, harness};

#[path = "evidence_domains.rs"]
mod domains;
#[path = "evidence_grades.rs"]
mod grades;

pub use domains::{ObservedDomainsFinal, bind_domains_final, observe_domains_final};
pub use grades::{ObservedGradesFinal, bind_grades_final, observe_grades_final};

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
            "evidence/composed" => composed(request),
            "completion/grades-final" => grades::observations(request),
            "completion/domains-final" => domains::observations(request),
            _ => unreachable!("availability was checked before dispatch"),
        }
    })
}

/// Projection observations required on their own and inside the composed scope.
pub const PROJECTION_FACTS: [&str; 12] = [
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
];

const COMPOSED_FACT: &str = "evidence.composed-partial-refused";

fn composed(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    let mut observations = projection(request)?;
    observations.extend(predicted_closure(request)?);
    observations.push(harness::expect_true(request, COMPOSED_FACT)?);
    Ok(observations)
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
    PROJECTION_FACTS
        .into_iter()
        .map(|key| harness::expect_true(request, key))
        .collect()
}
