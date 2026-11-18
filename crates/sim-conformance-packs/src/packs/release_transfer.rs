// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Authority-transfer record read from git, plus a provider directory scan.
//!
//! The record must say `scope portable`, `native-credential false`, and
//! either `caller-zero true` or `caller-zero false` with a retained role.
//! `git show` reads that blob with `--no-replace-objects`. The provider
//! directory is listed by the process. A credential file name refuses the
//! receipt. The result is `authority-transfer`. It is not
//! `completion/claim-22`: the record is not an observed handoff event.

use std::path::{Component, Path, PathBuf};

use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};
use sim_lib_roadmap_runner::{AdmittedEvidence, ProofCatalog};

use super::receipt::{self, AdmitExecuted, PackageRow};
use crate::packs::completion_claim::execute_process;
use crate::{CheckObservation, MemorySubject, PackFailure, PackRequest, PackVerdict, find_pack};

const SCOPE: &str = "release/authority-transfer";
const DOMAIN: &str = "authority/transfer";
const FACT: &str = "release.transfer";
const RESULT: &str = "authority-transfer";
const MAX_ENTRIES: usize = 4096;

/// Coordinates of one transfer record and the provider directory to scan.
pub struct TransferSource {
    /// Repository that contains the commit.
    pub repository: PathBuf,
    /// Commit object to verify. Forty lowercase hex digits.
    pub commit: String,
    /// Transfer-record path inside that commit.
    pub record: String,
    /// Provider configuration directory to list.
    pub provider_config: PathBuf,
}

/// Receipt produced by reading a transfer record and scanning a directory.
pub struct ObservedAuthorityTransfer {
    /// Subject the receipt names.
    pub subject: String,
    /// `authority-transfer`. This is not `completion/claim-22`.
    pub result: String,
    /// Retained role when caller-zero is false.
    pub retained_role: Option<String>,
    /// Catalog that admitted the evidence.
    pub catalog: ProofCatalog,
    /// Evidence the catalog admitted.
    pub admitted: AdmittedEvidence,
    /// Checker receipt that bound the admission.
    pub checker: sim_conformance_v02::CheckerReceiptId,
    /// Scope `release/authority-transfer`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Reads the transfer record, scans the provider directory, and admits the receipt.
///
/// # Errors
///
/// Returns a refusal when the record is not a portable caller-zero transfer,
/// git cannot show it, or the directory holds a credential file.
pub fn observe_authority_transfer(
    subject: &str,
    source: &TransferSource,
) -> Result<ObservedAuthorityTransfer, PackFailure> {
    let checked = CheckedSubjectId::from_text(subject)
        .map_err(|error| fail("subject-identity", &error.to_string()))?;
    let spec = find_pack("checker/c-release")
        .ok_or_else(|| fail("unknown-checker", "checker/c-release"))?;
    let binding = spec
        .checker_binding()
        .map_err(|error| fail("invalid-static-binding", &error.to_string()))?;
    let evidence = MemorySubject::default().with(FACT, encode_coordinates(source)?);
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

/// Re-checks an authority-transfer receipt against the catalog that admitted it.
///
/// # Errors
///
/// Returns a refusal when the result or scope is no longer `authority-transfer`.
pub fn bind_authority_transfer(observation: &ObservedAuthorityTransfer) -> Result<(), PackFailure> {
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
    let source = decode_coordinates(one_fact(request)?)?;
    let rev = verify_commit(&source)?;
    let record = git_show(&source.repository, &rev, &source.record)?;
    let role = parse_record(&record)?;
    let provider = scan_provider(&source.provider_config)?;
    let mut encoded = String::new();
    receipt::push_field(&mut encoded, &rev);
    receipt::push_field(&mut encoded, &record);
    receipt::push_field(&mut encoded, &provider);
    receipt::push_field(&mut encoded, role.as_deref().unwrap_or(""));
    Ok(vec![observation("release.execution", encoded)])
}

fn admit_rows(
    subject: &str,
    observations: &[CheckObservation],
) -> Result<ObservedAuthorityTransfer, PackFailure> {
    let encoded = execution(observations)?;
    let (rev, record, provider, role) = decode_execution(encoded)?;
    let parsed = parse_record(&record)?;
    if !is_rev(&rev) || parsed.as_deref().unwrap_or("") != role || provider.is_empty() {
        return Err(fail("evidence-mismatch", "release.execution"));
    }
    let rows = vec![PackageRow {
        name: "authority-transfer".to_owned(),
        version: "0".to_owned(),
        url: "git://authority-transfer".to_owned(),
        rev,
    }];
    let admitted = receipt::admit_executed(AdmitExecuted {
        subject,
        result: RESULT,
        scope: SCOPE,
        proof: "authority-transfer",
        domain: DOMAIN,
        policy: "policy/authority-transfer",
        provenance: "provenance/authority-transfer",
        support: "support/authority-transfer",
        projection: "projection/authority-transfer",
        environment: "environment/authority-transfer",
        leaf: encoded.as_bytes(),
        source_text: &receipt::package_source(DOMAIN, "authority-transfer", &rows),
        definitions: &receipt::package_definitions(DOMAIN, &rows),
    })?;
    Ok(ObservedAuthorityTransfer {
        subject: subject.to_owned(),
        result: RESULT.to_owned(),
        retained_role: parsed,
        catalog: admitted.catalog,
        admitted: admitted.admitted,
        checker: admitted.checker,
        scope: admitted.scope,
        revocation: admitted.revocation,
    })
}

fn parse_record(blob: &str) -> Result<Option<String>, PackFailure> {
    if blob.len() > 512
        || blob.contains('\0')
        || blob.contains('\r')
        || blob.contains("completion/claim-")
    {
        return Err(fail("bad-record", "record"));
    }
    let text = blob.strip_suffix('\n').unwrap_or(blob);
    let mut lines = text.lines();
    require_line(lines.next(), "scope portable")?;
    require_line(lines.next(), "native-credential false")?;
    match lines.next() {
        Some("caller-zero true") => {
            if lines.next().is_some() {
                return Err(fail("bad-record", "record"));
            }
            Ok(None)
        }
        Some("caller-zero false") => {
            let role = lines
                .next()
                .and_then(|line| line.strip_prefix("retained-role "))
                .ok_or_else(|| fail("transfer-role", "record"))?;
            if lines.next().is_some() || !is_token(role) {
                return Err(fail("transfer-role", "record"));
            }
            Ok(Some(role.to_owned()))
        }
        Some(line) if line.starts_with("native-credential") || line.starts_with("scope") => {
            Err(fail("transfer-scope", "record"))
        }
        _ => Err(fail("bad-record", "record")),
    }
}

fn require_line(line: Option<&str>, expected: &str) -> Result<(), PackFailure> {
    match line {
        Some(value) if value == expected => Ok(()),
        Some(value) if expected.starts_with("scope") && value.starts_with("scope ") => {
            Err(fail("transfer-scope", "record"))
        }
        Some(value) if expected.starts_with("native-credential") => {
            Err(fail("native-credential", value))
        }
        _ => Err(fail("bad-record", "record")),
    }
}

fn scan_provider(dir: &Path) -> Result<String, PackFailure> {
    let root = std::fs::canonicalize(dir).map_err(|_| fail("missing-source", "provider"))?;
    if !root.is_dir() {
        return Err(fail("bad-record", "provider"));
    }
    let mut stack = vec![root.clone()];
    let mut seen = 0usize;
    while let Some(current) = stack.pop() {
        let entries =
            std::fs::read_dir(&current).map_err(|_| fail("missing-source", "provider"))?;
        for entry in entries {
            let entry = entry.map_err(|_| fail("missing-source", "provider"))?;
            seen += 1;
            if seen > MAX_ENTRIES {
                return Err(fail("bound", "provider"));
            }
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if credential_name(&name) {
                return Err(fail("provider-credential", "provider"));
            }
            let meta = std::fs::symlink_metadata(entry.path())
                .map_err(|_| fail("missing-source", "provider"))?;
            if meta.is_dir() {
                stack.push(entry.path());
            }
        }
    }
    Ok(root.to_string_lossy().into_owned())
}

fn credential_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower == "credentials.toml"
        || lower == "credentials.json"
        || lower == "publish-token"
        || lower.ends_with(".pem")
        || lower.ends_with(".key")
}

fn verify_commit(source: &TransferSource) -> Result<String, PackFailure> {
    if !is_rev(&source.commit) {
        return Err(fail("bad-record", "commit"));
    }
    relative_path(&source.record)?;
    let repository =
        std::fs::canonicalize(&source.repository).map_err(|_| fail("bad-record", "repository"))?;
    let repo = repository
        .to_str()
        .ok_or_else(|| fail("bad-record", "repository"))?;
    let verified = execute_process(
        "git",
        &[
            "--no-replace-objects",
            "-C",
            repo,
            "rev-parse",
            "--verify",
            &format!("{}^{{commit}}", source.commit),
        ],
    )?;
    let rev = std::str::from_utf8(&verified.stdout)
        .map_err(|_| fail("bad-record", "commit"))?
        .trim_end_matches(['\n', '\r'])
        .to_owned();
    if rev != source.commit {
        return Err(fail("evidence-mismatch", "commit"));
    }
    Ok(rev)
}

fn git_show(repository: &Path, rev: &str, path: &str) -> Result<String, PackFailure> {
    let repository =
        std::fs::canonicalize(repository).map_err(|_| fail("bad-record", "repository"))?;
    let repo = repository
        .to_str()
        .ok_or_else(|| fail("bad-record", "repository"))?;
    let shown = execute_process(
        "git",
        &[
            "--no-replace-objects",
            "-C",
            repo,
            "show",
            &format!("{rev}:{path}"),
        ],
    )?;
    String::from_utf8(shown.stdout).map_err(|_| fail("bad-record", "record"))
}

fn encode_coordinates(source: &TransferSource) -> Result<String, PackFailure> {
    let repo = source
        .repository
        .to_str()
        .ok_or_else(|| fail("bad-record", "repository"))?;
    let provider = source
        .provider_config
        .to_str()
        .ok_or_else(|| fail("bad-record", "provider"))?;
    for text in [repo, provider] {
        if text.is_empty() || text.contains('\0') || text.contains('\n') {
            return Err(fail("bad-record", "path"));
        }
    }
    let mut encoded = String::new();
    receipt::push_field(&mut encoded, repo);
    receipt::push_field(&mut encoded, &source.commit);
    receipt::push_field(&mut encoded, &source.record);
    receipt::push_field(&mut encoded, provider);
    Ok(encoded)
}

fn decode_coordinates(body: &str) -> Result<TransferSource, PackFailure> {
    if body.contains("completion/claim-") || body.contains("git+") {
        return Err(fail("claim-relabelled", FACT));
    }
    let (repository, rest) = receipt::take_field(body)?;
    let (commit, rest) = receipt::take_field(rest)?;
    let (record, rest) = receipt::take_field(rest)?;
    let (provider, rest) = receipt::take_field(rest)?;
    if !rest.is_empty() {
        return Err(fail("bad-record", FACT));
    }
    Ok(TransferSource {
        repository: PathBuf::from(repository),
        commit: commit.to_owned(),
        record: record.to_owned(),
        provider_config: PathBuf::from(provider),
    })
}

fn decode_execution(body: &str) -> Result<(String, String, String, String), PackFailure> {
    let (rev, rest) = receipt::take_field(body)?;
    let (record, rest) = receipt::take_field(rest)?;
    let (provider, rest) = receipt::take_field(rest)?;
    let (role, rest) = receipt::take_field(rest)?;
    if !rest.is_empty() {
        return Err(fail("bad-record", "release.execution"));
    }
    Ok((
        rev.to_owned(),
        record.to_owned(),
        provider.to_owned(),
        role.to_owned(),
    ))
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

fn relative_path(path: &str) -> Result<(), PackFailure> {
    if path.is_empty() {
        return Err(fail("bad-record", "path"));
    }
    let path = Path::new(path);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(fail("bad-record", "path"));
    }
    Ok(())
}

fn is_rev(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn is_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        && !matches!(value, "true" | "published" | "shade-1" | "PortableClosed")
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
#[path = "release_transfer_tests.rs"]
mod tests;
