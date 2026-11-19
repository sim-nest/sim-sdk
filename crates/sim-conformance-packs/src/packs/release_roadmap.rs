// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Claim 1 for the source the recorded acts authorize.
//!
//! The human selection names its owner record. That record names the begin
//! record and the product contract. The source path is that contract, and
//! the source digest is the digest that owner record states for the
//! authorized bytes. A file that copies the sentences, and a round-18
//! example committed beside them, are not those records. `git show` of an
//! unrelated blob is not authority. The result is `completion/claim-1`
//! only after the owner records and the authorized source agree.
//! A short token record stays on `release/source-selection`.

use sha2::{Digest, Sha256};
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
const SOURCE_PATH: &str = "docs/workbench/NO_V3_18.md";
const SOURCE_DIGEST: &str = "4c0b9ad283be893cf4ccbe87e49bf2adc3e206a923c78b05e1dcaaf554fb0052";
const CONTRACT: &str = "`NO_V3_18.md` remains the selected product contract";
const RUN_MARKER: &str = "docs/workbench/NV12_RUN_";

/// Receipt admitted after the owner records authorize the source.
pub struct ObservedRoadmapAuthority {
    /// Subject the receipt names.
    pub subject: String,
    /// `completion/claim-1`.
    pub result: String,
    /// SHA-256 of the authorized source blob.
    pub source_digest: String,
    /// Round named by the source title. This reader admits only 18.
    pub round: u32,
    /// Human selection recorded by its owner record.
    pub human_selection: String,
    /// Begin authority recorded by its own owner record.
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

/// Reads the owner records and admits claim 1 for the source they authorize.
///
/// # Errors
///
/// Returns a refusal when the named paths are not the owner records, git
/// cannot show those blobs, the records do not contain the two acts, the
/// source is not the authorized bytes, the source is not round 18, or
/// another NO_V3 round appears.
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
    let bound_records = load_records(&selected, &rev)?;
    let mut encoded = String::new();
    receipt::push_field(&mut encoded, &rev);
    receipt::push_field(&mut encoded, &bound_records.digest);
    receipt::push_field(&mut encoded, &bound_records.round.to_string());
    receipt::push_field(&mut encoded, &bound_records.human);
    receipt::push_field(&mut encoded, &bound_records.begin);
    receipt::push_field(&mut encoded, &bound_records.source);
    receipt::push_field(&mut encoded, &bound_records.handoff);
    receipt::push_field(&mut encoded, &bound_records.run);
    Ok(vec![observation("completion.execution", encoded)])
}

struct BoundRecords {
    digest: String,
    round: u32,
    human: String,
    begin: String,
    source: String,
    handoff: String,
    run: String,
}

fn load_records(selected: &SelectedSource, rev: &str) -> Result<BoundRecords, PackFailure> {
    let handoff_path = act_path(HUMAN)?;
    if selected.selection != handoff_path {
        return Err(fail("bad-record", "human-selection"));
    }
    if selected.source != SOURCE_PATH {
        return Err(fail("bad-record", "source"));
    }
    let source = selection::git_show(&selected.repository, rev, &selected.source)?;
    let handoff = selection::git_show(&selected.repository, rev, &selected.selection)?;
    if !records(&handoff, HUMAN) {
        return Err(fail("bad-record", "human-selection"));
    }
    if !records(&handoff, CONTRACT) {
        return Err(fail("bad-record", "source"));
    }
    let run_path = one_run_path(&handoff)?;
    if run_path == handoff_path || run_path == SOURCE_PATH {
        return Err(fail("bad-record", "begin-authority"));
    }
    let run = selection::git_show(&selected.repository, rev, &run_path)?;
    if run == handoff {
        return Err(fail("selection-begin", "selection"));
    }
    confirm_begin(&run)?;
    let (round, digest) = selection::parse_source(&source)?;
    if round != 18 {
        return Err(fail("bad-record", "round"));
    }
    if digest != SOURCE_DIGEST || !handoff.contains(SOURCE_DIGEST) {
        return Err(fail("bad-record", "source-digest"));
    }
    let hashed = hex_bytes(Sha256::digest(source.as_bytes()).as_slice());
    if hashed != digest {
        return Err(fail("evidence-mismatch", "source-digest"));
    }
    Ok(BoundRecords {
        digest,
        round,
        human: HUMAN.to_owned(),
        begin: BEGIN.to_owned(),
        source,
        handoff,
        run,
    })
}

fn confirm_begin(run: &str) -> Result<(), PackFailure> {
    if HUMAN == BEGIN {
        return Err(fail("selection-begin", "selection"));
    }
    if records(run, BEGIN) {
        return Ok(());
    }
    if records(run, HUMAN) {
        return Err(fail("selection-begin", "selection"));
    }
    Err(fail("bad-record", "begin-authority"))
}

fn act_path(act: &str) -> Result<&str, PackFailure> {
    let rest = act
        .strip_prefix("I select ")
        .ok_or_else(|| fail("bad-record", "human-selection"))?;
    let path = rest
        .split_once(" as the authoritative")
        .map(|(path, _)| path)
        .ok_or_else(|| fail("bad-record", "human-selection"))?;
    if path.is_empty() || path.contains(' ') || path.contains('\\') {
        return Err(fail("bad-record", "human-selection"));
    }
    Ok(path)
}

fn one_run_path(handoff: &str) -> Result<String, PackFailure> {
    let mut found = None;
    let mut rest = handoff;
    while let Some(index) = rest.find(RUN_MARKER) {
        let tail = &rest[index + RUN_MARKER.len()..];
        let end = tail
            .find(".md")
            .ok_or_else(|| fail("bad-record", "begin-authority"))?;
        let id = &tail[..end];
        if id.is_empty()
            || !id
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        {
            return Err(fail("bad-record", "begin-authority"));
        }
        let path = format!("{RUN_MARKER}{id}.md");
        if path == SOURCE_PATH {
            return Err(fail("bad-record", "begin-authority"));
        }
        match &found {
            Some(previous) if previous != &path => {
                return Err(fail("bad-record", "begin-authority"));
            }
            None => found = Some(path),
            Some(_) => {}
        }
        rest = &tail[end + 3..];
    }
    found.ok_or_else(|| fail("bad-record", "begin-authority"))
}

fn records(record: &str, act: &str) -> bool {
    collapse(record).contains(act)
}

fn collapse(text: &str) -> String {
    let mut out = String::new();
    let mut pending_space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            pending_space = !out.is_empty();
            continue;
        }
        if pending_space {
            out.push(' ');
            pending_space = false;
        }
        out.push(ch);
    }
    out
}

fn admit_rows(
    subject: &str,
    observations: &[CheckObservation],
) -> Result<ObservedRoadmapAuthority, PackFailure> {
    let encoded = execution(observations)?;
    let decoded = decode_execution(encoded)?;
    if !records(&decoded.handoff, HUMAN)
        || !records(&decoded.handoff, CONTRACT)
        || one_run_path(&decoded.handoff).is_err()
        || confirm_begin(&decoded.run).is_err()
    {
        return Err(fail("evidence-mismatch", "completion.execution"));
    }
    let (parsed_round, parsed_digest) = selection::parse_source(&decoded.source)?;
    if parsed_round != 18
        || parsed_round != decoded.round
        || parsed_digest != decoded.digest
        || parsed_digest != SOURCE_DIGEST
        || !decoded.handoff.contains(SOURCE_DIGEST)
        || decoded.human != HUMAN
        || decoded.begin != BEGIN
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
    handoff: String,
    run: String,
}

fn decode_execution(body: &str) -> Result<RoadmapExecution, PackFailure> {
    let (rev, rest) = receipt::take_field(body)?;
    let (digest, rest) = receipt::take_field(rest)?;
    let (round, rest) = receipt::take_field(rest)?;
    let (human, rest) = receipt::take_field(rest)?;
    let (begin, rest) = receipt::take_field(rest)?;
    let (source, rest) = receipt::take_field(rest)?;
    let (handoff, rest) = receipt::take_field(rest)?;
    let (run, rest) = receipt::take_field(rest)?;
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
        handoff: handoff.to_owned(),
        run: run.to_owned(),
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

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
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
