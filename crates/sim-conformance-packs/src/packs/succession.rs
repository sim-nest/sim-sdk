// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Cold-succession conformance pack.
//!
//! The precheck derives a release closure and refuses a cold-change record.
//! The cold-change scope derives that record. The fresh-command scope reads
//! the installed later change. None of these scopes accepts a publication
//! flag or derives completion.

#[path = "succession_fresh.rs"]
mod fresh;

use super::succession_record::{self, fail};
use crate::{CheckObservation, PackRequest, PackVerdict, harness};

pub use fresh::observe_fresh_command;

/// Checks the exact requested succession scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-succeed", request, |request| {
        match request.scope {
            "succession/composed-precheck" => composed_precheck(request),
            "succession/cold-change" => cold_change(request),
            fresh::SCOPE => fresh::observations(request),
            _ => unreachable!("availability was checked before dispatch"),
        }
    })
}

fn composed_precheck(
    request: &PackRequest<'_>,
) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    if request
        .evidence
        .fact("release.publication-confirmed")
        .is_some()
    {
        return Err(fail(
            "local-publication-claim",
            "release.publication-confirmed",
        ));
    }
    let mut saw_cold_record = false;
    for key in request.evidence.fact_keys() {
        match key {
            "release.closure" | "release.dry-run" => {}
            "succession.record" => saw_cold_record = true,
            other => return Err(fail("unexpected-fact", other)),
        }
    }
    if saw_cold_record {
        return Err(fail("cold-change-present", "succession.record"));
    }
    let mut observations = super::release::closure_precheck(request)?;
    observations.push(observation("succession.precheck", "ready"));
    super::release::refuse_claim_values(&observations)?;
    Ok(observations)
}

fn cold_change(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    let mut record_text = None;
    for key in request.evidence.fact_keys() {
        match key {
            "succession.record" => record_text = request.evidence.fact(key),
            other => return Err(fail("unexpected-fact", other)),
        }
    }
    let Some(record_text) = record_text else {
        return Err(fail("missing-record", "succession.record"));
    };
    let derived = succession_record::derive(record_text)?;
    Ok(vec![
        observation("succession.closed", &derived.closed),
        observation("succession.change", &derived.change),
        observation("succession.project", &derived.project),
        observation(
            "succession.second-converge",
            &derived.converge_second.to_string(),
        ),
        observation(
            "succession.second-deliver",
            &derived.deliver_second.to_string(),
        ),
        observation("succession.holds", &derived.holds.to_string()),
        observation("succession.linux", &derived.linux),
    ])
}

fn observation(key: &str, value: &str) -> CheckObservation {
    CheckObservation {
        key: key.to_owned(),
        value: value.to_owned(),
    }
}
