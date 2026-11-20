// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Final source universe for the release checker.
//!
//! The check calls the loaded source command. The result is
//! `completion/claim-10` only when the captured universe is complete and
//! each removed class, an unread policy, an unbound fact, and the exact
//! read set return their own reason. The receipt's proof leaf and package
//! revision are the exact seal that command returned. The unread-policy
//! case keeps that identity. The changed-fact and changed-policy cases
//! move it. A constant digest is not that seal. A caller label is not
//! this result.

use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};

use super::receipt::{self, AdmitExecuted, PackageRow};
use crate::{CheckObservation, MemorySubject, PackFailure, PackRequest, PackVerdict, find_pack};

const SCOPE: &str = "completion/source-universe";
const RESULT: &str = "completion/claim-10";
const FACT: &str = "completion.source-universe";
const PROOF: &str = "source-universe";
const DOMAIN: &str = "completion/source-universe";
const EXPECTED: &str = "complete missing-target missing-resolver missing-checksum missing-feature missing-config missing-toolchain missing-nested unread-policy unbound-fact exact";
const CONSTANT_SEAL: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Receipt admitted after the loaded source universe holds.
#[derive(Debug)]
pub struct ObservedSourceUniverse {
    /// Subject the checker invocation and the receipt both name.
    pub subject: String,
    /// `completion/claim-10` for this subject.
    pub result: String,
    /// Reasons the loaded source command returned.
    pub execution: String,
    /// Catalog that admitted the evidence.
    pub catalog: sim_lib_roadmap_runner::ProofCatalog,
    /// Evidence the catalog admitted.
    pub admitted: sim_lib_roadmap_runner::AdmittedEvidence,
    /// Checker receipt that bound the admission.
    pub checker: sim_conformance_v02::CheckerReceiptId,
    /// Scope `completion/source-universe`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Runs the loaded source universe and admits claim 10.
///
/// # Errors
///
/// Returns a refusal when the subject is not a token, the fact carries a
/// completion label, or the loaded command returns another reason.
pub fn observe_source_universe(subject: &str) -> Result<ObservedSourceUniverse, PackFailure> {
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

/// Re-checks a source-universe receipt against the catalog that admitted it.
///
/// The execution string has to be the loaded reasons. A claim label in
/// that string is refused before the command runs again.
///
/// # Errors
///
/// Returns a refusal when the result, scope, or execution string is no
/// longer the loaded source universe.
pub fn bind_source_universe(observation: &ObservedSourceUniverse) -> Result<(), PackFailure> {
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
) -> Result<ObservedSourceUniverse, PackFailure> {
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
    let admitted = receipt::admit_executed(AdmitExecuted {
        subject,
        result: RESULT,
        scope: SCOPE,
        proof: PROOF,
        domain: DOMAIN,
        policy: "policy/source-universe",
        provenance: "provenance/source-universe",
        support: "support/source-universe",
        projection: "projection/source-universe",
        environment: "environment/source-universe",
        leaf: again.seal.as_bytes(),
        source_text: &receipt::package_source(DOMAIN, PROOF, &rows),
        definitions: &receipt::package_definitions(DOMAIN, &rows),
    })?;
    Ok(ObservedSourceUniverse {
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
    let loaded = sim_lib_world::universe::execute()
        .map_err(|error| fail("source-gap", &error.to_string()))?;
    require(&loaded.complete, "complete", "complete")?;
    require(&loaded.missing_target, "missing-target", "target")?;
    require(&loaded.missing_resolver, "missing-resolver", "resolver")?;
    require(&loaded.missing_checksum, "missing-checksum", "checksum")?;
    require(&loaded.missing_feature, "missing-feature", "feature")?;
    require(&loaded.missing_config, "missing-config", "config")?;
    require(&loaded.missing_toolchain, "missing-toolchain", "toolchain")?;
    require(&loaded.missing_nested, "missing-nested", "nested")?;
    require(&loaded.unread_policy, "unread-policy", "unread")?;
    require(&loaded.unbound_fact, "unbound-fact", "unbound")?;
    require(&loaded.exact, "exact", "exact")?;
    let reasons = format!(
        "{} {} {} {} {} {} {} {} {} {} {}",
        loaded.complete,
        loaded.missing_target,
        loaded.missing_resolver,
        loaded.missing_checksum,
        loaded.missing_feature,
        loaded.missing_config,
        loaded.missing_toolchain,
        loaded.missing_nested,
        loaded.unread_policy,
        loaded.unbound_fact,
        loaded.exact
    );
    if reasons != EXPECTED {
        return Err(fail("source-gap", SCOPE));
    }
    let seal = loaded.exact_seal;
    let exact_assessed = assessed("exact")?;
    let unread_assessed = assessed("unread-policy")?;
    let fact_assessed = assessed("changed-fact")?;
    let policy_assessed = assessed("changed-policy")?;
    if !hex_seal(&seal)
        || loaded.unread_seal != seal
        || seal != exact_assessed
        || unread_assessed != exact_assessed
        || loaded.changed_fact_seal != fact_assessed
        || loaded.changed_policy_seal != policy_assessed
        || fact_assessed == seal
        || policy_assessed == seal
        || fact_assessed == policy_assessed
        || loaded.changed_fact_seal == seal
        || loaded.changed_policy_seal == seal
        || loaded.changed_fact_seal == loaded.changed_policy_seal
        || loaded.changed_fact_seal == CONSTANT_SEAL
        || loaded.changed_policy_seal == CONSTANT_SEAL
        || seal == CONSTANT_SEAL
    {
        return Err(fail("source-gap", "seal-identity"));
    }
    let executed = format!("{reasons} {seal}");
    if executed.contains("completion/claim-") || !executed.ends_with(&seal) {
        return Err(fail("claim-relabelled", SCOPE));
    }
    Ok(Loaded { executed, seal })
}

fn assessed(case: &str) -> Result<String, PackFailure> {
    sim_lib_world::universe::source_digest(case)
        .map_err(|error| fail("source-gap", &error.to_string()))
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
        Err(fail("source-gap", label))
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
    body.ok_or_else(|| fail("missing-source-universe", FACT))
}

fn execution(observations: &[CheckObservation]) -> Result<&str, PackFailure> {
    observations
        .iter()
        .find(|item| item.key == "completion.execution")
        .map(|item| item.value.as_str())
        .ok_or_else(|| fail("missing-source-universe", FACT))
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
#[path = "release_universe_tests.rs"]
mod tests;
