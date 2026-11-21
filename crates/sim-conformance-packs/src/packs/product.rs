// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Loadable-product conformance pack.
//!
//! World, prove, deliver, roadmap, cached query latency, land, converge,
//! qualify, and promote run the program named by [`COMMAND_FACT`] and record
//! that program's output. The joined load records those four commands, their
//! index routes, and the specimen files under `product.root`. Boot, recipe,
//! brochure, index, and route guards stay their own results.

#[path = "product_command.rs"]
mod command;
#[path = "product_compose.rs"]
mod compose;
#[path = "product_converge.rs"]
mod converge;
#[path = "product_install.rs"]
mod install;
#[path = "product_land.rs"]
mod land;
#[path = "product_load.rs"]
mod load;
#[path = "product_query.rs"]
mod query;

pub use load::{bind_product_load, observe_product_load};

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
            "product/composed" => compose::records(request),
            "product/installed" => install::records(request),
            "product/prove-plan" | "product/prove" | "product/deliver" | "product/roadmap" => {
                portable(request)
            }
            "completion/product-load" => load::observations(request),
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
