// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Recipe guard executed from the repository that contains it.
//!
//! The caller names a repository root. This module runs
//! `bin/simctl-cookbook-lint --check` through `/bin/sh` with that root as the
//! working directory and records the output hashes. It does not take a
//! command string or a passed count. The result is `recipe-guard`.
//! It is not `completion/claim-18`: the command lints cookbook recipes,
//! and it does not run them or load World, prove, deliver, or roadmap.

use std::path::Path;

use sim_conformance_v02::{CheckScopeId, CheckerReceiptId, RevocationStatus};
use sim_lib_roadmap_runner::{AdmittedEvidence, ProofCatalog};

use super::command::{self, BoundCommand, GuardSpec, ObservedCommand};
use crate::{CheckObservation, PackFailure, PackRequest};

const SPEC: GuardSpec = GuardSpec {
    scope: "release/recipe-guard",
    domain: "recipe/guard",
    fact: "release.recipe",
    result: "recipe-guard",
    script: "bin/simctl-cookbook-lint",
    args: &["--check"],
};

/// Receipt produced by running the recipe lint.
pub struct ObservedRecipeGuard {
    /// Subject the receipt names.
    pub subject: String,
    /// `recipe-guard`. This is not `completion/claim-18`.
    pub result: String,
    /// Canonical root whose command ran.
    pub root: String,
    /// Catalog that admitted the evidence.
    pub catalog: ProofCatalog,
    /// Evidence the catalog admitted.
    pub admitted: AdmittedEvidence,
    /// Checker receipt that bound the admission.
    pub checker: CheckerReceiptId,
    /// Scope `release/recipe-guard`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Runs the recipe lint and admits `recipe-guard`.
///
/// # Errors
///
/// Returns a refusal when the command is absent, leaves the root, or exits
/// non-zero. The command's working directory is the canonical root.
pub fn observe_recipe_guard(
    subject: &str,
    root: &Path,
) -> Result<ObservedRecipeGuard, PackFailure> {
    command::observe(&SPEC, subject, root).map(from_command)
}

/// Re-checks a recipe receipt against the catalog that admitted it.
///
/// # Errors
///
/// Returns a refusal when the result or scope is no longer `recipe-guard`.
pub fn bind_recipe_guard(observation: &ObservedRecipeGuard) -> Result<(), PackFailure> {
    command::bind(&SPEC, bound(observation))
}

pub(super) fn observations(
    request: &PackRequest<'_>,
) -> Result<Vec<CheckObservation>, PackFailure> {
    command::observations(&SPEC, request)
}

fn bound(observation: &ObservedRecipeGuard) -> BoundCommand<'_> {
    BoundCommand {
        subject: &observation.subject,
        result: &observation.result,
        scope: &observation.scope,
        checker: &observation.checker,
        catalog: &observation.catalog,
        admitted: &observation.admitted,
        revocation: &observation.revocation,
    }
}

fn from_command(observed: ObservedCommand) -> ObservedRecipeGuard {
    ObservedRecipeGuard {
        subject: observed.subject,
        result: observed.result,
        root: observed.root,
        catalog: observed.catalog,
        admitted: observed.admitted,
        checker: observed.checker,
        scope: observed.scope,
        revocation: observed.revocation,
    }
}

#[cfg(test)]
#[path = "release_recipe_tests.rs"]
mod tests;
