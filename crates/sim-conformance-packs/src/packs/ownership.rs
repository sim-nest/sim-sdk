// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Activation ownership checks.

use super::succession_record::{self, fail};
use crate::{CheckObservation, PackRequest, PackVerdict, harness};

/// Checks the exact requested ownership scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-own", request, |request| match request.scope {
        "ownership/activation" => activation(request),
        "ownership/dependencies" => dependencies(request),
        "ownership/bootstrap" => bootstrap(request),
        "ownership/produced" => produced(request),
        "ownership/roadmap-final" => roadmap_final(request),
        _ => unreachable!("availability was checked before dispatch"),
    })
}

fn roadmap_final(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    let mut record_text = None;
    let mut edges = 0_u64;
    let mut cycles = 0_u64;
    let mut planned = 0_u64;
    for key in request.evidence.fact_keys() {
        if key == "ownership.succession-record" {
            record_text = request.evidence.fact(key);
            continue;
        }
        if let Some(name) = graph_name(key, "ownership.edge.") {
            if name.is_empty() {
                return Err(fail("unexpected-fact", key));
            }
            edges = edges.saturating_add(1);
            continue;
        }
        if let Some(name) = graph_name(key, "ownership.cycle.") {
            if name.is_empty() {
                return Err(fail("unexpected-fact", key));
            }
            cycles = cycles.saturating_add(1);
            continue;
        }
        if let Some(name) = graph_name(key, "ownership.planned.") {
            if name.is_empty() {
                return Err(fail("unexpected-fact", key));
            }
            planned = planned.saturating_add(1);
            continue;
        }
        return Err(fail("unexpected-fact", key));
    }
    if edges != 0 || cycles != 0 || planned != 0 {
        return Err(fail("graph", "ownership.graph"));
    }
    let Some(record_text) = record_text else {
        return Err(fail("missing-record", "ownership.succession-record"));
    };
    let derived = succession_record::derive(record_text)?;
    Ok(vec![
        observation("ownership.closed", &derived.closed),
        observation("ownership.change", &derived.change),
        observation("ownership.project", &derived.project),
        observation("ownership.holds", &derived.holds.to_string()),
        observation(
            "ownership.second-converge",
            &derived.converge_second.to_string(),
        ),
        observation(
            "ownership.second-deliver",
            &derived.deliver_second.to_string(),
        ),
        observation("ownership.edges", &edges.to_string()),
        observation("ownership.cycles", &cycles.to_string()),
        observation("ownership.planned-targets", &planned.to_string()),
        observation("ownership.linux", &derived.linux),
    ])
}

fn graph_name<'a>(key: &'a str, prefix: &str) -> Option<&'a str> {
    key.strip_prefix(prefix)
}

fn observation(key: &str, value: &str) -> CheckObservation {
    CheckObservation {
        key: key.to_owned(),
        value: value.to_owned(),
    }
}

fn produced(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    let mut observations = harness::expect_same_u64(
        request,
        "ownership.produced-target-surfaces",
        "ownership.qualified-target-surfaces",
    )?;
    for key in [
        "ownership.target-was-planned-before-construction",
        "ownership.packet-dependencies-released",
        "ownership.unimplemented-external-dependency-refused",
        "ownership.target-source-qualified",
        "ownership.target-api-qualified",
        "ownership.target-release-qualified",
        "ownership.owner-validation-and-docs-exact",
        "ownership.index-route-present",
        "ownership.crate-admission-complete",
        "ownership.support-graph-acyclic",
        "ownership.overlaps-zero",
        "ownership.kernel-head-unchanged",
    ] {
        observations.push(harness::expect_true(request, key)?);
    }
    Ok(observations)
}

fn bootstrap(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    let mut observations = harness::expect_same_u64(
        request,
        "ownership.bootstrap-reached-surfaces",
        "ownership.bootstrap-qualified-surfaces",
    )?;
    for key in [
        "ownership.bootstrap-closure-complete",
        "ownership.bootstrap-transitive-dependencies-released",
        "ownership.bootstrap-produced-source-qualified",
        "ownership.bootstrap-produced-api-qualified",
        "ownership.bootstrap-produced-release-qualified",
        "ownership.bootstrap-binding-receipts-verified",
        "ownership.bootstrap-dependency-use-sets-qualified",
        "ownership.bootstrap-later-surfaces-planned",
        "ownership.bootstrap-roadmap-final-unclaimed",
        "ownership.bootstrap-handoff-boundary-recorded",
        "ownership.bootstrap-support-graph-acyclic",
        "ownership.bootstrap-overlaps-zero",
    ] {
        observations.push(harness::expect_true(request, key)?);
    }
    Ok(observations)
}

fn activation(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    let mut observations = Vec::<CheckObservation>::new();
    for (actual, expected) in [
        (
            "ownership.owner-bindings",
            "ownership.expected-owner-bindings",
        ),
        (
            "ownership.checker-bindings",
            "ownership.expected-checker-bindings",
        ),
        ("ownership.phase-gates", "ownership.expected-phase-gates"),
        (
            "ownership.dependency-use-sets",
            "ownership.expected-dependency-use-sets",
        ),
    ] {
        observations.extend(harness::expect_same_u64(request, actual, expected)?);
    }
    for key in [
        "ownership.unresolved-rows-zero",
        "ownership.bindings-contain-no-invocations",
        "ownership.planned-target-admitted",
        "ownership.unavailable-dependency-refused",
        "ownership.activation-as-final-refused",
        "ownership.support-graph-acyclic",
    ] {
        observations.push(harness::expect_true(request, key)?);
    }
    Ok(observations)
}

fn dependencies(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    let mut observations = harness::expect_same_u64(
        request,
        "ownership.reached-owner-surfaces",
        "ownership.qualified-owner-surfaces",
    )?;
    for key in [
        "ownership.actual-closure-complete",
        "ownership.loader-owner-qualified",
        "ownership.tooling-owner-qualified",
        "ownership.codec-owns-vault-identity",
        "ownership.dependency-use-sets-qualified",
        "ownership.unresolved-rows-zero",
        "ownership.singular-owner-per-construction",
        "ownership.support-graph-acyclic",
        "ownership.overlaps-zero",
        "ownership.unavailable-dependency-refused",
    ] {
        observations.push(harness::expect_true(request, key)?);
    }
    Ok(observations)
}
