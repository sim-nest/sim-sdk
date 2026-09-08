//! Loadable-product conformance pack.

use crate::{CheckObservation, PackRequest, PackVerdict, harness};

/// Checks funded loadable-product scopes.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-product", request, |request| {
        match request.scope {
            "product/world" => world(request),
            "product/query-latency" => query_latency(request),
            _ => unreachable!("availability was checked before dispatch"),
        }
    })
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
