// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Versioned-ensemble authoring conformance pack.

use crate::{CheckObservation, PackFailure, PackRequest, PackVerdict, harness};

/// Scenarios every funded authoring scope must execute.
pub const SCENARIOS: [&str; 6] = [
    "authoring.family-self-contained",
    "authoring.consultation-not-a-requirement",
    "authoring.forbidden-sibling-requirement-refused",
    "authoring.superseded-not-selected",
    "authoring.human-selection-exact",
    "authoring.exact-source-roundtrip",
];

/// Checks one funded authoring scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-author", request, |request| match request.scope {
        "authoring/family" | "authoring/composed" => scenarios(request),
        _ => unreachable!("availability was checked before dispatch"),
    })
}

fn scenarios(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, PackFailure> {
    SCENARIOS
        .into_iter()
        .map(|key| harness::expect_true(request, key))
        .collect()
}
