// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Release-grade proofs for the evidence checker.
//!
//! The check calls the loaded grades command. The result is
//! `completion/claim-11` only when the held case is release grade on a
//! stable key with current support and an empty frontier, and the
//! weak-grade, revoked, unstable-key, frontier-open, and
//! continuation-open cases each return their own reason. The receipt's
//! proof leaf and package revision are the exact seal that command
//! returns. The other five cases move that seal. A constant digest is
//! not that seal. A projection or identity-grade fact is another fact
//! and is refused. A caller label is not this result.

use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};

use crate::packs::release::receipt::{self, AdmitExecuted, PackageRow};
use crate::{CheckObservation, MemorySubject, PackFailure, PackRequest, PackVerdict, find_pack};

const SCOPE: &str = "completion/grades-final";
const RESULT: &str = "completion/claim-11";
const FACT: &str = "completion.grades-final";
const PROOF: &str = "grades-final";
const DOMAIN: &str = "completion/grades-final";
const EXPECTED: &str =
    "held weak-grade revoked-support unstable-key frontier-open continuation-open";
const CONSTANT_SEAL: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Receipt the loaded grades admit.
#[derive(Debug)]
pub struct ObservedGradesFinal {
    /// Subject the checker invocation and the receipt both name.
    pub subject: String,
    /// `completion/claim-11` for this subject.
    pub result: String,
    /// Reasons the loaded grades command returns.
    pub execution: String,
    /// Catalog that admits the evidence.
    pub catalog: sim_lib_roadmap_runner::ProofCatalog,
    /// Evidence the catalog admits.
    pub admitted: sim_lib_roadmap_runner::AdmittedEvidence,
    /// Checker receipt that binds the admission.
    pub checker: sim_conformance_v02::CheckerReceiptId,
    /// Scope `completion/grades-final`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Runs the loaded grades command and admits its result.
///
/// # Errors
///
/// Returns a refusal when the subject is not a token, the fact carries a
/// completion label, or the loaded command returns another reason.
pub fn observe_grades_final(subject: &str) -> Result<ObservedGradesFinal, PackFailure> {
    let checked = CheckedSubjectId::from_text(subject)
        .map_err(|error| fail("subject-identity", &error.to_string()))?;
    let spec = find_pack("checker/c-evidence")
        .ok_or_else(|| fail("unknown-checker", "checker/c-evidence"))?;
    let binding = spec
        .checker_binding()
        .map_err(|error| fail("invalid-static-binding", &error.to_string()))?;
    let evidence = MemorySubject::default().with(FACT, encode_request(subject)?);
    let verdict = super::check(&PackRequest {
        checker: "checker/c-evidence",
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

/// Re-checks a grades-final receipt against the catalog that admitted it.
///
/// The execution string has to be the loaded reasons. A completion label in
/// that string is refused before the command runs again.
///
/// # Errors
///
/// Returns a refusal when the result, scope, or execution string is no
/// longer the loaded grades.
pub fn bind_grades_final(observation: &ObservedGradesFinal) -> Result<(), PackFailure> {
    if observation.result != RESULT
        || observation.scope != scope_id()?
        || observation.execution.contains("completion/claim-")
    {
        return Err(fail("claim-relabelled", SCOPE));
    }
    let again = load()?;
    if observation.execution != again.executed {
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
    let executed = load()?;
    Ok(vec![observation("completion.execution", executed.executed)])
}

fn admit(
    subject: &str,
    observations: &[CheckObservation],
) -> Result<ObservedGradesFinal, PackFailure> {
    let executed = execution(observations)?;
    let again = load()?;
    if executed != again.executed {
        return Err(fail("evidence-mismatch", FACT));
    }
    let rows = vec![PackageRow {
        name: PROOF.to_owned(),
        version: "0".to_owned(),
        url: format!("git://{PROOF}"),
        rev: again.seal.clone(),
    }];
    let admitted = receipt::admit_for(
        "checker/c-evidence",
        AdmitExecuted {
            subject,
            result: RESULT,
            scope: SCOPE,
            proof: PROOF,
            domain: DOMAIN,
            policy: "policy/grades-final",
            provenance: "provenance/grades-final",
            support: "support/grades-final",
            projection: "projection/grades-final",
            environment: "environment/grades-final",
            leaf: again.seal.as_bytes(),
            source_text: &receipt::package_source(DOMAIN, PROOF, &rows),
            definitions: &receipt::package_definitions(DOMAIN, &rows),
        },
    )?;
    Ok(ObservedGradesFinal {
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

struct Loaded {
    executed: String,
    seal: String,
}

fn load() -> Result<Loaded, PackFailure> {
    let loaded =
        sim_lib_world::grades::execute().map_err(|error| fail("grades-gap", &error.to_string()))?;
    require(&loaded.held, "held", "held")?;
    require(&loaded.weak_grade, "weak-grade", "weak-grade")?;
    require(&loaded.revoked, "revoked-support", "revoked")?;
    require(&loaded.unstable_key, "unstable-key", "unstable-key")?;
    require(&loaded.frontier_open, "frontier-open", "frontier-open")?;
    require(
        &loaded.continuation_open,
        "continuation-open",
        "continuation-open",
    )?;
    let reasons = format!(
        "{} {} {} {} {} {}",
        loaded.held,
        loaded.weak_grade,
        loaded.revoked,
        loaded.unstable_key,
        loaded.frontier_open,
        loaded.continuation_open
    );
    if reasons != EXPECTED {
        return Err(fail("grades-gap", SCOPE));
    }
    let seal = loaded.held_seal;
    let assessed = [
        (seal.as_str(), assessed("held")?),
        (loaded.weak_grade_seal.as_str(), assessed("weak-grade")?),
        (loaded.revoked_seal.as_str(), assessed("revoked")?),
        (loaded.unstable_key_seal.as_str(), assessed("unstable-key")?),
        (
            loaded.frontier_open_seal.as_str(),
            assessed("frontier-open")?,
        ),
        (
            loaded.continuation_open_seal.as_str(),
            assessed("continuation-open")?,
        ),
    ];
    let mut seen = std::collections::BTreeSet::new();
    for (found, digest) in &assessed {
        if !hex_seal(found)
            || *found == CONSTANT_SEAL
            || *found != digest.as_str()
            || !seen.insert(*found)
        {
            return Err(fail("grades-gap", "seal-identity"));
        }
    }
    let executed = format!("{reasons} {seal}");
    if executed.contains("completion/claim-") || !executed.ends_with(&seal) {
        return Err(fail("claim-relabelled", SCOPE));
    }
    Ok(Loaded { executed, seal })
}

fn assessed(case: &str) -> Result<String, PackFailure> {
    sim_lib_world::grades::grades_digest(case)
        .map_err(|error| fail("grades-gap", &error.to_string()))
}

fn hex_seal(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn require(found: &str, intended: &str, label: &str) -> Result<(), PackFailure> {
    if found == intended {
        Ok(())
    } else {
        Err(fail("grades-gap", label))
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
    body.ok_or_else(|| fail("missing-grades-final", FACT))
}

fn execution(observations: &[CheckObservation]) -> Result<&str, PackFailure> {
    observations
        .iter()
        .find(|item| item.key == "completion.execution")
        .map(|item| item.value.as_str())
        .ok_or_else(|| fail("missing-grades-final", FACT))
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
#[path = "evidence_grades_tests.rs"]
mod tests;
