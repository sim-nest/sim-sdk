// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Exact release-closure evidence checks.

#[path = "release_closure.rs"]
mod closure;

use crate::{CheckObservation, PackRequest, PackVerdict, harness};

/// Checks the exact requested release scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-release", request, |request| {
        if request.scope == "release/nv12-27" {
            return closure::check_nv12_27(request);
        }
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
