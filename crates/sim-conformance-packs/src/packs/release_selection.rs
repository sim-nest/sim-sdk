// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Selected source and its selection record, read from git objects.
//!
//! The caller names a commit, a source path, and a selection path. `git
//! rev-parse` and `git show` read those blobs. Both commands pass
//! `--no-replace-objects`. The source digest is computed from the blob.
//! The selection record must name two different authorities. Every
//! `NO_V3_<round>` marker in the source must be the round in its title,
//! and the source must contain claim lines 1 through 25.
//! The result is `source-selection`. It is not `completion/claim-1`:
//! this read does not compile the source back out, and it does not
//! verify a human signature. A caller-supplied digest is not an input.

use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};
use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};
use sim_lib_roadmap_runner::{AdmittedEvidence, ProofCatalog};

use super::receipt::{self, AdmitExecuted, PackageRow};
use crate::packs::completion_claim::execute_process;
use crate::{CheckObservation, MemorySubject, PackFailure, PackRequest, PackVerdict, find_pack};

const SCOPE: &str = "release/source-selection";
const DOMAIN: &str = "authoring/source-selection";
const FACT: &str = "release.selection";
const RESULT: &str = "source-selection";

/// Coordinates of one selected source in a git commit.
pub struct SelectedSource {
    /// Repository that contains the commit.
    pub repository: PathBuf,
    /// Commit object to verify. Forty lowercase hex digits.
    pub commit: String,
    /// Source path inside that commit.
    pub source: String,
    /// Selection-record path inside that commit.
    pub selection: String,
}

/// Receipt produced by reading a selected source from git.
pub struct ObservedSourceSelection {
    /// Subject the receipt names.
    pub subject: String,
    /// `source-selection`. This is not `completion/claim-1`.
    pub result: String,
    /// SHA-256 of the source blob.
    pub source_digest: String,
    /// Round named by the source title.
    pub round: u32,
    /// Catalog that admitted the evidence.
    pub catalog: ProofCatalog,
    /// Evidence the catalog admitted.
    pub admitted: AdmittedEvidence,
    /// Checker receipt that bound the admission.
    pub checker: sim_conformance_v02::CheckerReceiptId,
    /// Scope `release/source-selection`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Reads the source and selection record and admits `source-selection`.
///
/// # Errors
///
/// Returns a refusal when git cannot show the blobs, the selection
/// authorities match, another NO_V3 round appears, or a claim line is
/// missing.
pub fn observe_source_selection(
    subject: &str,
    source: &SelectedSource,
) -> Result<ObservedSourceSelection, PackFailure> {
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

/// Re-checks a source-selection receipt against the catalog that admitted it.
///
/// # Errors
///
/// Returns a refusal when the result, scope, subject, or catalog binding
/// no longer agrees with the admitted receipt.
pub fn bind_source_selection(observation: &ObservedSourceSelection) -> Result<(), PackFailure> {
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
    let selected = decode_coordinates(body)?;
    let rev = verify_commit(&selected)?;
    let source = git_show(&selected.repository, &rev, &selected.source)?;
    let selection = git_show(&selected.repository, &rev, &selected.selection)?;
    let (round, digest) = parse_source(&source)?;
    let (human, begin) = parse_selection(&selection)?;
    let mut encoded = String::new();
    receipt::push_field(&mut encoded, &rev);
    receipt::push_field(&mut encoded, &digest);
    receipt::push_field(&mut encoded, &round.to_string());
    receipt::push_field(&mut encoded, &human);
    receipt::push_field(&mut encoded, &begin);
    receipt::push_field(&mut encoded, &source);
    receipt::push_field(&mut encoded, &selection);
    Ok(vec![observation("release.execution", encoded)])
}

fn admit_rows(
    subject: &str,
    observations: &[CheckObservation],
) -> Result<ObservedSourceSelection, PackFailure> {
    let encoded = execution(observations)?;
    let decoded = decode_execution(encoded)?;
    let (parsed_round, parsed_digest) = parse_source(&decoded.source)?;
    let (parsed_human, parsed_begin) = parse_selection(&decoded.selection)?;
    if parsed_round != decoded.round
        || parsed_digest != decoded.digest
        || parsed_human != decoded.human
        || parsed_begin != decoded.begin
        || !is_rev(&decoded.rev)
    {
        return Err(fail("evidence-mismatch", "release.execution"));
    }
    let rows = vec![PackageRow {
        name: "source-selection".to_owned(),
        version: "0".to_owned(),
        url: "git://source-selection".to_owned(),
        rev: decoded.rev,
    }];
    let admitted = receipt::admit_executed(AdmitExecuted {
        subject,
        result: RESULT,
        scope: SCOPE,
        proof: "source-selection",
        domain: DOMAIN,
        policy: "policy/source-selection",
        provenance: "provenance/source-selection",
        support: "support/source-selection",
        projection: "projection/source-selection",
        environment: "environment/source-selection",
        leaf: encoded.as_bytes(),
        source_text: &receipt::package_source(DOMAIN, "source-selection", &rows),
        definitions: &receipt::package_definitions(DOMAIN, &rows),
    })?;
    Ok(ObservedSourceSelection {
        subject: subject.to_owned(),
        result: RESULT.to_owned(),
        source_digest: decoded.digest,
        round: decoded.round,
        catalog: admitted.catalog,
        admitted: admitted.admitted,
        checker: admitted.checker,
        scope: admitted.scope,
        revocation: admitted.revocation,
    })
}

fn parse_source(blob: &str) -> Result<(u32, String), PackFailure> {
    if blob.len() > 1_048_576 || blob.contains('\0') || blob.contains('\r') {
        return Err(fail("bound", "source"));
    }
    let title = blob
        .lines()
        .next()
        .ok_or_else(|| fail("bad-record", "source"))?;
    let round = title_round(title)?;
    let found = mentioned_rounds(blob)?;
    if found.is_empty() || found.iter().any(|item| *item != round) {
        return Err(fail("normative-sibling", "source"));
    }
    for claim in 1..=25 {
        let prefix = format!("{claim}. ");
        if !blob.lines().any(|line| line.starts_with(&prefix)) {
            return Err(fail("missing-observation", "claim-line"));
        }
    }
    Ok((round, hex_bytes(Sha256::digest(blob.as_bytes()).as_slice())))
}

fn parse_selection(blob: &str) -> Result<(String, String), PackFailure> {
    if blob.len() > 512 || blob.contains('\0') || blob.contains('\r') {
        return Err(fail("bound", "selection"));
    }
    let text = blob.strip_suffix('\n').unwrap_or(blob);
    let mut lines = text.lines();
    let human = one_field(lines.next(), "human-selection")?;
    let begin = one_field(lines.next(), "begin-authority")?;
    if lines.next().is_some() || human == begin {
        return Err(fail(
            if human == begin {
                "selection-begin"
            } else {
                "bad-record"
            },
            "selection",
        ));
    }
    Ok((human, begin))
}

fn one_field(line: Option<&str>, key: &str) -> Result<String, PackFailure> {
    let line = line.ok_or_else(|| fail("bad-record", key))?;
    let token = line
        .strip_prefix(&format!("{key} "))
        .ok_or_else(|| fail("bad-record", key))?;
    if !is_token(token) {
        return Err(fail("bad-record", key));
    }
    Ok(token.to_owned())
}

fn title_round(title: &str) -> Result<u32, PackFailure> {
    let rest = title
        .strip_prefix("# NO_V3_")
        .ok_or_else(|| fail("bad-record", "source"))?;
    let digits = rest.split(':').next().unwrap_or("");
    if digits.is_empty()
        || digits.len() > 4
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
        || (digits.len() > 1 && digits.starts_with('0'))
        || !rest[digits.len()..].starts_with(':')
    {
        return Err(fail("bad-record", "round"));
    }
    digits.parse().map_err(|_| fail("bad-record", "round"))
}

fn mentioned_rounds(text: &str) -> Result<Vec<u32>, PackFailure> {
    let bytes = text.as_bytes();
    let needle = b"NO_V3_";
    let mut found = Vec::new();
    let mut index = 0;
    while index + needle.len() <= bytes.len() {
        if &bytes[index..index + needle.len()] == needle {
            let start = index + needle.len();
            let mut end = start;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            if end == start {
                return Err(fail("bad-record", "round"));
            }
            let digits =
                std::str::from_utf8(&bytes[start..end]).map_err(|_| fail("bad-record", "round"))?;
            found.push(digits.parse().map_err(|_| fail("bad-record", "round"))?);
            index = end;
        } else {
            index += 1;
        }
    }
    Ok(found)
}

fn verify_commit(source: &SelectedSource) -> Result<String, PackFailure> {
    if !is_rev(&source.commit) {
        return Err(fail("bad-record", "commit"));
    }
    relative_path(&source.source)?;
    relative_path(&source.selection)?;
    if source.source == source.selection {
        return Err(fail("bad-record", "selection"));
    }
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
    String::from_utf8(shown.stdout).map_err(|_| fail("bad-record", "blob"))
}

fn encode_coordinates(source: &SelectedSource) -> Result<String, PackFailure> {
    let repo = source
        .repository
        .to_str()
        .ok_or_else(|| fail("bad-record", "repository"))?;
    if repo.is_empty() || repo.contains('\0') || repo.contains('\n') {
        return Err(fail("bad-record", "repository"));
    }
    let mut encoded = String::new();
    receipt::push_field(&mut encoded, repo);
    receipt::push_field(&mut encoded, &source.commit);
    receipt::push_field(&mut encoded, &source.source);
    receipt::push_field(&mut encoded, &source.selection);
    Ok(encoded)
}

fn decode_coordinates(body: &str) -> Result<SelectedSource, PackFailure> {
    if body.contains("completion/claim-") || body.contains("git+") {
        return Err(fail("claim-relabelled", FACT));
    }
    let (repository, rest) = receipt::take_field(body)?;
    let (commit, rest) = receipt::take_field(rest)?;
    let (source, rest) = receipt::take_field(rest)?;
    let (selection, rest) = receipt::take_field(rest)?;
    if !rest.is_empty() {
        return Err(fail("bad-record", FACT));
    }
    Ok(SelectedSource {
        repository: PathBuf::from(repository),
        commit: commit.to_owned(),
        source: source.to_owned(),
        selection: selection.to_owned(),
    })
}

struct SelectedExecution {
    rev: String,
    digest: String,
    round: u32,
    human: String,
    begin: String,
    source: String,
    selection: String,
}

fn decode_execution(body: &str) -> Result<SelectedExecution, PackFailure> {
    let (rev, rest) = receipt::take_field(body)?;
    let (digest, rest) = receipt::take_field(rest)?;
    let (round, rest) = receipt::take_field(rest)?;
    let (human, rest) = receipt::take_field(rest)?;
    let (begin, rest) = receipt::take_field(rest)?;
    let (source, rest) = receipt::take_field(rest)?;
    let (selection, rest) = receipt::take_field(rest)?;
    if !rest.is_empty() || digest.len() != 64 {
        return Err(fail("bad-record", "release.execution"));
    }
    Ok(SelectedExecution {
        rev: rev.to_owned(),
        digest: digest.to_owned(),
        round: round.parse().map_err(|_| fail("bad-record", "round"))?,
        human: human.to_owned(),
        begin: begin.to_owned(),
        source: source.to_owned(),
        selection: selection.to_owned(),
    })
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
#[path = "release_selection_tests.rs"]
mod tests;
