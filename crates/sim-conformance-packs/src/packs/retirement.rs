// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Permanent retired-runtime checks.

use crate::{CheckObservation, PackRequest, PackVerdict, harness};

#[path = "retirement_closures.rs"]
mod closures;

pub use closures::{ObservedClosureFinal, bind_closure_final, observe_closure_final};

/// Checks the exact requested retirement scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-v3", request, |request| match request.scope {
        "retirement/bootstrap" => bootstrap(request),
        "completion/closure-final" => closures::observations(request),
        _ => unreachable!("availability was checked before dispatch"),
    })
}

fn bootstrap(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, crate::PackFailure> {
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
    .collect()
}
