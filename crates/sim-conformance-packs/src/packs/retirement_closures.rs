// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Final production and conformance closures for the retirement checker.
//!
//! The check calls the loaded closures command. The result is
//! `completion/claim-3` only when the closed case holds and the executable,
//! reader, schema, oracle, parser, and live-fact cases each return their own
//! reason. The receipt's proof leaf and package revision are the exact seal
//! that command returns. The other six cases move that seal. A constant digest
//! is not that seal. A retirement guard is another fact and is refused. A
//! caller label is not this result.

use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};

use crate::packs::release::receipt::{self, AdmitExecuted, PackageRow};
use crate::{CheckObservation, MemorySubject, PackFailure, PackRequest, PackVerdict, find_pack};

const SCOPE: &str = "completion/closure-final";
const RESULT: &str = "completion/claim-3";
const FACT: &str = "completion.closure-final";
const PROOF: &str = "closure-final";
const DOMAIN: &str = "completion/closure-final";
const EXPECTED: &str = "closed executable reader schema oracle parser live-fact";
const CONSTANT_SEAL: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Receipt the loaded closures admit.
#[derive(Debug)]
pub struct ObservedClosureFinal {
    /// Subject the checker invocation and the receipt both name.
    pub subject: String,
    /// `completion/claim-3` for this subject.
    pub result: String,
    /// Reasons the loaded closures command returns.
    pub execution: String,
    /// Catalog that admits the evidence.
    pub catalog: sim_lib_roadmap_runner::ProofCatalog,
    /// Evidence the catalog admits.
    pub admitted: sim_lib_roadmap_runner::AdmittedEvidence,
    /// Checker receipt that binds the admission.
    pub checker: sim_conformance_v02::CheckerReceiptId,
    /// Scope `completion/closure-final`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Runs the loaded closures command and admits claim 3.
///
/// # Errors
///
/// Returns a refusal when the subject is not a token, the fact carries a
/// completion label, or the loaded command returns another reason.
pub fn observe_closure_final(subject: &str) -> Result<ObservedClosureFinal, PackFailure> {
    let checked = CheckedSubjectId::from_text(subject)
        .map_err(|error| fail("subject-identity", &error.to_string()))?;
    let spec = find_pack("checker/c-v3").ok_or_else(|| fail("unknown-checker", "checker/c-v3"))?;
    let binding = spec
        .checker_binding()
        .map_err(|error| fail("invalid-static-binding", &error.to_string()))?;
    let evidence = MemorySubject::default().with(FACT, encode_request(subject)?);
    let verdict = super::check(&PackRequest {
        checker: "checker/c-v3",
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

/// Re-checks a closure-final receipt against the catalog that admitted it.
///
/// The execution string has to be the loaded reasons. A claim label in
/// that string is refused before the command runs again.
///
/// # Errors
///
/// Returns a refusal when the result, scope, or execution string is no
/// longer the loaded closures.
pub fn bind_closure_final(observation: &ObservedClosureFinal) -> Result<(), PackFailure> {
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
) -> Result<ObservedClosureFinal, PackFailure> {
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
        "checker/c-v3",
        AdmitExecuted {
            subject,
            result: RESULT,
            scope: SCOPE,
            proof: PROOF,
            domain: DOMAIN,
            policy: "policy/closure-final",
            provenance: "provenance/closure-final",
            support: "support/closure-final",
            projection: "projection/closure-final",
            environment: "environment/closure-final",
            leaf: again.seal.as_bytes(),
            source_text: &receipt::package_source(DOMAIN, PROOF, &rows),
            definitions: &receipt::package_definitions(DOMAIN, &rows),
        },
    )?;
    Ok(ObservedClosureFinal {
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
    let loaded = sim_lib_world::closures::execute()
        .map_err(|error| fail("closures-gap", &error.to_string()))?;
    require(&loaded.closed, "closed", "closed")?;
    require(&loaded.executable, "executable", "executable")?;
    require(&loaded.reader, "reader", "reader")?;
    require(&loaded.schema, "schema", "schema")?;
    require(&loaded.oracle, "oracle", "oracle")?;
    require(&loaded.parser, "parser", "parser")?;
    require(&loaded.live_fact, "live-fact", "live-fact")?;
    let reasons = format!(
        "{} {} {} {} {} {} {}",
        loaded.closed,
        loaded.executable,
        loaded.reader,
        loaded.schema,
        loaded.oracle,
        loaded.parser,
        loaded.live_fact
    );
    if reasons != EXPECTED {
        return Err(fail("closures-gap", SCOPE));
    }
    let seal = loaded.closed_seal;
    let assessed = [
        (seal.as_str(), assessed("closed")?),
        (loaded.executable_seal.as_str(), assessed("executable")?),
        (loaded.reader_seal.as_str(), assessed("reader")?),
        (loaded.schema_seal.as_str(), assessed("schema")?),
        (loaded.oracle_seal.as_str(), assessed("oracle")?),
        (loaded.parser_seal.as_str(), assessed("parser")?),
        (loaded.live_fact_seal.as_str(), assessed("live-fact")?),
    ];
    let mut seen = std::collections::BTreeSet::new();
    for (found, digest) in &assessed {
        if !hex_seal(found)
            || *found == CONSTANT_SEAL
            || *found != digest.as_str()
            || !seen.insert(*found)
        {
            return Err(fail("closures-gap", "seal-identity"));
        }
    }
    let executed = format!("{reasons} {seal}");
    if executed.contains("completion/claim-") || !executed.ends_with(&seal) {
        return Err(fail("claim-relabelled", SCOPE));
    }
    Ok(Loaded { executed, seal })
}

fn assessed(case: &str) -> Result<String, PackFailure> {
    sim_lib_world::closures::closures_digest(case)
        .map_err(|error| fail("closures-gap", &error.to_string()))
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
        Err(fail("closures-gap", label))
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
    body.ok_or_else(|| fail("missing-closure-final", FACT))
}

fn execution(observations: &[CheckObservation]) -> Result<&str, PackFailure> {
    observations
        .iter()
        .find(|item| item.key == "completion.execution")
        .map(|item| item.value.as_str())
        .ok_or_else(|| fail("missing-closure-final", FACT))
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
#[path = "retirement_closures_tests.rs"]
mod tests;
