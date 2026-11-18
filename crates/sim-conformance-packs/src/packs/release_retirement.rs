// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Retirement guard executed from the repository that contains it.
//!
//! The caller names a repository root. This module runs
//! `scripts/check-v3-retired.sh` in that root through `/bin/sh` and
//! records the output hashes. It does not take a command string or a
//! passed count. The result is `retirement-guard`. It is not
//! `completion/claim-3`: the script is one owner's guard, not a scan of
//! every production and conformance closure.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};
use sim_lib_roadmap_runner::{AdmittedEvidence, ProofCatalog};

use super::receipt::{self, AdmitExecuted, PackageRow};
use crate::packs::completion_claim::execute_process;
use crate::{CheckObservation, MemorySubject, PackFailure, PackRequest, PackVerdict, find_pack};

const SCOPE: &str = "release/retirement-guard";
const DOMAIN: &str = "retirement/guard";
const FACT: &str = "release.retirement";
const RESULT: &str = "retirement-guard";
const SCRIPT: &str = "scripts/check-v3-retired.sh";

/// Receipt produced by running the retirement guard script.
pub struct ObservedRetirementGuard {
    /// Subject the receipt names.
    pub subject: String,
    /// `retirement-guard`. This is not `completion/claim-3`.
    pub result: String,
    /// Canonical root whose script ran.
    pub root: String,
    /// Catalog that admitted the evidence.
    pub catalog: ProofCatalog,
    /// Evidence the catalog admitted.
    pub admitted: AdmittedEvidence,
    /// Checker receipt that bound the admission.
    pub checker: sim_conformance_v02::CheckerReceiptId,
    /// Scope `release/retirement-guard`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Runs the retirement script and admits `retirement-guard`.
///
/// # Errors
///
/// Returns a refusal when the script is absent, leaves the root, or exits
/// non-zero.
pub fn observe_retirement_guard(
    subject: &str,
    root: &Path,
) -> Result<ObservedRetirementGuard, PackFailure> {
    let checked = CheckedSubjectId::from_text(subject)
        .map_err(|error| fail("subject-identity", &error.to_string()))?;
    let spec = find_pack("checker/c-release")
        .ok_or_else(|| fail("unknown-checker", "checker/c-release"))?;
    let binding = spec
        .checker_binding()
        .map_err(|error| fail("invalid-static-binding", &error.to_string()))?;
    let evidence = MemorySubject::default().with(FACT, encode_root(root)?);
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
    admit_rows(subject, &observations)
}

/// Re-checks a retirement receipt against the catalog that admitted it.
///
/// # Errors
///
/// Returns a refusal when the result or scope is no longer `retirement-guard`.
pub fn bind_retirement_guard(observation: &ObservedRetirementGuard) -> Result<(), PackFailure> {
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
    let root = decode_root(one_fact(request)?)?;
    let script = script_in_root(&root)?;
    let ran = execute_process(
        "/bin/sh",
        &[script
            .to_str()
            .ok_or_else(|| fail("bad-record", "script"))?],
    )?;
    let mut encoded = String::new();
    receipt::push_field(&mut encoded, &root.to_string_lossy());
    receipt::push_field(
        &mut encoded,
        std::str::from_utf8(&ran.stdout).map_err(|_| fail("bad-record", "stdout"))?,
    );
    receipt::push_field(
        &mut encoded,
        std::str::from_utf8(&ran.stderr).map_err(|_| fail("bad-record", "stderr"))?,
    );
    Ok(vec![observation("release.execution", encoded)])
}

fn admit_rows(
    subject: &str,
    observations: &[CheckObservation],
) -> Result<ObservedRetirementGuard, PackFailure> {
    let encoded = execution(observations)?;
    let (root, stdout, stderr) = decode_execution(encoded)?;
    let mut hasher = Sha256::new();
    hasher.update(stdout.as_bytes());
    hasher.update(stderr.as_bytes());
    let rows = vec![PackageRow {
        name: "retirement-guard".to_owned(),
        version: "0".to_owned(),
        url: "git://retirement-guard".to_owned(),
        rev: hex_bytes(hasher.finalize().as_slice()),
    }];
    let admitted = receipt::admit_executed(AdmitExecuted {
        subject,
        result: RESULT,
        scope: SCOPE,
        proof: "retirement-guard",
        domain: DOMAIN,
        policy: "policy/retirement-guard",
        provenance: "provenance/retirement-guard",
        support: "support/retirement-guard",
        projection: "projection/retirement-guard",
        environment: "environment/retirement-guard",
        leaf: encoded.as_bytes(),
        source_text: &receipt::package_source(DOMAIN, "retirement-guard", &rows),
        definitions: &receipt::package_definitions(DOMAIN, &rows),
    })?;
    Ok(ObservedRetirementGuard {
        subject: subject.to_owned(),
        result: RESULT.to_owned(),
        root,
        catalog: admitted.catalog,
        admitted: admitted.admitted,
        checker: admitted.checker,
        scope: admitted.scope,
        revocation: admitted.revocation,
    })
}

fn script_in_root(root: &Path) -> Result<PathBuf, PackFailure> {
    let script = root.join(SCRIPT);
    let canonical = std::fs::canonicalize(&script).map_err(|_| fail("missing-source", SCRIPT))?;
    if !canonical.starts_with(root) || !canonical.is_file() {
        return Err(fail("bad-record", SCRIPT));
    }
    Ok(canonical)
}

fn encode_root(root: &Path) -> Result<String, PackFailure> {
    let text = root.to_str().ok_or_else(|| fail("bad-record", "root"))?;
    if text.is_empty()
        || text.contains('\0')
        || text.contains('\n')
        || text.contains("completion/claim-")
    {
        return Err(fail("bad-record", "root"));
    }
    let mut encoded = String::new();
    receipt::push_field(&mut encoded, text);
    Ok(encoded)
}

fn decode_root(body: &str) -> Result<PathBuf, PackFailure> {
    if body.contains("completion/claim-") {
        return Err(fail("claim-relabelled", FACT));
    }
    let (text, rest) = receipt::take_field(body)?;
    if !rest.is_empty() {
        return Err(fail("bad-record", FACT));
    }
    let root = std::fs::canonicalize(text).map_err(|_| fail("missing-source", "root"))?;
    if !root.is_dir() {
        return Err(fail("bad-record", "root"));
    }
    Ok(root)
}

fn decode_execution(body: &str) -> Result<(String, String, String), PackFailure> {
    let (root, rest) = receipt::take_field(body)?;
    let (stdout, rest) = receipt::take_field(rest)?;
    let (stderr, rest) = receipt::take_field(rest)?;
    if !rest.is_empty() {
        return Err(fail("bad-record", "release.execution"));
    }
    Ok((root.to_owned(), stdout.to_owned(), stderr.to_owned()))
}

fn one_fact<'a>(request: &'a PackRequest<'a>) -> Result<&'a str, PackFailure> {
    let mut body = None;
    for key in request.evidence.fact_keys() {
        match key {
            FACT => body = request.evidence.fact(key),
            other => return Err(fail("unexpected-fact", other)),
        }
    }
    body.ok_or_else(|| fail("missing-source", FACT))
}

fn execution(observations: &[CheckObservation]) -> Result<&str, PackFailure> {
    observations
        .iter()
        .find(|item| item.key == "release.execution")
        .map(|item| item.value.as_str())
        .ok_or_else(|| fail("missing-source", "release.execution"))
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
#[path = "release_retirement_tests.rs"]
mod tests;
