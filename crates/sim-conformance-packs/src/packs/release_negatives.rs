// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Required negative and mutation refusals for the release checker.
//!
//! The check calls the loaded bind command. The result is
//! `completion/claim-6` only when a same-domain mismatch and a mutated
//! digest are `wrong-binding`, another domain is `domain-confusion`, and
//! the exact pair is `accepted`. A caller label is not this result.

use sha2::{Digest, Sha256};
use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};

use super::receipt::{self, AdmitExecuted, PackageRow};
use crate::{CheckObservation, MemorySubject, PackFailure, PackRequest, PackVerdict, find_pack};

const SCOPE: &str = "completion/negatives";
const RESULT: &str = "completion/claim-6";
const FACT: &str = "completion.negatives";
const PROOF: &str = "negatives";
const DOMAIN: &str = "completion/negatives";
const EXPECTED: &str = "wrong-binding domain-confusion wrong-binding accepted";

/// Receipt admitted after the loaded bind negatives hold.
#[derive(Debug)]
pub struct ObservedNegatives {
    /// Subject the checker invocation and the receipt both name.
    pub subject: String,
    /// `completion/claim-6` for this subject.
    pub result: String,
    /// Reasons the loaded bind command returned.
    pub execution: String,
    /// Catalog that admitted the evidence.
    pub catalog: sim_lib_roadmap_runner::ProofCatalog,
    /// Evidence the catalog admitted.
    pub admitted: sim_lib_roadmap_runner::AdmittedEvidence,
    /// Checker receipt that bound the admission.
    pub checker: sim_conformance_v02::CheckerReceiptId,
    /// Scope `completion/negatives`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Runs the loaded bind negatives and admits claim 6.
///
/// # Errors
///
/// Returns a refusal when the subject is not a token, the fact carries a
/// completion label, or the loaded command returns another reason.
pub fn observe_negatives(subject: &str) -> Result<ObservedNegatives, PackFailure> {
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

/// Re-checks a negatives receipt against the catalog that admitted it.
///
/// The execution string has to be the loaded reasons. A claim label in
/// that string is refused before the command runs again.
///
/// # Errors
///
/// Returns a refusal when the result, scope, or execution string is no
/// longer the loaded negatives.
pub fn bind_negatives(observation: &ObservedNegatives) -> Result<(), PackFailure> {
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
) -> Result<ObservedNegatives, PackFailure> {
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
        policy: "policy/negatives",
        provenance: "provenance/negatives",
        support: "support/negatives",
        projection: "projection/negatives",
        environment: "environment/negatives",
        leaf: executed.as_bytes(),
        source_text: &receipt::package_source(DOMAIN, PROOF, &rows),
        definitions: &receipt::package_definitions(DOMAIN, &rows),
    })?;
    Ok(ObservedNegatives {
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
    let loaded =
        sim_lib_world::bind::execute().map_err(|error| fail("negative-gap", &error.to_string()))?;
    require(&loaded.wrong_binding, "wrong-binding", "crossed")?;
    require(&loaded.domain_confusion, "domain-confusion", "confused")?;
    require(&loaded.mutated, "wrong-binding", "mutated")?;
    require(&loaded.exact, "accepted", "exact")?;
    if loaded.domain_confusion == loaded.wrong_binding || loaded.mutated == loaded.domain_confusion
    {
        return Err(fail("negative-gap", "collapsed-reason"));
    }
    let executed = format!(
        "{} {} {} {}",
        loaded.wrong_binding, loaded.domain_confusion, loaded.mutated, loaded.exact
    );
    if executed.contains("completion/claim-") {
        return Err(fail("claim-relabelled", SCOPE));
    }
    if executed != EXPECTED {
        return Err(fail("negative-gap", SCOPE));
    }
    Ok(executed)
}

fn require(found: &str, intended: &str, label: &str) -> Result<(), PackFailure> {
    if found == intended {
        Ok(())
    } else {
        Err(fail("negative-gap", label))
    }
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
    body.ok_or_else(|| fail("missing-negatives", FACT))
}

fn execution(observations: &[CheckObservation]) -> Result<&str, PackFailure> {
    observations
        .iter()
        .find(|item| item.key == "completion.execution")
        .map(|item| item.value.as_str())
        .ok_or_else(|| fail("missing-negatives", FACT))
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
#[path = "release_negatives_tests.rs"]
mod tests;
