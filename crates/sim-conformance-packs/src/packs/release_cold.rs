// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Claim 9 for the loaded cold comparison.
//!
//! The check calls `cold_stage::execute`. The result is `completion/claim-9`
//! only when that run reports the complete denominator, zero false reuse,
//! and the 100/95/70 percent D1/D2/D4 bounds. A scripted receipt is refused
//! before launch. A caller label is not this result.

use sha2::{Digest, Sha256};
use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};

use super::receipt::{self, AdmitExecuted, PackageRow};
use crate::{CheckObservation, MemorySubject, PackFailure, PackRequest, PackVerdict, find_pack};

const SCOPE: &str = "completion/cold-stage";
const RESULT: &str = "completion/claim-9";
const FACT: &str = "completion.cold-stage";
const PROOF: &str = "cold-stage";
const DOMAIN: &str = "completion/cold-stage";
const EXPECTED: &str = concat!(
    "cold-denominator zero-false-reuse bounds-100-95-70 ",
    "scripted-refused false-reuse"
);

/// Receipt admitted after the loaded cold comparison holds.
#[derive(Debug)]
pub struct ObservedColdStage {
    /// Subject the checker invocation and the receipt both name.
    pub subject: String,
    /// `completion/claim-9` for this subject.
    pub result: String,
    /// Tokens the loaded comparison recorded.
    pub execution: String,
    /// Catalog that admitted the evidence.
    pub catalog: sim_lib_roadmap_runner::ProofCatalog,
    /// Evidence the catalog admitted.
    pub admitted: sim_lib_roadmap_runner::AdmittedEvidence,
    /// Checker receipt that bound the admission.
    pub checker: sim_conformance_v02::CheckerReceiptId,
    /// Scope `completion/cold-stage`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Runs the loaded cold comparison and admits claim 9.
///
/// # Errors
///
/// Returns a refusal when the subject is not a token, the fact carries a
/// completion label, or the comparison leaves a denominator, reuse, or bound gap.
pub fn observe_cold_stage(subject: &str) -> Result<ObservedColdStage, PackFailure> {
    let checked = CheckedSubjectId::from_text(subject)
        .map_err(|error| fail("subject-identity", &error.to_string()))?;
    let spec = find_pack("checker/c-release")
        .ok_or_else(|| fail("unknown-checker", "checker/c-release"))?;
    let binding = spec
        .checker_binding()
        .map_err(|error| fail("invalid-static-binding", &error.to_string()))?;
    let evidence = MemorySubject::default().with(FACT, encode_request(subject)?);
    let verdict = super::check(&PackRequest {
        checker: "checker/c-release",
        binding: &receipt::render(binding.id().content_id()),
        subject: &receipt::render(checked.content_id()),
        scope: SCOPE,
        evidence: &evidence,
    });
    let observations = match verdict {
        PackVerdict::Pass { observations, .. } => observations,
        PackVerdict::Refused(failure) => return Err(failure),
        PackVerdict::UnimplementedPack { .. } => return Err(fail("unimplemented", SCOPE)),
    };
    admit(subject, &observations)
}

/// Re-checks a cold-stage receipt against the catalog that admitted it.
///
/// # Errors
///
/// Returns a refusal when the result or scope is no longer claim 9.
pub fn bind_cold_stage(observation: &ObservedColdStage) -> Result<(), PackFailure> {
    if observation.result != RESULT || observation.scope != scope_id()? {
        return Err(fail("claim-relabelled", SCOPE));
    }
    receipt::bind_executed(
        &observation.subject,
        &observation.result,
        &observation.scope,
        &observation.checker,
        &observation.catalog,
        &observation.admitted,
        observation.revocation.clone(),
    )
}

pub(super) fn observations(
    request: &PackRequest<'_>,
) -> Result<Vec<CheckObservation>, PackFailure> {
    let body = one_fact(request)?;
    if body.contains("completion/claim-") {
        return Err(fail("claim-relabelled", FACT));
    }
    let subject = decode_request(body)?;
    let bound =
        CheckedSubjectId::from_text(&subject).map_err(|_| fail("subject-mismatch", FACT))?;
    if receipt::render(bound.content_id()) != request.subject {
        return Err(fail("subject-mismatch", FACT));
    }
    let executed = execute()?;
    Ok(vec![observation("completion.execution", executed)])
}

fn admit(
    subject: &str,
    observations: &[CheckObservation],
) -> Result<ObservedColdStage, PackFailure> {
    let executed = execution(observations)?;
    let again = execute()?;
    if executed != again {
        return Err(fail("evidence-mismatch", FACT));
    }
    let rows = vec![PackageRow {
        name: PROOF.to_owned(),
        version: "0".to_owned(),
        url: format!("git://{PROOF}"),
        rev: hex_bytes(Sha256::digest(executed.as_bytes()).as_slice()),
    }];
    let admitted = receipt::admit_executed(AdmitExecuted {
        subject,
        result: RESULT,
        scope: SCOPE,
        proof: PROOF,
        domain: DOMAIN,
        policy: "policy/cold-stage",
        provenance: "provenance/cold-stage",
        support: "support/cold-stage",
        projection: "projection/cold-stage",
        environment: "environment/cold-stage",
        leaf: executed.as_bytes(),
        source_text: &receipt::package_source(DOMAIN, PROOF, &rows),
        definitions: &receipt::package_definitions(DOMAIN, &rows),
    })?;
    Ok(ObservedColdStage {
        subject: subject.to_owned(),
        result: RESULT.to_owned(),
        execution: executed.to_owned(),
        catalog: admitted.catalog,
        admitted: admitted.admitted,
        checker: admitted.checker,
        scope: admitted.scope,
        revocation: admitted.revocation,
    })
}

fn execute() -> Result<String, PackFailure> {
    let stage = sim_lib_world::cold_stage::execute()
        .map_err(|error| fail("cold-gap", &error.to_string()))?;
    let mut tokens = Vec::new();
    if stage.denominator == 0 || !stage.reuse_disabled {
        return Err(fail("cold-gap", "denominator"));
    }
    tokens.push("cold-denominator");
    if stage.false_reuse != 0 {
        return Err(fail("cold-gap", "false-reuse"));
    }
    tokens.push("zero-false-reuse");
    if stage.d1 != 10_000 || stage.d2 != 9_500 || stage.d4 != 7_000 {
        return Err(fail("cold-gap", "bounds"));
    }
    tokens.push("bounds-100-95-70");
    sim_lib_world::cold_stage::scripted_receipt_is_refused("conclusion/01")
        .map_err(|error| fail("cold-gap", &error.to_string()))?;
    tokens.push("scripted-refused");
    sim_lib_world::cold_stage::contradictory_basis_is_false_reuse()
        .map_err(|error| fail("cold-gap", &error.to_string()))?;
    tokens.push("false-reuse");
    let executed = tokens.join(" ");
    if executed.contains("completion/claim-") {
        return Err(fail("claim-relabelled", SCOPE));
    }
    if executed != EXPECTED {
        return Err(fail("cold-gap", SCOPE));
    }
    Ok(executed)
}

fn encode_request(subject: &str) -> Result<String, PackFailure> {
    if !token(subject) {
        return Err(fail("bad-record", FACT));
    }
    let mut body = String::new();
    receipt::push_field(&mut body, subject);
    Ok(body)
}

fn decode_request(body: &str) -> Result<String, PackFailure> {
    let (subject, rest) = receipt::take_field(body)?;
    if !rest.is_empty() || !token(subject) {
        return Err(fail("bad-record", FACT));
    }
    Ok(subject.to_owned())
}

fn one_fact<'a>(request: &'a PackRequest<'a>) -> Result<&'a str, PackFailure> {
    let mut body = None;
    for key in request.evidence.fact_keys() {
        match key {
            FACT => body = request.evidence.fact(key),
            other => return Err(fail("unexpected-fact", other)),
        }
    }
    body.ok_or_else(|| fail("missing-cold-stage", FACT))
}

fn execution(observations: &[CheckObservation]) -> Result<&str, PackFailure> {
    observations
        .iter()
        .find(|item| item.key == "completion.execution")
        .map(|item| item.value.as_str())
        .ok_or_else(|| fail("missing-cold-stage", FACT))
}

fn token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !value.contains("completion/claim-")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && byte != b' ')
}

fn scope_id() -> Result<CheckScopeId, PackFailure> {
    CheckScopeId::from_text(SCOPE).map_err(|error| fail("receipt-refused", &error.to_string()))
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn observation(key: &str, value: impl Into<String>) -> CheckObservation {
    CheckObservation {
        key: key.to_owned(),
        value: value.into(),
    }
}

fn fail(code: &'static str, detail: &str) -> PackFailure {
    PackFailure {
        code,
        detail: detail.chars().take(256).collect(),
    }
}

#[cfg(test)]
#[path = "release_cold_tests.rs"]
mod tests;
