// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Exact release-closure evidence checks.

#[path = "release_closure.rs"]
mod closure;
#[path = "release_receipt.rs"]
mod receipt;
#[path = "release_replay.rs"]
mod replay;
#[path = "release_source.rs"]
mod source;
#[path = "release_validation.rs"]
mod validation;
#[path = "release_worktree.rs"]
mod worktree;

pub use replay::{
    ExecutedEntry, ExecutedJournal, ObservedJournalCompletion, format_executed_journal,
    observe_journal_completion,
};
pub use source::{ObservedSourceRelease, observe_source_identity};
pub use validation::{ObservedOwnerValidation, bind_owner_validation, observe_owner_validation};
pub use worktree::{GitPackage, ObservedGitRelease, bind_git_release, observe_git_release};

use crate::{CheckObservation, PackFailure, PackRequest, PackVerdict, harness};

/// Checks one release closure before a cold succession record is accepted.
pub(super) fn closure_precheck(
    request: &PackRequest<'_>,
) -> Result<Vec<CheckObservation>, PackFailure> {
    closure::check_nv12_27(request)
}

/// Refuses an observation whose whole value is a publication or completion claim.
pub(super) fn refuse_claim_values(observations: &[CheckObservation]) -> Result<(), PackFailure> {
    closure::refuse_claim_values(observations)
}

/// Closure observations shared by the funded release phase scopes.
pub const CLOSURE_FACTS: [&str; 12] = [
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
];

/// Checks the exact requested release scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-release", request, |request| {
        if request.scope == "release/nv12-27" {
            return closure::check_nv12_27(request);
        }
        if request.scope == "release/nv12-28" {
            return closure::check_nv12_28(request);
        }
        if request.scope == "release/source-identity" {
            return source::observations(request);
        }
        if request.scope == "release/git-worktree" {
            return worktree::observations(request);
        }
        if request.scope == "release/owner-validation" {
            return validation::observations(request);
        }
        if request.scope == "completion/replay" {
            return replay::observations(request);
        }
        CLOSURE_FACTS
            .into_iter()
            .map(|key| harness::expect_true(request, key))
            .collect::<Result<Vec<CheckObservation>, _>>()
    })
}
