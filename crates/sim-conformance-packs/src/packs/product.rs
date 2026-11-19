// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Loadable-product conformance pack.
//!
//! World, prove, deliver, roadmap, cached query latency, land, and converge run the
//! program named by [`COMMAND_FACT`] and record that program's output. The
//! remaining portable product scopes have no executed command.

#[path = "product_command.rs"]
mod command;
#[path = "product_converge.rs"]
mod converge;
#[path = "product_land.rs"]
mod land;
#[path = "product_query.rs"]
mod query;

use crate::{CheckObservation, PackRequest, PackVerdict, harness};

/// Fact whose value is the filesystem path of the `sim` program.
pub const COMMAND_FACT: &str = "product.command";

/// Checks funded loadable-product scopes.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-product", request, |request| {
        match request.scope {
            "product/world" => world(request),
            "product/query-latency" => query_latency(request),
            "product/landing-queries" => land::queries(request),
            "product/convergence-queries" => converge::queries(request),
            "product/prove-plan" | "product/prove" | "product/deliver" | "product/roadmap"
            | "product/installed" | "product/composed" => portable(request),
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
    query::measure(request)
}
