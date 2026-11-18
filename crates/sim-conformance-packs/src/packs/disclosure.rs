// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Disclosure-policy conformance pack.

use crate::{CheckObservation, PackRequest, PackVerdict, harness};

/// Checks the exact funded disclosure scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-disclose", request, |request| {
        match request.scope {
            "disclosure/projection" => projection(request),
            "disclosure/composed" => composed(request),
            _ => unreachable!("availability was checked before dispatch"),
        }
    })
}

const COMPOSED_FACT: &str = "disclosure.composed-partial-refused";

fn composed(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    let mut observations = projection(request)?;
    observations.push(harness::expect_true(request, COMPOSED_FACT)?);
    Ok(observations)
}

fn projection(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    [
        "disclosure.audit-rules-exact",
        "disclosure.policy-value-canonical",
        "disclosure.provider-kind-open",
        "disclosure.policy-is-supplied-observation",
        "disclosure.nonconsumer-excludes-policy-digest",
        "disclosure.consumer-binds-policy-digest",
        "disclosure.allowlist-change-invalidates-exact-consumers",
        "disclosure.unrelated-conclusions-reused",
        "disclosure.disagreement-remains-unknown",
        "disclosure.hostile-public-sink-refused",
    ]
    .into_iter()
    .map(|key| harness::expect_true(request, key))
    .collect()
}
