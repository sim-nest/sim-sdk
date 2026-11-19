// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Claim 1 for one selected round-18 source, read from git objects.
//!
//! The caller names a commit, a source path, and a selection path. `git
//! rev-parse` and `git show` read those blobs with `--no-replace-objects`.
//! The source digest is the SHA-256 of the blob. The source title round is
//! 18, every `NO_V3_<round>` marker is that round, and claim lines 1
//! through 25 are present. The selection record is exactly two different
//! lines: the recorded human selection and the recorded begin authority.
//! Those lines are the full sentences. A short token record stays on
//! `release/source-selection` and is not this result. The result is
//! `completion/claim-1` only after the blobs pass these checks.

use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};
use sim_lib_roadmap_runner::{AdmittedEvidence, ProofCatalog};

use super::receipt::{self, AdmitExecuted, PackageRow};
use super::selection::{self, SelectedSource};
use crate::{CheckObservation, MemorySubject, PackFailure, PackRequest, PackVerdict, find_pack};

const SCOPE: &str = "completion/roadmap";
const DOMAIN: &str = "completion/roadmap";
const FACT: &str = "completion.roadmap";
const RESULT: &str = "completion/claim-1";
const HUMAN: &str = "I select docs/workbench/NV12_FINISH_1.md as the authoritative execution handoff, including its verification simplification.";
const BEGIN: &str = "This starts the fresh worker; the previous worker remains paused. No additional maintainer-start sentence is required.";

/// Receipt admitted after the selected source and the two acts agree.
pub struct ObservedRoadmapAuthority {
    /// Subject the receipt names.
    pub subject: String,
    /// `completion/claim-1`.
    pub result: String,
    /// SHA-256 of the source blob.
    pub source_digest: String,
    /// Round named by the source title. This reader admits only 18.
    pub round: u32,
    /// Recorded human selection, as its own line.
    pub human_selection: String,
    /// Recorded begin authority, as its own line.
    pub begin_authority: String,
    /// Catalog that admitted the evidence.
    pub catalog: ProofCatalog,
    /// Evidence the catalog admitted.
    pub admitted: AdmittedEvidence,
    /// Checker receipt that bound the admission.
    pub checker: sim_conformance_v02::CheckerReceiptId,
    /// Scope `completion/roadmap`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Reads the source and the two-act record and admits claim 1.
///
/// # Errors
///
/// Returns a refusal when git cannot show the blobs, the source is not
/// round 18, another NO_V3 round appears, a claim line is missing, or the
/// selection lines are not the two recorded sentences.
pub fn observe_roadmap_authority(
    subject: &str,
    source: &SelectedSource,
) -> Result<ObservedRoadmapAuthority, PackFailure> {
    let checked = CheckedSubjectId::from_text(subject)
        .map_err(|error| fail("subject-identity", &error.to_string()))?;
    let spec = find_pack("checker/c-release")
        .ok_or_else(|| fail("unknown-checker", "checker/c-release"))?;
    let binding = spec
        .checker_binding()
        .map_err(|error| fail("invalid-static-binding", &error.to_string()))?;
    let evidence = MemorySubject::default().with(FACT, encode_coordinates(subject, source)?);
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

/// Re-checks a claim-1 receipt against the catalog that admitted it.
///
/// # Errors
///
/// Returns a refusal when the result or scope is no longer claim 1.
pub fn bind_roadmap_authority(observation: &ObservedRoadmapAuthority) -> Result<(), PackFailure> {
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
    if body.contains("completion/claim-") || body.contains("git+") {
        return Err(fail("claim-relabelled", FACT));
    }
    let (subject, selected) = decode_coordinates(body)?;
    let bound =
        CheckedSubjectId::from_text(&subject).map_err(|_| fail("subject-mismatch", FACT))?;
    if receipt::render(bound.content_id()) != request.subject {
        return Err(fail("subject-mismatch", FACT));
    }
    let rev = selection::verify_commit(&selected)?;
    let source = selection::git_show(&selected.repository, &rev, &selected.source)?;
    let selection_blob = selection::git_show(&selected.repository, &rev, &selected.selection)?;
    let (round, digest) = selection::parse_source(&source)?;
    if round != 18 {
        return Err(fail("bad-record", "round"));
    }
    let (human, begin) = parse_acts(&selection_blob)?;
    let mut encoded = String::new();
    receipt::push_field(&mut encoded, &rev);
    receipt::push_field(&mut encoded, &digest);
    receipt::push_field(&mut encoded, &round.to_string());
    receipt::push_field(&mut encoded, &human);
    receipt::push_field(&mut encoded, &begin);
    receipt::push_field(&mut encoded, &source);
    receipt::push_field(&mut encoded, &selection_blob);
    Ok(vec![observation("completion.execution", encoded)])
}

fn admit_rows(
    subject: &str,
    observations: &[CheckObservation],
) -> Result<ObservedRoadmapAuthority, PackFailure> {
    let encoded = execution(observations)?;
    let decoded = decode_execution(encoded)?;
    let (parsed_round, parsed_digest) = selection::parse_source(&decoded.source)?;
    let (parsed_human, parsed_begin) = parse_acts(&decoded.selection)?;
    if parsed_round != 18
        || parsed_round != decoded.round
        || parsed_digest != decoded.digest
        || parsed_human != decoded.human
        || parsed_begin != decoded.begin
        || !is_rev(&decoded.rev)
    {
        return Err(fail("evidence-mismatch", "completion.execution"));
    }
    let rows = vec![PackageRow {
        name: "roadmap-authority".to_owned(),
        version: "0".to_owned(),
        url: "git://roadmap-authority".to_owned(),
        rev: decoded.rev,
    }];
    let admitted = receipt::admit_executed(AdmitExecuted {
        subject,
        result: RESULT,
        scope: SCOPE,
        proof: "roadmap-authority",
        domain: DOMAIN,
        policy: "policy/roadmap-authority",
        provenance: "provenance/roadmap-authority",
        support: "support/roadmap-authority",
        projection: "projection/roadmap-authority",
        environment: "environment/roadmap-authority",
        leaf: encoded.as_bytes(),
        source_text: &receipt::package_source(DOMAIN, "roadmap-authority", &rows),
        definitions: &receipt::package_definitions(DOMAIN, &rows),
    })?;
    Ok(ObservedRoadmapAuthority {
        subject: subject.to_owned(),
        result: RESULT.to_owned(),
        source_digest: decoded.digest,
        round: decoded.round,
        human_selection: decoded.human,
        begin_authority: decoded.begin,
        catalog: admitted.catalog,
        admitted: admitted.admitted,
        checker: admitted.checker,
        scope: admitted.scope,
        revocation: admitted.revocation,
    })
}

fn parse_acts(blob: &str) -> Result<(String, String), PackFailure> {
    if blob.len() > 1024 || blob.contains('\0') || blob.contains('\r') {
        return Err(fail("bound", "selection"));
    }
    let text = blob.strip_suffix('\n').unwrap_or(blob);
    let mut lines = text.lines();
    let human = lines
        .next()
        .filter(|line| !line.is_empty())
        .ok_or_else(|| fail("bad-record", "human-selection"))?;
    let begin = lines
        .next()
        .filter(|line| !line.is_empty())
        .ok_or_else(|| fail("bad-record", "begin-authority"))?;
    if lines.next().is_some() {
        return Err(fail("bad-record", "selection"));
    }
    if human == begin {
        return Err(fail("selection-begin", "selection"));
    }
    if human != HUMAN {
        return Err(fail("bad-record", "human-selection"));
    }
    if begin != BEGIN {
        return Err(fail("bad-record", "begin-authority"));
    }
    Ok((human.to_owned(), begin.to_owned()))
}

fn encode_coordinates(subject: &str, source: &SelectedSource) -> Result<String, PackFailure> {
    if subject.is_empty()
        || subject.contains('\0')
        || subject.contains('\n')
        || subject.contains("completion/claim-")
    {
        return Err(fail("bad-record", "subject"));
    }
    let repo = source
        .repository
        .to_str()
        .ok_or_else(|| fail("bad-record", "repository"))?;
    if repo.is_empty() || repo.contains('\0') || repo.contains('\n') {
        return Err(fail("bad-record", "repository"));
    }
    let mut encoded = String::new();
    receipt::push_field(&mut encoded, subject);
    receipt::push_field(&mut encoded, repo);
    receipt::push_field(&mut encoded, &source.commit);
    receipt::push_field(&mut encoded, &source.source);
    receipt::push_field(&mut encoded, &source.selection);
    Ok(encoded)
}

fn decode_coordinates(body: &str) -> Result<(String, SelectedSource), PackFailure> {
    let (subject, rest) = receipt::take_field(body)?;
    let (repository, rest) = receipt::take_field(rest)?;
    let (commit, rest) = receipt::take_field(rest)?;
    let (source, rest) = receipt::take_field(rest)?;
    let (selection, rest) = receipt::take_field(rest)?;
    if !rest.is_empty() {
        return Err(fail("bad-record", FACT));
    }
    Ok((
        subject.to_owned(),
        SelectedSource {
            repository: std::path::PathBuf::from(repository),
            commit: commit.to_owned(),
            source: source.to_owned(),
            selection: selection.to_owned(),
        },
    ))
}

struct RoadmapExecution {
    rev: String,
    digest: String,
    round: u32,
    human: String,
    begin: String,
    source: String,
    selection: String,
}

fn decode_execution(body: &str) -> Result<RoadmapExecution, PackFailure> {
    let (rev, rest) = receipt::take_field(body)?;
    let (digest, rest) = receipt::take_field(rest)?;
    let (round, rest) = receipt::take_field(rest)?;
    let (human, rest) = receipt::take_field(rest)?;
    let (begin, rest) = receipt::take_field(rest)?;
    let (source, rest) = receipt::take_field(rest)?;
    let (selection, rest) = receipt::take_field(rest)?;
    if !rest.is_empty() || digest.len() != 64 {
        return Err(fail("bad-record", "completion.execution"));
    }
    Ok(RoadmapExecution {
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
        .find(|item| item.key == "completion.execution")
        .map(|item| item.value.as_str())
        .ok_or_else(|| fail("missing-source", "completion.execution"))
}

fn is_rev(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
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
#[path = "release_roadmap_tests.rs"]
mod tests;
