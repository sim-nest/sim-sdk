// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Outward-delivery conformance pack.

use crate::{CheckObservation, PackFailure, PackRequest, PackVerdict, harness};

/// Scenarios every funded delivery scope must execute.
pub const SCENARIOS: [&str; 7] = [
    "delivery.already-true",
    "delivery.stale-observer-refused",
    "delivery.lost-acknowledgement-refused",
    "delivery.divergent-tag-refused",
    "delivery.registry-lag-refused",
    "delivery.uncertain-exact-once-refused",
    "delivery.completed-checkpoint-omission-refused",
];

/// Checks one funded delivery scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-deliver", request, |request| {
        match request.scope {
            "delivery/observation"
            | "delivery/plan"
            | "delivery/adapters"
            | "delivery/real-canary"
            | "delivery/composed" => scenarios(request),
            _ => unreachable!("availability was checked before dispatch"),
        }
    })
}

fn scenarios(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, PackFailure> {
    SCENARIOS
        .into_iter()
        .map(|key| harness::expect_true(request, key))
        .collect()
}
