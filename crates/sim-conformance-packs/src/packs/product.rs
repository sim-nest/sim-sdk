// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Loadable-product conformance pack.

use crate::{CheckObservation, PackRequest, PackVerdict, harness};

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

/// Loadable-product observations required beyond the world and latency scopes.
pub const PORTABLE_SCENARIOS: [&str; 7] = [
    "product.loads-through-sim-run",
    "product.stable-value",
    "product.index-route",
    "product.recipe-checked",
    "product.brochure-present",
    "product.boot-smoke-passed",
    "product.verb-not-baked",
];

fn portable(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    PORTABLE_SCENARIOS
        .into_iter()
        .map(|key| harness::expect_true(request, key))
        .collect()
}

fn world(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    [
        "product.world-loads-through-sim-run",
        "product.world-verb-not-kernel-or-frame",
        "product.world-project-stable-value",
        "product.world-diff-stable-value",
        "product.world-why-stable-value",
        "product.world-read-only",
        "product.world-no-observation-port",
        "product.world-no-proof-execution-port",
        "product.world-no-mutation-capability",
        "product.world-index-route",
        "product.world-recipe-checked",
        "product.world-brochure-present",
        "product.world-boot-smoke-passed",
    ]
    .into_iter()
    .map(|key| harness::expect_true(request, key))
    .collect()
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
