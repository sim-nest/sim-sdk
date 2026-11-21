// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Domain comparison for the evidence checker.
//!
//! The check calls the loaded domains command. The result is
//! `completion/claim-14` only when model delivery matches its own
//! observation, operator release matches its own observation, and a
//! row from one realization is refused as the observation of the
//! other. The receipt's proof leaf is the model seal that command
//! returns. The real seal and the substitution seal are different
//! seals in the same execution. A constant digest is not a seal. A
//! projection or identity-grade fact is another fact and is refused.
//! A caller label is not this result.

use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};

use crate::packs::release::receipt::{self, AdmitExecuted, PackageRow};
use crate::{CheckObservation, MemorySubject, PackFailure, PackRequest, PackVerdict, find_pack};

const SCOPE: &str = "completion/domains-final";
const RESULT: &str = "completion/claim-14";
const FACT: &str = "completion.domains-final";
const PROOF: &str = "domains-final";
const DOMAIN: &str = "completion/domains-final";
const EXPECTED: &str = "aligned aligned refused";
const CONSTANT_SEAL: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Receipt the loaded domains admit.
#[derive(Debug)]
pub struct ObservedDomainsFinal {
    /// Subject the checker invocation and the receipt both name.
    pub subject: String,
    /// `completion/claim-14` for this subject.
    pub result: String,
    /// Reasons and seals the loaded domains command returns.
    pub execution: String,
    /// Catalog that admits the evidence.
    pub catalog: sim_lib_roadmap_runner::ProofCatalog,
    /// Evidence the catalog admits.
    pub admitted: sim_lib_roadmap_runner::AdmittedEvidence,
    /// Checker receipt that binds the admission.
    pub checker: sim_conformance_v02::CheckerReceiptId,
    /// Scope `completion/domains-final`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Runs the loaded domains command and admits its result.
///
/// # Errors
///
/// Returns a refusal when the subject is not a token, the fact carries a
/// completion label, or the loaded command returns another reason.
pub fn observe_domains_final(subject: &str) -> Result<ObservedDomainsFinal, PackFailure> {
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

/// Re-checks a domains-final receipt against the catalog that admitted it.
///
/// The execution string has to be the loaded reasons and seals. A completion
/// label in that string is refused before the command runs again.
///
/// # Errors
///
/// Returns a refusal when the result, scope, or execution string is no
/// longer the loaded domains.
pub fn bind_domains_final(observation: &ObservedDomainsFinal) -> Result<(), PackFailure> {
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
) -> Result<ObservedDomainsFinal, PackFailure> {
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
            policy: "policy/domains-final",
            provenance: "provenance/domains-final",
            support: "support/domains-final",
            projection: "projection/domains-final",
            environment: "environment/domains-final",
            leaf: again.seal.as_bytes(),
            source_text: &receipt::package_source(DOMAIN, PROOF, &rows),
            definitions: &receipt::package_definitions(DOMAIN, &rows),
        },
    )?;
    Ok(ObservedDomainsFinal {
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
    let loaded = sim_lib_world::domains::execute()
        .map_err(|error| fail("domains-gap", &error.to_string()))?;
    require(&loaded.model, "aligned", "model")?;
    require(&loaded.real, "aligned", "real")?;
    require(&loaded.substitution, "refused", "substitution")?;
    let reasons = format!("{} {} {}", loaded.model, loaded.real, loaded.substitution);
    if reasons != EXPECTED {
        return Err(fail("domains-gap", SCOPE));
    }
    let seals = [
        loaded.model_seal.as_str(),
        loaded.real_seal.as_str(),
        loaded.substitution_seal.as_str(),
    ];
    let mut seen = std::collections::BTreeSet::new();
    for found in seals {
        if !hex_seal(found) || found == CONSTANT_SEAL || !seen.insert(found) {
            return Err(fail("domains-gap", "seal-identity"));
        }
    }
    let seal = loaded.model_seal;
    let executed = format!(
        "{reasons} {seal} {} {}",
        loaded.real_seal, loaded.substitution_seal
    );
    if executed.contains("completion/claim-") || !executed.contains(&seal) {
        return Err(fail("claim-relabelled", SCOPE));
    }
    Ok(Loaded { executed, seal })
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
        Err(fail("domains-gap", label))
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
    body.ok_or_else(|| fail("missing-domains-final", FACT))
}

fn execution(observations: &[CheckObservation]) -> Result<&str, PackFailure> {
    observations
        .iter()
        .find(|item| item.key == "completion.execution")
        .map(|item| item.value.as_str())
        .ok_or_else(|| fail("missing-domains-final", FACT))
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
#[path = "evidence_domains_tests.rs"]
mod tests;
