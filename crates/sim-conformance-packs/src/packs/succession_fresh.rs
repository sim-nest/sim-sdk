// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Fresh-command observation for the installed later change.
//!
//! The checker reads that change through the public fresh command and admits
//! the resulting record. A publication flag, a claim label, and the
//! construction specimen are refused. This module does not derive a seal.

use sha2::{Digest, Sha256};
use sim_conformance_v02::{CheckScopeId, CheckedSubjectId};
use sim_kernel::Datum;

use crate::packs::release::receipt::{self, AdmitExecuted, PackageRow};
use crate::{CheckObservation, MemorySubject, PackFailure, PackRequest, PackVerdict, find_pack};

pub(super) const SCOPE: &str = "completion/fresh-command";
const RESULT: &str = "completion/claim-23";
const PROOF: &str = "fresh-command";
const DOMAIN: &str = "completion/fresh-command";
const EXPORT: &str = "\
export world/succession-export-v1
package sim-run 0.3.10 registry
package sim-lib-world 0.1.29 registry
project harbor
input provider/widget observed widget-body
input remote/hostile unavailable -
";
const CHANGE: &str = "later-0310 harbor provider/marker observed later-body";

const KEYS: &[&str] = &[
    "fresh.change",
    "fresh.project",
    "fresh.seal",
    "fresh.succession",
    "fresh.linux",
    "fresh.converge-second",
    "fresh.deliver-second",
    "fresh.effects",
    "fresh.subprocesses",
    "fresh.resident",
];

/// Receipt the fresh command admits for one subject.
#[derive(Debug)]
pub struct ObservedFreshCommand {
    /// Subject the checker invocation and the receipt both name.
    pub subject: String,
    /// `completion/claim-23` for this subject.
    pub result: String,
    /// Joined fresh-command fields. This string is not the claim.
    pub execution: String,
    /// Catalog that admits the evidence.
    pub catalog: sim_lib_roadmap_runner::ProofCatalog,
    /// Evidence the catalog admits.
    pub admitted: sim_lib_roadmap_runner::AdmittedEvidence,
    /// Checker receipt that binds the admission.
    pub checker: sim_conformance_v02::CheckerReceiptId,
    /// Scope `completion/fresh-command`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: sim_conformance_v02::RevocationStatus,
}

/// Reads the installed later change and admits that fresh command.
///
/// # Errors
///
/// Returns a refusal when the subject is a claim label, the fresh command
/// refuses, or the record is not the installed later change.
pub fn observe_fresh_command(subject: &str) -> Result<ObservedFreshCommand, PackFailure> {
    if !token(subject) {
        return Err(fail("bad-record", "subject"));
    }
    let checked = CheckedSubjectId::from_text(subject)
        .map_err(|error| fail("subject-identity", &error.to_string()))?;
    let spec = find_pack("checker/c-succeed")
        .ok_or_else(|| fail("unknown-checker", "checker/c-succeed"))?;
    let binding = spec
        .checker_binding()
        .map_err(|error| fail("invalid-static-binding", &error.to_string()))?;
    let evidence = MemorySubject::default();
    let rendered_subject = receipt::render(checked.content_id());
    let rendered_binding = receipt::render(binding.id().content_id());
    let verdict = super::check(&PackRequest {
        checker: "checker/c-succeed",
        binding: &rendered_binding,
        subject: &rendered_subject,
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

pub(super) fn observations(
    request: &PackRequest<'_>,
) -> Result<Vec<CheckObservation>, PackFailure> {
    if request.subject.contains("completion/claim-") {
        return Err(fail("bad-record", "subject"));
    }
    if let Some(key) = request.evidence.fact_keys().into_iter().next() {
        return Err(fail("unexpected-fact", key));
    }
    let read = read_change(EXPORT, CHANGE)?;
    if read.execution.contains("completion/claim-") {
        return Err(fail("claim-relabelled", SCOPE));
    }
    Ok(KEYS
        .iter()
        .zip(read.parts.iter())
        .map(|(key, value)| CheckObservation {
            key: (*key).to_owned(),
            value: value.clone(),
        })
        .collect())
}

#[derive(Debug)]
struct Read {
    parts: Vec<String>,
    execution: String,
    digest: Vec<u8>,
}

fn read_change(export: &str, change: &str) -> Result<Read, PackFailure> {
    let record = sim_lib_world::run_fresh_command(export, change)
        .map_err(|error| fail("fresh-refused", &error.to_string()))?;
    let qualified = qualify(&record)?;
    let bytes = record
        .canonical_bytes()
        .map_err(|_| fail("fresh-unqualified", "record"))?;
    let digest = Sha256::digest(&bytes).to_vec();
    if digest.iter().all(|byte| *byte == 0) {
        return Err(fail("unstable", "seal"));
    }
    let execution = join(&qualified);
    if execution.contains("completion/claim-") {
        return Err(fail("claim-relabelled", SCOPE));
    }
    Ok(Read {
        parts: qualified,
        execution,
        digest,
    })
}

fn qualify(record: &Datum) -> Result<Vec<String>, PackFailure> {
    let change = text(record, "change")?;
    let project = text(record, "project")?;
    if change.is_empty() || project.is_empty() || change == "shade-1" {
        return Err(fail("fresh-unqualified", "change"));
    }
    if change != "later-0310" || project != "harbor" {
        return Err(fail("fresh-unqualified", "change"));
    }
    if text(record, "seal")? != "unclaimed" {
        return Err(fail("fresh-unqualified", "seal"));
    }
    if text(record, "succession")? != "portable" {
        return Err(fail("fresh-unqualified", "succession"));
    }
    if text(record, "linux")? != "absent" {
        return Err(fail("fresh-unqualified", "linux"));
    }
    for name in [
        "converge-second",
        "deliver-second",
        "effects",
        "subprocesses",
    ] {
        if number(record, name)? != "0" {
            return Err(fail("fresh-unqualified", name));
        }
    }
    if !bool_field(record, "resident")? {
        return Err(fail("fresh-unqualified", "resident"));
    }
    Ok(vec![
        change,
        project,
        "unclaimed".to_owned(),
        "portable".to_owned(),
        "absent".to_owned(),
        "0".to_owned(),
        "0".to_owned(),
        "0".to_owned(),
        "0".to_owned(),
        "true".to_owned(),
    ])
}

fn text(record: &Datum, name: &str) -> Result<String, PackFailure> {
    let Datum::Node { tag, fields, .. } = record else {
        return Err(fail("fresh-unqualified", "record"));
    };
    if tag.as_qualified_str() != "world/fresh-v1" {
        return Err(fail("fresh-unqualified", "record"));
    }
    match fields
        .iter()
        .find(|(field, _)| field.as_qualified_str() == name)
        .map(|(_, value)| value)
    {
        Some(Datum::String(value)) => Ok(value.clone()),
        _ => Err(fail("fresh-unqualified", name)),
    }
}

fn number(record: &Datum, name: &str) -> Result<String, PackFailure> {
    let Datum::Node { fields, .. } = record else {
        return Err(fail("fresh-unqualified", name));
    };
    match fields
        .iter()
        .find(|(field, _)| field.as_qualified_str() == name)
        .map(|(_, value)| value)
    {
        Some(Datum::Number(value)) => Ok(value.canonical.clone()),
        _ => Err(fail("fresh-unqualified", name)),
    }
}

fn bool_field(record: &Datum, name: &str) -> Result<bool, PackFailure> {
    let Datum::Node { fields, .. } = record else {
        return Err(fail("fresh-unqualified", name));
    };
    match fields
        .iter()
        .find(|(field, _)| field.as_qualified_str() == name)
        .map(|(_, value)| value)
    {
        Some(Datum::Bool(value)) => Ok(*value),
        _ => Err(fail("fresh-unqualified", name)),
    }
}

fn admit(
    subject: &str,
    observations: &[CheckObservation],
) -> Result<ObservedFreshCommand, PackFailure> {
    let executed = execution(observations)?;
    let again = read_change(EXPORT, CHANGE)?;
    if executed != again.execution {
        return Err(fail("evidence-mismatch", "fresh.change"));
    }
    let rows = vec![PackageRow {
        name: PROOF.to_owned(),
        version: "0".to_owned(),
        url: format!("git://{PROOF}"),
        rev: hex(&again.digest),
    }];
    let admitted = receipt::admit_for(
        "checker/c-succeed",
        AdmitExecuted {
            subject,
            result: RESULT,
            scope: SCOPE,
            proof: PROOF,
            domain: DOMAIN,
            policy: "policy/fresh-command",
            provenance: "provenance/fresh-command",
            support: "support/fresh-command",
            projection: "projection/fresh-command",
            environment: "environment/fresh-command",
            leaf: &again.digest,
            source_text: &receipt::package_source(DOMAIN, PROOF, &rows),
            definitions: &receipt::package_definitions(DOMAIN, &rows),
        },
    )?;
    Ok(ObservedFreshCommand {
        subject: subject.to_owned(),
        result: RESULT.to_owned(),
        execution: executed,
        catalog: admitted.catalog,
        admitted: admitted.admitted,
        checker: admitted.checker,
        scope: admitted.scope,
        revocation: admitted.revocation,
    })
}

fn execution(observations: &[CheckObservation]) -> Result<String, PackFailure> {
    let mut parts = Vec::with_capacity(KEYS.len());
    for key in KEYS {
        let value = observations
            .iter()
            .find(|item| item.key == *key)
            .map(|item| item.value.as_str())
            .ok_or_else(|| fail("missing-source", key))?;
        if value.contains("completion/claim-") {
            return Err(fail("claim-relabelled", key));
        }
        parts.push(value.to_owned());
    }
    Ok(join(&parts))
}

fn join(parts: &[String]) -> String {
    let mut out = String::new();
    for part in parts {
        receipt::push_field(&mut out, part);
    }
    out
}

fn token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !value.contains("completion/claim-")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && byte != b' ')
}

fn fail(code: &'static str, detail: &str) -> PackFailure {
    PackFailure {
        code,
        detail: detail.to_owned(),
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 0xf) as usize] as char);
    }
    out
}

#[cfg(test)]
#[path = "succession_fresh_tests.rs"]
mod tests;
