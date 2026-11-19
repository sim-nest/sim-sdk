// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Versioned-ensemble authoring conformance pack.
//!
//! Each funded scope runs `world author` and admits those records only when
//! the six authoring laws are printed in them. A caller fact is not an authoring.

#[path = "authoring_execute.rs"]
mod execute;

use crate::{PackRequest, PackVerdict, harness};

/// Fact whose value is the filesystem path of the `sim` program.
pub const COMMAND_FACT: &str = "authoring.command";

/// Laws the author records must show. These names are not caller facts.
pub const SCENARIOS: [&str; 6] = [
    "authoring.family-self-contained",
    "authoring.consultation-not-a-requirement",
    "authoring.forbidden-sibling-requirement-refused",
    "authoring.superseded-not-selected",
    "authoring.human-selection-exact",
    "authoring.exact-source-roundtrip",
];

/// Checks one funded authoring scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-author", request, |request| match request.scope {
        "authoring/family" | "authoring/composed" => execute::record(request),
        _ => unreachable!("availability was checked before dispatch"),
    })
}
