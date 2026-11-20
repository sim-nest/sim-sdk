// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Claim 8 for the reviewed stage-one assay.
//!
//! The check calls `stage_one::execute`. The result is `completion/claim-8`
//! only when that run reviewed the oracle before the provider observation,
//! matched denominator 40, and held the D1, D2, and D4 floors. A caller
//! label is not this result.

use sha2::{Digest, Sha256};
use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};

use super::receipt::{self, AdmitExecuted, PackageRow};
use crate::{CheckObservation, MemorySubject, PackFailure, PackRequest, PackVerdict, find_pack};

const SCOPE: &str = "completion/stage-one";
const RESULT: &str = "completion/claim-8";
const FACT: &str = "completion.stage-one";
const PROOF: &str = "stage-one";
const DOMAIN: &str = "completion/stage-one";
const EXPECTED: &str = concat!(
    "oracle-reviewed denominator-40 bounds-100-95-70 ",
    "copied-refused roles-refused freeze-refused short-refused swapped-refused"
);

/// Receipt admitted after the reviewed stage-one assay holds.
#[derive(Debug)]
pub struct ObservedStageOne {
    /// Subject the checker invocation and the receipt both name.
    pub subject: String,
    /// `completion/claim-8` for this subject.
    pub result: String,
    /// Tokens the reviewed assay recorded.
    pub execution: String,
    /// Catalog that admitted the evidence.
    pub catalog: sim_lib_roadmap_runner::ProofCatalog,
    /// Evidence the catalog admitted.
    pub admitted: sim_lib_roadmap_runner::AdmittedEvidence,
    /// Checker receipt that bound the admission.
    pub checker: sim_conformance_v02::CheckerReceiptId,
    /// Scope `completion/stage-one`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Runs the reviewed stage-one assay and admits claim 8.
///
/// # Errors
///
/// Returns a refusal when the subject is not a token, the fact carries a
/// completion label, or the assay leaves a review, set, or floor gap.
pub fn observe_stage_one(subject: &str) -> Result<ObservedStageOne, PackFailure> {
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

/// Re-checks a stage-one receipt against the catalog that admitted it.
///
/// The execution string has to be the reviewed assay. A claim label in that
/// string is refused before the assay runs again.
///
/// # Errors
///
/// Returns a refusal when the result, scope, or execution string is no
/// longer the reviewed assay.
pub fn bind_stage_one(observation: &ObservedStageOne) -> Result<(), PackFailure> {
    if observation.result != RESULT
        || observation.scope != scope_id()?
        || observation.execution.contains("completion/claim-")
    {
        return Err(fail("claim-relabelled", SCOPE));
    }
    let again = execute()?;
    if observation.execution != again {
        return Err(fail("evidence-mismatch", FACT));
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
) -> Result<ObservedStageOne, PackFailure> {
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
        policy: "policy/stage-one",
        provenance: "provenance/stage-one",
        support: "support/stage-one",
        projection: "projection/stage-one",
        environment: "environment/stage-one",
        leaf: executed.as_bytes(),
        source_text: &receipt::package_source(DOMAIN, PROOF, &rows),
        definitions: &receipt::package_definitions(DOMAIN, &rows),
    })?;
    Ok(ObservedStageOne {
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
    let stage = sim_lib_world::stage_one::execute()
        .map_err(|error| fail("stage-gap", &error.to_string()))?;
    let mut tokens = Vec::new();
    if !stage.frozen
        || stage.author == stage.reviewer
        || !hex64(&stage.author)
        || !hex64(&stage.reviewer)
        || !hex64(&stage.subject)
    {
        return Err(fail("stage-gap", "review"));
    }
    tokens.push("oracle-reviewed");
    if stage.denominator != 40 {
        return Err(fail("stage-gap", "denominator"));
    }
    tokens.push("denominator-40");
    if stage.d1 != 10_000 || stage.d2 < 9_500 || stage.d4 < 7_000 {
        return Err(fail("stage-gap", "bounds"));
    }
    tokens.push("bounds-100-95-70");
    sim_lib_world::stage_one::copied_prediction_is_refused("affected conclusion/01")
        .map_err(|error| fail("stage-gap", &error.to_string()))?;
    tokens.push("copied-refused");
    sim_lib_world::stage_one::same_role_is_refused()
        .map_err(|error| fail("stage-gap", &error.to_string()))?;
    tokens.push("roles-refused");
    sim_lib_world::stage_one::unreviewed_freeze_is_refused()
        .map_err(|error| fail("stage-gap", &error.to_string()))?;
    tokens.push("freeze-refused");
    sim_lib_world::stage_one::short_bound_is_refused(10_000, 9_499, 7_000)
        .map_err(|error| fail("stage-gap", &error.to_string()))?;
    tokens.push("short-refused");
    sim_lib_world::stage_one::swapped_set_is_refused()
        .map_err(|error| fail("stage-gap", &error.to_string()))?;
    tokens.push("swapped-refused");
    let executed = tokens.join(" ");
    if executed.contains("completion/claim-") {
        return Err(fail("claim-relabelled", SCOPE));
    }
    if executed != EXPECTED {
        return Err(fail("stage-gap", SCOPE));
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
    body.ok_or_else(|| fail("missing-stage-one", FACT))
}

fn execution(observations: &[CheckObservation]) -> Result<&str, PackFailure> {
    observations
        .iter()
        .find(|item| item.key == "completion.execution")
        .map(|item| item.value.as_str())
        .ok_or_else(|| fail("missing-stage-one", FACT))
}

fn token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !value.contains("completion/claim-")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && byte != b' ')
}

fn hex64(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
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
#[path = "release_stage_tests.rs"]
mod tests;
