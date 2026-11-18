// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Brochure guard executed from the repository that contains it.
//!
//! The caller names a repository root. This module runs
//! `bin/simctl-brochure --check` through `/bin/sh` with that root as the
//! working directory and records the output hashes. It does not take a
//! command string or a passed count. The result is `brochure-guard`.
//! It is not `completion/claim-18`: the command checks brochure files,
//! and it does not load World, prove, deliver, or roadmap, or read Index
//! routes, stable values, recipes, or a kernel verb table.

use std::path::Path;

use sim_conformance_v02::{CheckScopeId, CheckerReceiptId, RevocationStatus};
use sim_lib_roadmap_runner::{AdmittedEvidence, ProofCatalog};

use super::command::{self, BoundCommand, GuardSpec, ObservedCommand};
use crate::{CheckObservation, PackFailure, PackRequest};

const SPEC: GuardSpec = GuardSpec {
    scope: "release/brochure-guard",
    domain: "brochure/guard",
    fact: "release.brochure",
    result: "brochure-guard",
    script: "bin/simctl-brochure",
    args: &["--check"],
};

/// Receipt produced by running the brochure check.
pub struct ObservedBrochureGuard {
    /// Subject the receipt names.
    pub subject: String,
    /// `brochure-guard`. This is not `completion/claim-18`.
    pub result: String,
    /// Canonical root whose command ran.
    pub root: String,
    /// Catalog that admitted the evidence.
    pub catalog: ProofCatalog,
    /// Evidence the catalog admitted.
    pub admitted: AdmittedEvidence,
    /// Checker receipt that bound the admission.
    pub checker: CheckerReceiptId,
    /// Scope `release/brochure-guard`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Runs the brochure check and admits `brochure-guard`.
///
/// # Errors
///
/// Returns a refusal when the command is absent, leaves the root, or exits
/// non-zero. The command's working directory is the canonical root.
pub fn observe_brochure_guard(
    subject: &str,
    root: &Path,
) -> Result<ObservedBrochureGuard, PackFailure> {
    command::observe(&SPEC, subject, root).map(from_command)
}

/// Re-checks a brochure receipt against the catalog that admitted it.
///
/// # Errors
///
/// Returns a refusal when the result or scope is no longer `brochure-guard`.
pub fn bind_brochure_guard(observation: &ObservedBrochureGuard) -> Result<(), PackFailure> {
    command::bind(&SPEC, bound(observation))
}

pub(super) fn observations(
    request: &PackRequest<'_>,
) -> Result<Vec<CheckObservation>, PackFailure> {
    command::observations(&SPEC, request)
}

fn bound(observation: &ObservedBrochureGuard) -> BoundCommand<'_> {
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

fn from_command(observed: ObservedCommand) -> ObservedBrochureGuard {
    ObservedBrochureGuard {
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
#[path = "release_brochure_tests.rs"]
mod tests;
