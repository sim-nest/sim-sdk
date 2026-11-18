// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Unpublished package identities read from git objects.
//!
//! Each package names a repository, a commit, and a manifest path. `git
//! rev-parse` verifies the commit and `git show` reads that commit's
//! manifest. Both commands pass `--no-replace-objects`, so a replacement
//! ref cannot substitute another manifest. The name, version, and
//! `publish = false` come from that blob.
//! The result is `completion/claim-21` plus those identities, sorted.
//! Tags, mirrors, pins, and a registry are not read. A caller-supplied
//! identity list is not an input, and this scope is not
//! `release/source-identity`.

use std::path::{Component, Path, PathBuf};

use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};
use sim_lib_roadmap_runner::{AdmittedEvidence, ProofCatalog};

use super::receipt::{self, AdmitExecuted, PackageRow};
use crate::packs::completion_claim::execute_process;
use crate::{CheckObservation, MemorySubject, PackFailure, PackRequest, PackVerdict, find_pack};

const SCOPE: &str = "release/git-worktree";
const DOMAIN: &str = "resolver/source";
const FACT: &str = "release.worktrees";
const MAX_PACKAGES: usize = 64;

/// One package whose identity is read from a git commit.
pub struct GitPackage {
    /// Repository that contains the commit.
    pub repository: PathBuf,
    /// Commit object to verify. Forty lowercase hex digits.
    pub commit: String,
    /// Manifest path inside that commit, ending in `Cargo.toml`.
    pub manifest: String,
}

/// Claim 21 receipt produced by reading git objects.
pub struct ObservedGitRelease {
    /// Subject the receipt names.
    pub subject: String,
    /// `completion/claim-21` plus the sorted git identities.
    pub result: String,
    /// Identities `git show` produced, in result order.
    pub identities: Vec<String>,
    /// Catalog that admitted the evidence.
    pub catalog: ProofCatalog,
    /// Evidence the catalog admitted.
    pub admitted: AdmittedEvidence,
    /// Checker receipt that bound the admission.
    pub checker: sim_conformance_v02::CheckerReceiptId,
    /// Scope `release/git-worktree`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Reads each package from git and admits claim 21 for those identities.
///
/// # Errors
///
/// Returns a refusal when a commit is missing, the manifest is not an
/// unpublished package, or the catalog refuses the receipt.
pub fn observe_git_release(
    subject: &str,
    packages: &[GitPackage],
) -> Result<ObservedGitRelease, PackFailure> {
    let checked = CheckedSubjectId::from_text(subject)
        .map_err(|error| fail("subject-identity", &error.to_string()))?;
    let spec = find_pack("checker/c-release")
        .ok_or_else(|| fail("unknown-checker", "checker/c-release"))?;
    let binding = spec
        .checker_binding()
        .map_err(|error| fail("invalid-static-binding", &error.to_string()))?;
    let binding_id = receipt::render(binding.id().content_id());
    let evidence = MemorySubject::default().with(FACT, encode_coordinates(packages)?);
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
    admit_rows(subject, &observations)
}

/// Re-checks a git receipt against the catalog that admitted it.
///
/// # Errors
///
/// Returns a refusal when the result, scope, subject, or catalog binding
/// no longer agrees with the admitted receipt.
pub fn bind_git_release(observation: &ObservedGitRelease) -> Result<(), PackFailure> {
    let expected = claim_result(&observation.identities);
    if observation.result != expected || observation.scope != scope_id()? {
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
    let mut rows = Vec::new();
    for package in decode_coordinates(body)? {
        rows.push(read_package(&package)?);
    }
    rows.sort_by_key(identity);
    let mut names = std::collections::BTreeSet::new();
    let mut identities = std::collections::BTreeSet::new();
    let mut encoded = String::new();
    for row in &rows {
        if !names.insert(row.name.as_str()) || !identities.insert(identity(row)) {
            return Err(fail("duplicate-crate", &row.name));
        }
        receipt::push_field(&mut encoded, &row.rev);
        receipt::push_field(&mut encoded, &row.name);
        receipt::push_field(&mut encoded, &row.version);
        receipt::push_field(&mut encoded, &row.blob);
    }
    Ok(vec![
        observation("release.unpublished", "publish-false"),
        observation("release.execution", encoded),
    ])
}

fn admit_rows(
    subject: &str,
    observations: &[CheckObservation],
) -> Result<ObservedGitRelease, PackFailure> {
    let unpublished = observations
        .iter()
        .filter(|item| item.key == "release.unpublished")
        .map(|item| item.value.as_str())
        .collect::<Vec<_>>();
    if unpublished != ["publish-false"] {
        return Err(fail("evidence-mismatch", "release.unpublished"));
    }
    let encoded = observations
        .iter()
        .find(|item| item.key == "release.execution")
        .ok_or_else(|| fail("missing-source", "release.execution"))?;
    let rows = decode_rows(&encoded.value)?;
    if rows.is_empty() {
        return Err(fail("missing-source", FACT));
    }
    let mut package_rows = Vec::new();
    let mut identities = Vec::new();
    for row in &rows {
        let parsed = parse_package(&row.blob)?;
        if parsed.0 != row.name || parsed.1 != row.version || !is_rev(&row.rev) {
            return Err(fail("evidence-mismatch", "release.execution"));
        }
        let identity = format!("git+{}:{}@{}", row.rev, row.name, row.version);
        package_rows.push(PackageRow {
            name: row.name.clone(),
            version: row.version.clone(),
            url: "git://worktree".to_owned(),
            rev: row.rev.clone(),
        });
        identities.push(identity);
    }
    let result = claim_result(&identities);
    let source = receipt::package_source(DOMAIN, "git-worktree", &package_rows);
    let definitions = receipt::package_definitions(DOMAIN, &package_rows);
    let admitted = receipt::admit_executed(AdmitExecuted {
        subject,
        result: &result,
        scope: SCOPE,
        proof: "git-worktree",
        domain: DOMAIN,
        policy: "policy/git-worktree",
        provenance: "provenance/git-worktree",
        support: "support/git-worktree",
        projection: "projection/git-worktree",
        environment: "environment/git-worktree",
        leaf: encoded.value.as_bytes(),
        source_text: &source,
        definitions: &definitions,
    })?;
    Ok(ObservedGitRelease {
        subject: subject.to_owned(),
        result,
        identities,
        catalog: admitted.catalog,
        admitted: admitted.admitted,
        checker: admitted.checker,
        scope: admitted.scope,
        revocation: admitted.revocation,
    })
}

struct ReadPackage {
    name: String,
    version: String,
    rev: String,
    blob: String,
}

fn read_package(package: &GitPackage) -> Result<ReadPackage, PackFailure> {
    if !is_rev(&package.commit) {
        return Err(fail("bad-record", "commit"));
    }
    relative_manifest(&package.manifest)?;
    let repository =
        std::fs::canonicalize(&package.repository).map_err(|_| fail("bad-record", "repository"))?;
    if !repository.is_dir() {
        return Err(fail("bad-record", "repository"));
    }
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
            &format!("{}^{{commit}}", package.commit),
        ],
    )?;
    let rev = std::str::from_utf8(&verified.stdout)
        .map_err(|_| fail("bad-record", "commit"))?
        .trim_end_matches(['\n', '\r'])
        .to_owned();
    if rev != package.commit {
        return Err(fail("evidence-mismatch", "commit"));
    }
    let shown = execute_process(
        "git",
        &[
            "--no-replace-objects",
            "-C",
            repo,
            "show",
            &format!("{}:{}", package.commit, package.manifest),
        ],
    )?;
    let blob = String::from_utf8(shown.stdout).map_err(|_| fail("bad-record", "manifest"))?;
    let (name, version) = parse_package(&blob)?;
    Ok(ReadPackage {
        name,
        version,
        rev,
        blob,
    })
}

fn parse_package(blob: &str) -> Result<(String, String), PackFailure> {
    if blob.len() > 65_536 || blob.contains('\0') {
        return Err(fail("bound", "manifest"));
    }
    let root: toml::Value = toml::from_str(blob).map_err(|_| fail("bad-record", "manifest"))?;
    let package = root
        .get("package")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| fail("missing-observation", "package"))?;
    match package.get("publish") {
        Some(toml::Value::Boolean(false)) => {}
        Some(toml::Value::Boolean(true)) => {
            return Err(fail("local-publication-claim", "publish"));
        }
        _ => return Err(fail("missing-observation", "publish = false")),
    }
    let name = require_string(package, "name")?;
    let version = require_string(package, "version")?;
    if !is_name(name) || !is_version(version) {
        return Err(fail("bad-record", name));
    }
    Ok((name.to_owned(), version.to_owned()))
}

fn require_string<'a>(table: &'a toml::Table, key: &str) -> Result<&'a str, PackFailure> {
    match table.get(key) {
        Some(toml::Value::String(value)) => Ok(value),
        _ => Err(fail("bad-record", key)),
    }
}

fn encode_coordinates(packages: &[GitPackage]) -> Result<String, PackFailure> {
    if packages.is_empty() || packages.len() > MAX_PACKAGES {
        return Err(fail("missing-source", FACT));
    }
    let mut encoded = String::new();
    for package in packages {
        let repo = package
            .repository
            .to_str()
            .ok_or_else(|| fail("bad-record", "repository"))?;
        if repo.is_empty() || repo.contains('\0') || repo.contains('\n') {
            return Err(fail("bad-record", "repository"));
        }
        receipt::push_field(&mut encoded, repo);
        receipt::push_field(&mut encoded, &package.commit);
        receipt::push_field(&mut encoded, &package.manifest);
    }
    Ok(encoded)
}

fn decode_coordinates(body: &str) -> Result<Vec<GitPackage>, PackFailure> {
    if body.contains("completion/claim-") || body.contains("git+") {
        return Err(fail("claim-relabelled", FACT));
    }
    let mut rest = body;
    let mut packages = Vec::new();
    while !rest.is_empty() {
        let (repository, next) = receipt::take_field(rest)?;
        let (commit, next) = receipt::take_field(next)?;
        let (manifest, next) = receipt::take_field(next)?;
        packages.push(GitPackage {
            repository: PathBuf::from(repository),
            commit: commit.to_owned(),
            manifest: manifest.to_owned(),
        });
        rest = next;
        if packages.len() > MAX_PACKAGES {
            return Err(fail("bound", FACT));
        }
    }
    if packages.is_empty() {
        return Err(fail("missing-source", FACT));
    }
    Ok(packages)
}

struct EncodedRow {
    rev: String,
    name: String,
    version: String,
    blob: String,
}

fn decode_rows(body: &str) -> Result<Vec<EncodedRow>, PackFailure> {
    let mut rest = body;
    let mut rows = Vec::new();
    while !rest.is_empty() {
        let (rev, next) = receipt::take_field(rest)?;
        let (name, next) = receipt::take_field(next)?;
        let (version, next) = receipt::take_field(next)?;
        let (blob, next) = receipt::take_field(next)?;
        rows.push(EncodedRow {
            rev: rev.to_owned(),
            name: name.to_owned(),
            version: version.to_owned(),
            blob: blob.to_owned(),
        });
        rest = next;
    }
    Ok(rows)
}

fn identity(row: &ReadPackage) -> String {
    format!("git+{}:{}@{}", row.rev, row.name, row.version)
}

fn claim_result(identities: &[String]) -> String {
    let mut result = String::from("completion/claim-21");
    for identity in identities {
        result.push(' ');
        result.push_str(identity);
    }
    result
}

fn scope_id() -> Result<CheckScopeId, PackFailure> {
    CheckScopeId::from_text(SCOPE).map_err(|error| fail("receipt-refused", &error.to_string()))
}

fn relative_manifest(path: &str) -> Result<(), PackFailure> {
    if path.is_empty() || !path.ends_with("Cargo.toml") {
        return Err(fail("bad-record", "manifest"));
    }
    let path = Path::new(path);
    if path.is_absolute() {
        return Err(fail("bad-record", "manifest"));
    }
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(fail("bad-record", "manifest"));
    }
    Ok(())
}

fn is_rev(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn is_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        && !matches!(value, "true" | "published" | "shade-1" | "PortableClosed")
}

fn is_version(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('.')
        && !value.ends_with('.')
        && value.bytes().any(|byte| byte.is_ascii_digit())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.')
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
#[path = "release_worktree_tests.rs"]
mod tests;
