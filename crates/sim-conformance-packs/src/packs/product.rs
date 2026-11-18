// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Loadable-product conformance pack.
//!
//! World, prove, deliver, and roadmap run the program named by
//! [`COMMAND_FACT`] and record that program's output. Query latency compares
//! its measurement facts. The remaining portable product scopes have no
//! executed command.

#[path = "product_command.rs"]
mod command;

use crate::{CheckObservation, PackRequest, PackVerdict, harness};

/// Fact whose value is the filesystem path of the `sim` program.
pub const COMMAND_FACT: &str = "product.command";

/// Checks funded loadable-product scopes.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-product", request, |request| {
        match request.scope {
            "product/world" => world(request),
            "product/query-latency" => query_latency(request),
            "product/prove-plan"
            | "product/prove"
            | "product/convergence-queries"
            | "product/landing-queries"
            | "product/deliver"
            | "product/roadmap"
            | "product/installed"
            | "product/composed" => portable(request),
            _ => unreachable!("availability was checked before dispatch"),
        }
    })
}

fn portable(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    command::portable(request)
}

fn world(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    command::world(request)
}

fn query_latency(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    let mut observations = harness::expect_eq(request, "product.query-command", "world/why")
        .map(|observation| vec![observation])?;
    for key in [
        "product.query-warmups-20",
        "product.query-samples-200",
        "product.query-p95-at-most-200ms",
        "product.query-effect-calls-zero",
    ] {
        observations.push(harness::expect_true(request, key)?);
    }
    Ok(observations)
}
