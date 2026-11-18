// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Owner validation executed from a repository manifest.
//!
//! The selected names are looked up in the manifest. Each repository's
//! `validation_command` and `docs_command` are read from that file and
//! spawned. The receipt records that both commands exited zero. The
//! result is `owner-validation`. It is not `completion/claim-20`:
//! integration, package, generated output, and standalone CI are not
//! executed here. There is no passed-count argument and no command-string
//! argument. A non-zero status produces no receipt.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use sha2::{Digest, Sha256};
use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};
use sim_lib_roadmap_runner::{AdmittedEvidence, ProofCatalog};

use super::receipt::{self, AdmitExecuted, PackageRow};
use crate::{CheckObservation, MemorySubject, PackFailure, PackRequest, PackVerdict, find_pack};

const SCOPE: &str = "release/owner-validation";
const DOMAIN: &str = "release/owner-validation";
const FACT: &str = "release.owners";
const MAX_REPOS: usize = 64;

/// Receipt for commands the manifest declared and this process ran.
pub struct ObservedOwnerValidation {
    /// Subject the receipt names.
    pub subject: String,
    /// `owner-validation` when every declared command exited zero.
    ///
    /// This is not `completion/claim-20`.
    pub result: String,
    /// Repository names that were executed, in manifest selection order.
    pub repos: Vec<String>,
    /// Catalog that admitted the evidence.
    pub catalog: ProofCatalog,
    /// Evidence the catalog admitted.
    pub admitted: AdmittedEvidence,
    /// Checker receipt that bound the admission.
    pub checker: sim_conformance_v02::CheckerReceiptId,
    /// Scope `release/owner-validation`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Runs the declared validation and docs commands for `repos`.
///
/// # Errors
///
/// Returns a refusal when a repository is absent, a command cannot be
/// spawned, a command exits non-zero, or the catalog refuses the receipt.
pub fn observe_owner_validation(
    subject: &str,
    manifest: &Path,
    repos: &[&str],
) -> Result<ObservedOwnerValidation, PackFailure> {
    let checked = CheckedSubjectId::from_text(subject)
        .map_err(|error| fail("subject-identity", &error.to_string()))?;
    let spec = find_pack("checker/c-release")
        .ok_or_else(|| fail("unknown-checker", "checker/c-release"))?;
    let binding = spec
        .checker_binding()
        .map_err(|error| fail("invalid-static-binding", &error.to_string()))?;
    let binding_id = receipt::render(binding.id().content_id());
    let evidence = MemorySubject::default().with(FACT, encode_request(manifest, repos)?);
    let verdict = super::check(&PackRequest {
        checker: "checker/c-release",
        binding: &binding_id,
        subject: &receipt::render(checked.content_id()),
        scope: SCOPE,
        evidence: &evidence,
    });
    let observations = match verdict {
        PackVerdict::Pass { observations, .. } => observations,
        PackVerdict::Refused(failure) => return Err(failure),
        PackVerdict::UnimplementedPack { .. } => return Err(fail("unimplemented", SCOPE)),
    };
    admit_rows(subject, repos, &observations)
}

/// Re-checks an owner-validation receipt against the catalog that admitted it.
///
/// # Errors
///
/// Returns a refusal when the result, scope, subject, or catalog binding
/// no longer agrees with the admitted receipt.
pub fn bind_owner_validation(observation: &ObservedOwnerValidation) -> Result<(), PackFailure> {
    if observation.result != "owner-validation" || observation.scope != scope_id()? {
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
    let mut body = None;
    for key in request.evidence.fact_keys() {
        match key {
            FACT => body = request.evidence.fact(key),
            other => return Err(fail("unexpected-fact", other)),
        }
    }
    let Some(body) = body else {
        return Err(fail("missing-source", FACT));
    };
    if body.contains("completion/claim-") {
        return Err(fail("claim-relabelled", FACT));
    }
    let (manifest, repos) = decode_request(body)?;
    let declared = declared_repos(&manifest)?;
    let mut encoded = String::new();
    for name in &repos {
        let repo = declared
            .get(name.as_str())
            .ok_or_else(|| fail("missing-source", name))?;
        let validation = run_shell(&repo.directory, &repo.validation)?;
        let docs = run_shell(&repo.directory, &repo.docs)?;
        receipt::push_field(&mut encoded, name);
        receipt::push_field(&mut encoded, &validation.stdout_hex);
        receipt::push_field(&mut encoded, &validation.stdout_bytes.to_string());
        receipt::push_field(&mut encoded, &validation.stderr_hex);
        receipt::push_field(&mut encoded, &validation.stderr_bytes.to_string());
        receipt::push_field(&mut encoded, &docs.stdout_hex);
        receipt::push_field(&mut encoded, &docs.stdout_bytes.to_string());
        receipt::push_field(&mut encoded, &docs.stderr_hex);
        receipt::push_field(&mut encoded, &docs.stderr_bytes.to_string());
    }
    Ok(vec![observation("release.execution", encoded)])
}

fn admit_rows(
    subject: &str,
    repos: &[&str],
    observations: &[CheckObservation],
) -> Result<ObservedOwnerValidation, PackFailure> {
    let encoded = observations
        .iter()
        .find(|item| item.key == "release.execution")
        .ok_or_else(|| fail("missing-source", "release.execution"))?;
    let executed = decode_execution(&encoded.value)?;
    if executed.len() != repos.len()
        || executed
            .iter()
            .zip(repos)
            .any(|(row, name)| row.name != *name)
    {
        return Err(fail("evidence-mismatch", "release.execution"));
    }
    let rows = executed
        .iter()
        .map(|row| PackageRow {
            name: row.name.clone(),
            version: "0".to_owned(),
            url: "git://owner-validation".to_owned(),
            rev: row.stdout_hex.clone(),
        })
        .collect::<Vec<_>>();
    let source = receipt::package_source(DOMAIN, "owner-validation", &rows);
    let definitions = receipt::package_definitions(DOMAIN, &rows);
    let admitted = receipt::admit_executed(AdmitExecuted {
        subject,
        result: "owner-validation",
        scope: SCOPE,
        proof: "owner-validation",
        domain: DOMAIN,
        policy: "policy/owner-validation",
        provenance: "provenance/owner-validation",
        support: "support/owner-validation",
        projection: "projection/owner-validation",
        environment: "environment/owner-validation",
        leaf: encoded.value.as_bytes(),
        source_text: &source,
        definitions: &definitions,
    })?;
    Ok(ObservedOwnerValidation {
        subject: subject.to_owned(),
        result: "owner-validation".to_owned(),
        repos: executed.iter().map(|row| row.name.clone()).collect(),
        catalog: admitted.catalog,
        admitted: admitted.admitted,
        checker: admitted.checker,
        scope: admitted.scope,
        revocation: admitted.revocation,
    })
}

struct DeclaredRepo {
    directory: PathBuf,
    validation: String,
    docs: String,
}

fn declared_repos(manifest: &Path) -> Result<BTreeMap<String, DeclaredRepo>, PackFailure> {
    let text = std::fs::read_to_string(manifest).map_err(|_| fail("bad-record", "manifest"))?;
    if text.len() > 1_048_576 {
        return Err(fail("bound", "manifest"));
    }
    let root: toml::Value = toml::from_str(&text).map_err(|_| fail("bad-record", "manifest"))?;
    let repos = root
        .get("repo")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| fail("missing-source", "repo"))?;
    let parent = manifest
        .parent()
        .ok_or_else(|| fail("bad-record", "manifest"))?;
    let mut declared = BTreeMap::new();
    for repo in repos {
        let table = repo.as_table().ok_or_else(|| fail("bad-record", "repo"))?;
        let name = require_string(table, "name")?;
        if declared.contains_key(name) {
            return Err(fail("duplicate-crate", name));
        }
        let local = require_string(table, "local_path")?;
        let validation = require_command(table, "validation_command")?;
        let docs = require_command(table, "docs_command")?;
        let directory = resolve_dir(parent, local)?;
        declared.insert(
            name.to_owned(),
            DeclaredRepo {
                directory,
                validation,
                docs,
            },
        );
    }
    Ok(declared)
}

fn resolve_dir(parent: &Path, local: &str) -> Result<PathBuf, PackFailure> {
    let path = Path::new(local);
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        parent.join(path)
    };
    std::fs::canonicalize(&joined).map_err(|_| fail("process-unrun", "directory"))
}

fn require_string<'a>(table: &'a toml::Table, key: &str) -> Result<&'a str, PackFailure> {
    match table.get(key) {
        Some(toml::Value::String(value)) if !value.is_empty() => Ok(value),
        _ => Err(fail("bad-record", key)),
    }
}

fn require_command(table: &toml::Table, key: &str) -> Result<String, PackFailure> {
    let command = require_string(table, key)?;
    if command.contains('\n') || command.contains('\r') || command.contains('\0') {
        return Err(fail("bad-record", key));
    }
    Ok(command.to_owned())
}

struct PipeDigest {
    stdout_hex: String,
    stdout_bytes: u64,
    stderr_hex: String,
    stderr_bytes: u64,
}

fn run_shell(dir: &Path, command: &str) -> Result<PipeDigest, PackFailure> {
    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| fail("process-unrun", "sh"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| fail("process-unrun", "stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| fail("process-unrun", "stderr"))?;
    let stderr_task = std::thread::spawn(move || hash_reader(stderr));
    let stdout_digest = hash_reader(stdout).map_err(|_| fail("process-unrun", "stdout"))?;
    let stderr_digest = stderr_task
        .join()
        .map_err(|_| fail("process-unrun", "stderr"))?
        .map_err(|_| fail("process-unrun", "stderr"))?;
    let status = child.wait().map_err(|_| fail("process-unrun", "wait"))?;
    let code = status
        .code()
        .ok_or_else(|| fail("process-unrun", "signal"))?;
    if code != 0 {
        return Err(fail("process-failed", &code.to_string()));
    }
    Ok(PipeDigest {
        stdout_hex: stdout_digest.0,
        stdout_bytes: stdout_digest.1,
        stderr_hex: stderr_digest.0,
        stderr_bytes: stderr_digest.1,
    })
}

fn hash_reader(mut reader: impl Read) -> std::io::Result<(String, u64)> {
    let mut hasher = Sha256::new();
    let mut buf = [0_u8; 8192];
    let mut total = 0_u64;
    loop {
        let read = reader.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
        total += read as u64;
    }
    Ok((
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        total,
    ))
}

fn encode_request(manifest: &Path, repos: &[&str]) -> Result<String, PackFailure> {
    if repos.is_empty() || repos.len() > MAX_REPOS {
        return Err(fail("missing-source", FACT));
    }
    let mut seen = std::collections::BTreeSet::new();
    let path = manifest
        .to_str()
        .ok_or_else(|| fail("bad-record", "manifest"))?;
    if path.is_empty() || path.contains('\0') || path.contains('\n') {
        return Err(fail("bad-record", "manifest"));
    }
    let mut encoded = String::new();
    receipt::push_field(&mut encoded, path);
    for name in repos {
        if !is_name(name) || !seen.insert(*name) {
            return Err(fail("bad-record", name));
        }
        receipt::push_field(&mut encoded, name);
    }
    Ok(encoded)
}

fn decode_request(body: &str) -> Result<(PathBuf, Vec<String>), PackFailure> {
    let (manifest, mut rest) = receipt::take_field(body)?;
    let mut repos = Vec::new();
    while !rest.is_empty() {
        let (name, next) = receipt::take_field(rest)?;
        repos.push(name.to_owned());
        rest = next;
        if repos.len() > MAX_REPOS {
            return Err(fail("bound", FACT));
        }
    }
    if repos.is_empty() {
        return Err(fail("missing-source", FACT));
    }
    Ok((PathBuf::from(manifest), repos))
}

struct ExecutedRepo {
    name: String,
    stdout_hex: String,
}

fn decode_execution(body: &str) -> Result<Vec<ExecutedRepo>, PackFailure> {
    let mut rest = body;
    let mut rows = Vec::new();
    while !rest.is_empty() {
        let (name, next) = receipt::take_field(rest)?;
        let (stdout_hex, next) = receipt::take_field(next)?;
        if stdout_hex.len() != 64 || !stdout_hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(fail("bad-record", "stdout"));
        }
        let mut next = next;
        for _ in 0..7 {
            let (_, followed) = receipt::take_field(next)?;
            next = followed;
        }
        rows.push(ExecutedRepo {
            name: name.to_owned(),
            stdout_hex: stdout_hex.to_owned(),
        });
        rest = next;
    }
    Ok(rows)
}

fn scope_id() -> Result<CheckScopeId, PackFailure> {
    CheckScopeId::from_text(SCOPE).map_err(|error| fail("receipt-refused", &error.to_string()))
}

fn is_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
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
#[path = "release_validation_tests.rs"]
mod tests;
