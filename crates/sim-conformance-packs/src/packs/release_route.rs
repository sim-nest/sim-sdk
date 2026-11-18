// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Route guard executed from the repository that contains it.
//!
//! The caller names a repository root. This module runs
//! `bin/simctl-index route` for one fixed task through `/bin/sh` with that
//! root as the working directory. It admits `route-guard` only when the
//! command prints the route id. An empty route list is refused even when
//! the command exits 0. It does not take a command string or a passed
//! count. The result is `route-guard`. It is not `completion/claim-18`:
//! the command does not load World, prove, deliver, or roadmap.

use std::path::Path;

use sim_conformance_v02::{CheckScopeId, CheckerReceiptId, RevocationStatus};
use sim_lib_roadmap_runner::{AdmittedEvidence, ProofCatalog};

use super::command::{self, BoundCommand, GuardSpec, ObservedCommand};
use crate::{CheckObservation, PackFailure, PackRequest};

const ROUTE_ID: &str = "route/add-a-codec";

const SPEC: GuardSpec = GuardSpec {
    scope: "release/route-guard",
    domain: "route/guard",
    fact: "release.route",
    result: "route-guard",
    script: "bin/simctl-index",
    args: &["route", "add a codec"],
};

/// Receipt produced by running the route query.
pub struct ObservedRouteGuard {
    /// Subject the receipt names.
    pub subject: String,
    /// `route-guard`. This is not `completion/claim-18`.
    pub result: String,
    /// Canonical root whose command ran.
    pub root: String,
    /// Catalog that admitted the evidence.
    pub catalog: ProofCatalog,
    /// Evidence the catalog admitted.
    pub admitted: AdmittedEvidence,
    /// Checker receipt that bound the admission.
    pub checker: CheckerReceiptId,
    /// Scope `release/route-guard`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Runs the route query and admits `route-guard`.
///
/// # Errors
///
/// Returns a refusal when the command is absent, leaves the root, exits
/// non-zero, or prints no `route/add-a-codec` row. The command's working
/// directory is the canonical root.
pub fn observe_route_guard(subject: &str, root: &Path) -> Result<ObservedRouteGuard, PackFailure> {
    command::observe(&SPEC, subject, root).map(from_command)
}

/// Re-checks a route receipt against the catalog that admitted it.
///
/// # Errors
///
/// Returns a refusal when the result or scope is no longer `route-guard`.
pub fn bind_route_guard(observation: &ObservedRouteGuard) -> Result<(), PackFailure> {
    command::bind(&SPEC, bound(observation))
}

pub(super) fn observations(
    request: &PackRequest<'_>,
) -> Result<Vec<CheckObservation>, PackFailure> {
    command::checked(&SPEC, request, require_route)
}

fn require_route(stdout: &str) -> Result<(), PackFailure> {
    let found = stdout
        .lines()
        .any(|line| line.split('\t').next().is_some_and(|id| id == ROUTE_ID));
    if found {
        Ok(())
    } else {
        Err(PackFailure {
            code: "route-absent",
            detail: ROUTE_ID.to_owned(),
        })
    }
}

fn bound(observation: &ObservedRouteGuard) -> BoundCommand<'_> {
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

fn from_command(observed: ObservedCommand) -> ObservedRouteGuard {
    ObservedRouteGuard {
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
#[path = "release_route_tests.rs"]
mod tests;
