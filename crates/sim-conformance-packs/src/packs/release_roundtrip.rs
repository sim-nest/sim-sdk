// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Family source compiled back out through the authoring compiler.
//!
//! The caller names a commit and the family member paths in that commit.
//! `git rev-parse` and `git show` read those blobs. Both commands pass
//! `--no-replace-objects`. Each blob is parsed by the roadmap runner, then
//! `compile_authoring` compiles the family. The requirement ids are the
//! selected member's own facts. A consulted or superseded member's facts
//! are not requirements. The compiled source bytes must equal the selected
//! blob.
//! The result is `source-roundtrip`. It is not `completion/claim-1`: this
//! compiler reads `family/v1` members, and it does not verify a human
//! signature.

use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};
use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};
use sim_lib_roadmap_runner::{
    Family, FamilyMember, Stance, StatementKind, compile_authoring, parse_member, seal_id,
    source_round_trip, statement_shape,
};

#[path = "release_roundtrip_ground.rs"]
mod ground;

use super::receipt::{self, AdmitExecuted, PackageRow};
use crate::packs::completion_claim::execute_process;
use crate::{CheckObservation, MemorySubject, PackFailure, PackRequest, PackVerdict, find_pack};

const SCOPE: &str = "release/source-roundtrip";
const DOMAIN: &str = "authoring/source-roundtrip";
const FACT: &str = "release.roundtrip";
const RESULT: &str = "source-roundtrip";
const MAX_MEMBERS: usize = 8;
const MAX_FACTS: usize = 64;
const MAX_BLOB: usize = 1_048_576;

/// Coordinates of one family source in a git commit.
pub struct SourceRoundtrip {
    /// Repository that contains the commit.
    pub repository: PathBuf,
    /// Commit object to verify. Forty lowercase hex digits.
    pub commit: String,
    /// Family member paths inside that commit.
    pub paths: Vec<String>,
}

/// Receipt produced by compiling a selected family member from git.
pub struct ObservedSourceRoundtrip {
    /// Subject the receipt names.
    pub subject: String,
    /// `source-roundtrip`. This is not `completion/claim-1`.
    pub result: String,
    /// SHA-256 of the selected member bytes.
    pub source_digest: String,
    /// Selected member name.
    pub name: String,
    /// Selected member round.
    pub round: String,
    /// Requirement ids named by the selected member, in source order.
    pub facts: Vec<String>,
    /// Consulted members whose source seals the compiler recorded.
    pub consultations: usize,
    /// Catalog that admitted the evidence.
    pub catalog: sim_lib_roadmap_runner::ProofCatalog,
    /// Evidence the catalog admitted.
    pub admitted: sim_lib_roadmap_runner::AdmittedEvidence,
    /// Checker receipt that bound the admission.
    pub checker: sim_conformance_v02::CheckerReceiptId,
    /// Scope `release/source-roundtrip`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Reads the family blobs, compiles them, and admits `source-roundtrip`.
///
/// # Errors
///
/// Returns a refusal when git cannot show the blobs, the bytes are not a
/// `family/v1` family, the compiler refuses, or the compiled source differs
/// from the selected blob.
pub fn observe_source_roundtrip(
    subject: &str,
    source: &SourceRoundtrip,
) -> Result<ObservedSourceRoundtrip, PackFailure> {
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

/// Re-checks a source-roundtrip receipt against the catalog that admitted it.
///
/// # Errors
///
/// Returns a refusal when the result, scope, subject, or catalog binding
/// no longer agrees with the admitted receipt.
pub fn bind_source_roundtrip(observation: &ObservedSourceRoundtrip) -> Result<(), PackFailure> {
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
    let mut blobs = Vec::with_capacity(selected.paths.len());
    for path in &selected.paths {
        blobs.push(git_show(&selected.repository, &rev, path)?);
    }
    let compiled = compile_blobs(&blobs)?;
    Ok(vec![observation(
        "release.execution",
        encode_execution(&rev, &compiled, &blobs),
    )])
}

struct Roundtrip {
    name: String,
    round: String,
    facts: Vec<String>,
    consultations: usize,
    digest: String,
    source: Vec<u8>,
}

fn admit_rows(
    subject: &str,
    observations: &[CheckObservation],
) -> Result<ObservedSourceRoundtrip, PackFailure> {
    let encoded = execution(observations)?;
    let decoded = decode_execution(encoded)?;
    let compiled = compile_blobs(&decoded.blobs)?;
    if compiled.digest != decoded.digest
        || compiled.name != decoded.name
        || compiled.round != decoded.round
        || compiled.facts != decoded.facts
        || compiled.consultations != decoded.consultations
        || compiled.source != decoded_selected(&decoded.blobs, &compiled)?
        || !is_rev(&decoded.rev)
    {
        return Err(fail("evidence-mismatch", "release.execution"));
    }
    let rows = vec![PackageRow {
        name: "source-roundtrip".to_owned(),
        version: "0".to_owned(),
        url: "git://source-roundtrip".to_owned(),
        rev: decoded.rev,
    }];
    let admitted = receipt::admit_executed(AdmitExecuted {
        subject,
        result: RESULT,
        scope: SCOPE,
        proof: "source-roundtrip",
        domain: DOMAIN,
        policy: "policy/source-roundtrip",
        provenance: "provenance/source-roundtrip",
        support: "support/source-roundtrip",
        projection: "projection/source-roundtrip",
        environment: "environment/source-roundtrip",
        leaf: encoded.as_bytes(),
        source_text: &receipt::package_source(DOMAIN, "source-roundtrip", &rows),
        definitions: &receipt::package_definitions(DOMAIN, &rows),
    })?;
    Ok(ObservedSourceRoundtrip {
        subject: subject.to_owned(),
        result: RESULT.to_owned(),
        source_digest: compiled.digest,
        name: compiled.name,
        round: compiled.round,
        facts: compiled.facts,
        consultations: compiled.consultations,
        catalog: admitted.catalog,
        admitted: admitted.admitted,
        checker: admitted.checker,
        scope: admitted.scope,
        revocation: admitted.revocation,
    })
}

fn decoded_selected(blobs: &[String], compiled: &Roundtrip) -> Result<Vec<u8>, PackFailure> {
    blobs
        .iter()
        .find(|blob| blob.as_bytes() == compiled.source)
        .map(|blob| blob.as_bytes().to_vec())
        .ok_or_else(|| fail("evidence-mismatch", "source"))
}

fn compile_blobs(blobs: &[String]) -> Result<Roundtrip, PackFailure> {
    if blobs.is_empty() || blobs.len() > MAX_MEMBERS {
        return Err(fail("bound", "member"));
    }
    let mut parsed = Vec::with_capacity(blobs.len());
    for blob in blobs {
        if blob.len() > MAX_BLOB || blob.contains('\0') {
            return Err(fail("bound", "source"));
        }
        parsed.push(parse_member(blob.as_bytes()).map_err(ground::author_error)?);
    }
    let family = Family::from_members(parsed).map_err(ground::author_error)?;
    let anchor = anchor_member(&family)?;
    if anchor.facts.len() > MAX_FACTS {
        return Err(fail("bound", "fact"));
    }
    let spec = ground::roadmap_spec(&anchor)?;
    let files: Vec<(String, Vec<u8>)> = family
        .members()
        .iter()
        .map(|member| (member.path.clone(), member.source.clone()))
        .collect();
    let deck = ground::source_deck(&anchor.owner, &files)?;
    let compiled = compile_authoring(&family, &deck, &spec).map_err(ground::author_error)?;
    let selected = selected_member(&family)?;
    if compiled.source != selected.source
        || source_round_trip(selected) != selected.source.as_slice()
        || compiled.selection.source != selected.source
        || compiled.selection.name != selected.name
        || compiled.selection.round != selected.round
    {
        return Err(fail("evidence-mismatch", "source"));
    }
    let consultation = family
        .members()
        .iter()
        .filter(|member| member.stance == Stance::Consulted)
        .map(|member| seal_id(&member.source))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| fail("authoring-refused", &error.to_string()))?;
    if compiled.consultation != consultation {
        return Err(fail("evidence-mismatch", "consultation"));
    }
    distinct_shapes()?;
    if ground::checkpoint_ids(&spec) != selected.facts {
        return Err(fail("evidence-mismatch", "requirement"));
    }
    Ok(Roundtrip {
        name: selected.name.clone(),
        round: selected.round.clone(),
        facts: selected.facts.clone(),
        consultations: consultation.len(),
        digest: hex_bytes(&Sha256::digest(&selected.source)),
        source: selected.source.clone(),
    })
}

fn anchor_member(family: &Family) -> Result<FamilyMember, PackFailure> {
    let selected: Vec<&FamilyMember> = family
        .members()
        .iter()
        .filter(|member| member.stance == Stance::Selected)
        .collect();
    let anchor = if selected.len() == 1 {
        selected[0]
    } else {
        family
            .members()
            .first()
            .ok_or_else(|| fail("missing-source", "member"))?
    };
    Ok(anchor.clone())
}

fn selected_member(family: &Family) -> Result<&FamilyMember, PackFailure> {
    let mut found = None;
    for member in family.members() {
        if member.stance != Stance::Selected {
            continue;
        }
        if found.is_some() {
            return Err(fail("selection-count", "member"));
        }
        found = Some(member);
    }
    found.ok_or_else(|| fail("selection-count", "member"))
}

fn distinct_shapes() -> Result<(), PackFailure> {
    let advice = statement_shape(StatementKind::Advice);
    let fact = statement_shape(StatementKind::Fact);
    let decision = statement_shape(StatementKind::Decision);
    if advice.expected() == fact.expected()
        || fact.expected() == decision.expected()
        || advice.expected() == decision.expected()
    {
        return Err(fail("shape", "statement"));
    }
    Ok(())
}

fn verify_commit(source: &SourceRoundtrip) -> Result<String, PackFailure> {
    if !is_rev(&source.commit) {
        return Err(fail("bad-record", "commit"));
    }
    if source.paths.is_empty() || source.paths.len() > MAX_MEMBERS {
        return Err(fail("bound", "member"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for path in &source.paths {
        relative_path(path)?;
        if !seen.insert(path.as_str()) {
            return Err(fail("bad-record", "path"));
        }
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

fn encode_coordinates(source: &SourceRoundtrip) -> Result<String, PackFailure> {
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
    push_list(&mut encoded, &source.paths);
    Ok(encoded)
}

fn decode_coordinates(body: &str) -> Result<SourceRoundtrip, PackFailure> {
    if body.contains("completion/claim-") || body.contains("git+") {
        return Err(fail("claim-relabelled", FACT));
    }
    let (repository, rest) = receipt::take_field(body)?;
    let (commit, rest) = receipt::take_field(rest)?;
    let (paths, rest) = take_list(rest, MAX_MEMBERS)?;
    if !rest.is_empty() {
        return Err(fail("bad-record", FACT));
    }
    Ok(SourceRoundtrip {
        repository: PathBuf::from(repository),
        commit: commit.to_owned(),
        paths,
    })
}

struct DecodedExecution {
    rev: String,
    digest: String,
    name: String,
    round: String,
    consultations: usize,
    facts: Vec<String>,
    blobs: Vec<String>,
}

fn encode_execution(rev: &str, compiled: &Roundtrip, blobs: &[String]) -> String {
    let mut encoded = String::new();
    receipt::push_field(&mut encoded, rev);
    receipt::push_field(&mut encoded, &compiled.digest);
    receipt::push_field(&mut encoded, &compiled.name);
    receipt::push_field(&mut encoded, &compiled.round);
    receipt::push_field(&mut encoded, &compiled.consultations.to_string());
    push_list(&mut encoded, &compiled.facts);
    push_list(&mut encoded, blobs);
    encoded
}

fn decode_execution(body: &str) -> Result<DecodedExecution, PackFailure> {
    let (rev, rest) = receipt::take_field(body)?;
    let (digest, rest) = receipt::take_field(rest)?;
    let (name, rest) = receipt::take_field(rest)?;
    let (round, rest) = receipt::take_field(rest)?;
    let (consultations, rest) = receipt::take_field(rest)?;
    let (facts, rest) = take_list(rest, MAX_FACTS)?;
    let (blobs, rest) = take_list(rest, MAX_MEMBERS)?;
    if !rest.is_empty() || digest.len() != 64 {
        return Err(fail("bad-record", "release.execution"));
    }
    let consultations = consultations
        .parse()
        .map_err(|_| fail("bad-record", "consultation"))?;
    if consultations > MAX_MEMBERS {
        return Err(fail("bound", "consultation"));
    }
    Ok(DecodedExecution {
        rev: rev.to_owned(),
        digest: digest.to_owned(),
        name: name.to_owned(),
        round: round.to_owned(),
        consultations,
        facts,
        blobs,
    })
}

fn push_list(out: &mut String, items: &[String]) {
    receipt::push_field(out, &items.len().to_string());
    for item in items {
        receipt::push_field(out, item);
    }
}

fn take_list(input: &str, max: usize) -> Result<(Vec<String>, &str), PackFailure> {
    let (count_text, mut rest) = receipt::take_field(input)?;
    if count_text.len() > 8 || !count_text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(fail("bad-record", "count"));
    }
    let count: usize = count_text
        .parse()
        .map_err(|_| fail("bad-record", "count"))?;
    if count > max {
        return Err(fail("bound", "count"));
    }
    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        let (item, next) = receipt::take_field(rest)?;
        items.push(item.to_owned());
        rest = next;
    }
    Ok((items, rest))
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
#[path = "release_roundtrip_tests.rs"]
mod tests;
