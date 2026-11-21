// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Settlement comparison for the evidence checker.
//!
//! The check calls the loaded effects command. The result is
//! `completion/claim-15` only when the settled case is settled, the
//! dispatched, divergent, uncertain, and leased cases each stay in that
//! state, a replay is refused, and the historical case is held. The
//! receipt's proof leaf is the settled seal that command returns. The
//! other six seals are different seals in the same execution. A constant
//! digest is not a seal. A projection or identity-grade fact is another
//! fact and is refused. The operate, conduct, and schedule commands
//! remain their own commands. A caller label is not this result.

use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};

use crate::packs::release::receipt::{self, AdmitExecuted, PackageRow};
use crate::{CheckObservation, MemorySubject, PackFailure, PackRequest, PackVerdict, find_pack};

const SCOPE: &str = "completion/effects-final";
const RESULT: &str = "completion/claim-15";
const FACT: &str = "completion.effects-final";
const PROOF: &str = "effects-final";
const DOMAIN: &str = "completion/effects-final";
const EXPECTED: &str = "settled dispatched divergent uncertain leased refused held";
const CONSTANT_SEAL: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Receipt the loaded effects admit.
#[derive(Debug)]
pub struct ObservedEffectsFinal {
    /// Subject the checker invocation and the receipt both name.
    pub subject: String,
    /// `completion/claim-15` for this subject.
    pub result: String,
    /// Reasons and seals the loaded effects command returns.
    pub execution: String,
    /// Catalog that admits the evidence.
    pub catalog: sim_lib_roadmap_runner::ProofCatalog,
    /// Evidence the catalog admits.
    pub admitted: sim_lib_roadmap_runner::AdmittedEvidence,
    /// Checker receipt that binds the admission.
    pub checker: sim_conformance_v02::CheckerReceiptId,
    /// Scope `completion/effects-final`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Runs the loaded effects command and admits its result.
///
/// # Errors
///
/// Returns a refusal when the subject is not a token, the fact carries a
/// completion label, or the loaded command returns another reason.
pub fn observe_effects_final(subject: &str) -> Result<ObservedEffectsFinal, PackFailure> {
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

/// Re-checks an effects-final receipt against the catalog that admitted it.
///
/// The execution string has to be the loaded reasons and seals. A completion
/// label in that string is refused before the command runs again.
///
/// # Errors
///
/// Returns a refusal when the result, scope, or execution string is no
/// longer the loaded effects.
pub fn bind_effects_final(observation: &ObservedEffectsFinal) -> Result<(), PackFailure> {
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
) -> Result<ObservedEffectsFinal, PackFailure> {
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
            policy: "policy/effects-final",
            provenance: "provenance/effects-final",
            support: "support/effects-final",
            projection: "projection/effects-final",
            environment: "environment/effects-final",
            leaf: again.seal.as_bytes(),
            source_text: &receipt::package_source(DOMAIN, PROOF, &rows),
            definitions: &receipt::package_definitions(DOMAIN, &rows),
        },
    )?;
    Ok(ObservedEffectsFinal {
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
    let loaded = sim_lib_world::effects::execute()
        .map_err(|error| fail("effects-gap", &error.to_string()))?;
    require(&loaded.settled, "settled", "settled")?;
    require(&loaded.dispatched, "dispatched", "dispatched")?;
    require(&loaded.divergent, "divergent", "divergent")?;
    require(&loaded.uncertain, "uncertain", "uncertain")?;
    require(&loaded.leased, "leased", "leased")?;
    require(&loaded.replayed, "refused", "replayed")?;
    require(&loaded.historical, "held", "historical")?;
    let reasons = format!(
        "{} {} {} {} {} {} {}",
        loaded.settled,
        loaded.dispatched,
        loaded.divergent,
        loaded.uncertain,
        loaded.leased,
        loaded.replayed,
        loaded.historical
    );
    if reasons != EXPECTED {
        return Err(fail("effects-gap", SCOPE));
    }
    let seals = [
        loaded.settled_seal.as_str(),
        loaded.dispatched_seal.as_str(),
        loaded.divergent_seal.as_str(),
        loaded.uncertain_seal.as_str(),
        loaded.leased_seal.as_str(),
        loaded.replayed_seal.as_str(),
        loaded.historical_seal.as_str(),
    ];
    let mut seen = std::collections::BTreeSet::new();
    for found in seals {
        if !hex_seal(found) || found == CONSTANT_SEAL || !seen.insert(found) {
            return Err(fail("effects-gap", "seal-identity"));
        }
    }
    let seal = loaded.settled_seal;
    let executed = format!(
        "{reasons} {seal} {} {} {} {} {} {}",
        loaded.dispatched_seal,
        loaded.divergent_seal,
        loaded.uncertain_seal,
        loaded.leased_seal,
        loaded.replayed_seal,
        loaded.historical_seal
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
        Err(fail("effects-gap", label))
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
    body.ok_or_else(|| fail("missing-effects-final", FACT))
}

fn execution(observations: &[CheckObservation]) -> Result<&str, PackFailure> {
    observations
        .iter()
        .find(|item| item.key == "completion.execution")
        .map(|item| item.value.as_str())
        .ok_or_else(|| fail("missing-effects-final", FACT))
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
#[path = "evidence_effects_tests.rs"]
mod tests;
