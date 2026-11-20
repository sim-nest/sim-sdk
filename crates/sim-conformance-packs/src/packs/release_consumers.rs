// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Loaded world consumers for the release checker.
//!
//! The check calls `consumers::execute` and the refusal helpers on that
//! same loaded entry. The result is `completion/claim-7` only when the
//! returned projection, diff, why, custody, compose order, and index
//! routes hold, and a direct helper is refused. A caller label is not
//! this result.

use sha2::{Digest, Sha256};
use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};

use super::receipt::{self, AdmitExecuted, PackageRow};
use crate::{CheckObservation, MemorySubject, PackFailure, PackRequest, PackVerdict, find_pack};

const SCOPE: &str = "completion/consumers";
const RESULT: &str = "completion/claim-7";
const FACT: &str = "completion.consumers";
const PROOF: &str = "consumers";
const DOMAIN: &str = "completion/consumers";
const EXPECTED: &str = concat!(
    "loaded-project equal-diff changed-diff why read-only compose ",
    "index-routes helper-refused unknown-refused unrelated-refused"
);
const INDEX: &str = "route/explain-semantic-world-change\nroute/run-world-command";

/// Receipt admitted after the loaded world consumers hold.
#[derive(Debug)]
pub struct ObservedConsumers {
    /// Subject the checker invocation and the receipt both name.
    pub subject: String,
    /// `completion/claim-7` for this subject.
    pub result: String,
    /// Tokens the loaded consumers recorded.
    pub execution: String,
    /// Catalog that admitted the evidence.
    pub catalog: sim_lib_roadmap_runner::ProofCatalog,
    /// Evidence the catalog admitted.
    pub admitted: sim_lib_roadmap_runner::AdmittedEvidence,
    /// Checker receipt that bound the admission.
    pub checker: sim_conformance_v02::CheckerReceiptId,
    /// Scope `completion/consumers`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Runs the loaded world consumers and admits claim 7.
///
/// # Errors
///
/// Returns a refusal when the subject is not a token, the fact carries a
/// completion label, or the loaded entry leaves a field or refusal gap.
pub fn observe_consumers(subject: &str) -> Result<ObservedConsumers, PackFailure> {
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

/// Re-checks a consumers receipt against the catalog that admitted it.
///
/// The execution string has to be the loaded consumers. A claim label in
/// that string is refused before the entry runs again.
///
/// # Errors
///
/// Returns a refusal when the result, scope, or execution string is no
/// longer the loaded consumers.
pub fn bind_consumers(observation: &ObservedConsumers) -> Result<(), PackFailure> {
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
) -> Result<ObservedConsumers, PackFailure> {
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
        policy: "policy/consumers",
        provenance: "provenance/consumers",
        support: "support/consumers",
        projection: "projection/consumers",
        environment: "environment/consumers",
        leaf: executed.as_bytes(),
        source_text: &receipt::package_source(DOMAIN, PROOF, &rows),
        definitions: &receipt::package_definitions(DOMAIN, &rows),
    })?;
    Ok(ObservedConsumers {
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
    let loaded = sim_lib_world::consumers::execute()
        .map_err(|error| fail("consumer-gap", &error.to_string()))?;
    let mut tokens = Vec::new();
    require_text(
        &loaded.project,
        "(kind \"world/public-api-v1\")",
        "project kind",
    )?;
    require_text(
        &loaded.project,
        "(fact \"source/public-api\")",
        "project fact",
    )?;
    require_text(
        &loaded.project,
        "(shape \"world/config-v1\")",
        "project shape",
    )?;
    require_text(
        &loaded.project,
        "(provider \"platform/site/model\")",
        "project provider",
    )?;
    require_text(&loaded.project, "load-receipt-v1", "project receipt")?;
    tokens.push("loaded-project");
    require_text(&loaded.same_diff, "(changed false)", "equal diff")?;
    tokens.push("equal-diff");
    require_text(&loaded.changed_diff, "(changed true)", "changed diff")?;
    tokens.push("changed-diff");
    require_text(&loaded.why, "conclusion/source-api", "why conclusion")?;
    require_text(&loaded.why, "source/public-api", "why fact")?;
    tokens.push("why");
    require_text(&loaded.read_only, "(effects 0)", "read-only effects")?;
    require_text(&loaded.read_only, "(resident true)", "read-only residency")?;
    require_text(
        &loaded.read_only,
        "(capabilities [])",
        "read-only capabilities",
    )?;
    tokens.push("read-only");
    require_after(
        &loaded.compose,
        "(disposition \"current\")",
        "RevocationUnknownOrActive",
    )?;
    require_after(
        &loaded.compose,
        "RevocationUnknownOrActive",
        "checker owner is not live",
    )?;
    require_after(
        &loaded.compose,
        "checker owner is not live",
        "(call \"reopen\")",
    )?;
    require_after(
        &loaded.compose,
        "(call \"reopen\")",
        "(disposition \"current\")",
    )?;
    tokens.push("compose");
    if loaded.index != INDEX {
        return Err(fail("consumer-gap", "index"));
    }
    tokens.push("index-routes");
    match sim_lib_world::consumers::helper_dispatch_is_refused() {
        Ok(()) => return Err(fail("consumer-gap", "helper accepted")),
        Err(error) => {
            let text = error.to_string();
            if !text.contains("project provider missing") || text.contains("load-receipt-v1") {
                return Err(fail("consumer-gap", "helper"));
            }
        }
    }
    tokens.push("helper-refused");
    sim_lib_world::consumers::unknown_fact_is_refused()
        .map_err(|error| fail("consumer-gap", &error.to_string()))?;
    tokens.push("unknown-refused");
    sim_lib_world::consumers::unrelated_index_task_is_refused()
        .map_err(|error| fail("consumer-gap", &error.to_string()))?;
    tokens.push("unrelated-refused");
    let executed = tokens.join(" ");
    if executed.contains("completion/claim-") {
        return Err(fail("claim-relabelled", SCOPE));
    }
    if executed != EXPECTED {
        return Err(fail("consumer-gap", SCOPE));
    }
    Ok(executed)
}

fn require_text(text: &str, needle: &str, label: &str) -> Result<(), PackFailure> {
    if text.contains(needle) {
        Ok(())
    } else {
        Err(fail("consumer-gap", label))
    }
}

fn require_after(text: &str, earlier: &str, later: &str) -> Result<(), PackFailure> {
    let Some(start) = text.find(earlier) else {
        return Err(fail("consumer-gap", earlier));
    };
    if text[start + earlier.len()..].contains(later) {
        Ok(())
    } else {
        Err(fail("consumer-gap", later))
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
    body.ok_or_else(|| fail("missing-consumers", FACT))
}

fn execution(observations: &[CheckObservation]) -> Result<&str, PackFailure> {
    observations
        .iter()
        .find(|item| item.key == "completion.execution")
        .map(|item| item.value.as_str())
        .ok_or_else(|| fail("missing-consumers", FACT))
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
#[path = "release_consumers_tests.rs"]
mod tests;
