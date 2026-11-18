// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Outward-delivery conformance pack.
//!
//! Each funded scope runs `world deliver model` and admits that program's
//! record only when the seven delivery laws are printed in it. A caller fact
//! is not a delivery.

#[path = "delivery_execute.rs"]
mod execute;

use crate::{PackRequest, PackVerdict, harness};

/// Fact whose value is the filesystem path of the `sim` program.
pub const COMMAND_FACT: &str = "delivery.command";

/// Laws the deliver record must show. These names are not caller facts.
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
            | "delivery/composed" => execute::record(request),
            _ => unreachable!("availability was checked before dispatch"),
        }
    })
}
