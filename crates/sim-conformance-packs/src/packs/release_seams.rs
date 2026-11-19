// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Portable seam completion for one subject.
//!
//! The pack runs the consumer-seams specimen and the foreign-project
//! specimen. The freeze digest must be the compiled executor closure.
//! A matching digest still runs both specimens. The result is
//! `completion/claim-19` only when revision pages, one direction
//! transition, select/begin separation, exact lineage, and advice
//! refusal all hold, and the foreign-project admission names this
//! subject. A caller boolean is not that result.

use sha2::{Digest, Sha256};
use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};
use sim_lib_roadmap_runner::{AdmittedEvidence, ProofCatalog};

use super::receipt::{self, AdmitExecuted, PackageRow};
use crate::packs::portability::consumer_seams::{self, SeamReport};
use crate::{
    CheckObservation, MemorySubject, PackFailure, PackRequest, PackVerdict,
    compiled_executor_closure, find_pack,
};

const SCOPE: &str = "completion/seams";
const RESULT: &str = "completion/claim-19";
const FACT: &str = "completion.seams";
const PROOF: &str = "portable-seams";
const DOMAIN: &str = "completion/seams";

/// Receipt admitted after both portable seam specimens pass.
#[derive(Debug)]
pub struct ObservedPortableSeams {
    /// Subject the checker invocation and the receipt both name.
    pub subject: String,
    /// `completion/claim-19` for this subject.
    pub result: String,
    /// Compiled executor closure the specimens ran against.
    pub freeze: String,
    /// Catalog that admitted the evidence.
    pub catalog: ProofCatalog,
    /// Evidence the catalog admitted.
    pub admitted: AdmittedEvidence,
    /// Checker receipt that bound the admission.
    pub checker: sim_conformance_v02::CheckerReceiptId,
    /// Scope `completion/seams`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Runs both seam specimens and admits claim 19.
///
/// `freeze` is the closure digest the caller read. It is accepted only
/// when it is the compiled executor closure. The specimens still run.
///
/// # Errors
///
/// Returns a refusal when the digest differs, either specimen refuses,
/// or the catalog rejects the constructed receipt.
pub fn observe_portable_seams(
    subject: &str,
    freeze: &str,
) -> Result<ObservedPortableSeams, PackFailure> {
    let checked = CheckedSubjectId::from_text(subject)
        .map_err(|error| fail("subject-identity", &error.to_string()))?;
    let spec = find_pack("checker/c-release")
        .ok_or_else(|| fail("unknown-checker", "checker/c-release"))?;
    let binding = spec
        .checker_binding()
        .map_err(|error| fail("invalid-static-binding", &error.to_string()))?;
    let evidence = MemorySubject::default().with(FACT, encode_request(subject, freeze)?);
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
    admit(subject, freeze, &observations)
}

/// Re-checks a seam receipt against the catalog that admitted it.
///
/// # Errors
///
/// Returns a refusal when the result or scope is no longer claim 19.
pub fn bind_portable_seams(observation: &ObservedPortableSeams) -> Result<(), PackFailure> {
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
    let (subject, freeze) = decode_request(body)?;
    let bound =
        CheckedSubjectId::from_text(&subject).map_err(|_| fail("subject-mismatch", FACT))?;
    if receipt::render(bound.content_id()) != request.subject {
        return Err(fail("subject-mismatch", FACT));
    }
    let executed = execute(&subject, &freeze)?;
    Ok(vec![observation("completion.execution", executed)])
}

fn admit(
    subject: &str,
    freeze: &str,
    observations: &[CheckObservation],
) -> Result<ObservedPortableSeams, PackFailure> {
    let executed = execution(observations)?;
    let again = execute(subject, freeze)?;
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
        policy: "policy/portable-seams",
        provenance: "provenance/portable-seams",
        support: "support/portable-seams",
        projection: "projection/portable-seams",
        environment: "environment/portable-seams",
        leaf: executed.as_bytes(),
        source_text: &receipt::package_source(DOMAIN, PROOF, &rows),
        definitions: &receipt::package_definitions(DOMAIN, &rows),
    })?;
    Ok(ObservedPortableSeams {
        subject: subject.to_owned(),
        result: RESULT.to_owned(),
        freeze: freeze.to_owned(),
        catalog: admitted.catalog,
        admitted: admitted.admitted,
        checker: admitted.checker,
        scope: admitted.scope,
        revocation: admitted.revocation,
    })
}

fn execute(subject: &str, freeze: &str) -> Result<String, PackFailure> {
    if freeze != compiled_executor_closure() {
        return Err(fail("freeze-mismatch", SCOPE));
    }
    let report = consumer_seams::qualify_subject(freeze, freeze, subject)
        .map_err(|fault| fail(fault.code(), SCOPE))?;
    require_report(&report)?;
    let foreign = crate::executor::foreign_project(subject)?;
    require_foreign(subject, &foreign)?;
    Ok(encode_execution(subject, freeze, &report, &foreign))
}

fn require_report(report: &SeamReport) -> Result<(), PackFailure> {
    if report.first_page < 1
        || report.retained_revision.is_empty()
        || report.transitions != 1
        || report.stale_effects != 0
        || report.effects != 1
        || report.hot_delta != 0
        || report.stale_edges != 1
        || report.unrelated_proof.is_empty()
        || report.source_bound < 1
        || report.providers.is_empty()
        || report.providers.iter().any(|name| name.contains("linux"))
    {
        return Err(fail("seam-gap", SCOPE));
    }
    Ok(())
}

fn require_foreign(subject: &str, foreign: &[CheckObservation]) -> Result<(), PackFailure> {
    let escape = foreign_value(foreign, "source.escape")?;
    let head = foreign_value(foreign, "source.head")?;
    let oversize = foreign_value(foreign, "source.oversize")?;
    let owner = foreign_value(foreign, "source.subject")?;
    if escape == "0" || head != "head-mismatch" || oversize != "oversize" || owner != subject {
        return Err(fail("foreign-project", SCOPE));
    }
    Ok(())
}

fn foreign_value<'a>(foreign: &'a [CheckObservation], key: &str) -> Result<&'a str, PackFailure> {
    foreign
        .iter()
        .find(|item| item.key == key)
        .map(|item| item.value.as_str())
        .ok_or_else(|| fail("foreign-project", key))
}

fn encode_request(subject: &str, freeze: &str) -> Result<String, PackFailure> {
    if !token(subject) || !hex64(freeze) {
        return Err(fail("bad-record", FACT));
    }
    Ok(format!("subject {subject}\nfreeze {freeze}\n"))
}

fn decode_request(body: &str) -> Result<(String, String), PackFailure> {
    let mut subject = None;
    let mut freeze = None;
    for line in body.lines() {
        if line.is_empty() {
            return Err(fail("bad-record", FACT));
        }
        let Some((key, value)) = line.split_once(' ') else {
            return Err(fail("bad-record", FACT));
        };
        if value.contains(' ') {
            return Err(fail("bad-record", FACT));
        }
        match key {
            "subject" if subject.is_none() && token(value) => subject = Some(value.to_owned()),
            "freeze" if freeze.is_none() && hex64(value) => freeze = Some(value.to_owned()),
            _ => return Err(fail("bad-record", FACT)),
        }
    }
    match (subject, freeze) {
        (Some(subject), Some(freeze)) => Ok((subject, freeze)),
        _ => Err(fail("bad-record", FACT)),
    }
}

fn encode_execution(
    subject: &str,
    freeze: &str,
    report: &SeamReport,
    foreign: &[CheckObservation],
) -> String {
    let mut text = String::new();
    receipt::push_field(&mut text, subject);
    receipt::push_field(&mut text, freeze);
    receipt::push_field(&mut text, &report.first_page.to_string());
    receipt::push_field(&mut text, &report.retained_revision);
    receipt::push_field(&mut text, &report.transitions.to_string());
    receipt::push_field(&mut text, &report.stale_effects.to_string());
    receipt::push_field(&mut text, &report.effects.to_string());
    receipt::push_field(&mut text, &report.hot_delta.to_string());
    receipt::push_field(&mut text, &report.stale_edges.to_string());
    receipt::push_field(&mut text, &report.unrelated_proof);
    receipt::push_field(&mut text, &report.source_bound.to_string());
    receipt::push_field(&mut text, &report.providers.len().to_string());
    for provider in &report.providers {
        receipt::push_field(&mut text, provider);
    }
    receipt::push_field(&mut text, &foreign.len().to_string());
    for item in foreign {
        receipt::push_field(&mut text, &item.key);
        receipt::push_field(&mut text, &item.value);
    }
    text
}

fn one_fact<'a>(request: &'a PackRequest<'a>) -> Result<&'a str, PackFailure> {
    let mut body = None;
    for key in request.evidence.fact_keys() {
        match key {
            FACT => body = request.evidence.fact(key),
            other => return Err(fail("unexpected-fact", other)),
        }
    }
    body.ok_or_else(|| fail("missing-seams", FACT))
}

fn execution(observations: &[CheckObservation]) -> Result<&str, PackFailure> {
    observations
        .iter()
        .find(|item| item.key == "completion.execution")
        .map(|item| item.value.as_str())
        .ok_or_else(|| fail("missing-seams", FACT))
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
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
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
#[path = "release_seams_tests.rs"]
mod tests;
