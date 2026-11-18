// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Replay-digest observations for one executed portable phase union.
//!
//! The pack reads the injected replay body. It does not read a caller
//! boolean, a source-deck label, a manifest, or a completion claim. Each
//! line is one executed leaf, `phase/NN` plus its digest, from `phase/00`
//! through `phase/28`. The receipt subject is the subject the caller names.
//! Its result is `completion/claim-17`. A missing or refused body produces
//! no receipt.

use std::collections::BTreeSet;

use sha2::{Digest, Sha256};
use sim_conformance_v02::{
    CheckArgument, CheckInputClosureId, CheckScopeId, CheckTemplate, CheckedSubjectId,
    CheckerBinding, CheckerReceipt, CheckerReceiptId, CheckerResultId, CommandId,
    ConformancePackId, EnvironmentPolicyId, EvidenceGrade, EvidenceProvenanceId, EvidenceSetId,
    OutputShapeId, OwnerBindingId, PolicyId, ProofCodeId, RevocationSourceId, RevocationStatus,
    WorkingDirectoryPolicyId,
};
use sim_kernel::ContentId;
use sim_lib_roadmap_runner::{
    AdmittedEvidence, DiscoveredDefinition, EvidenceClaim, EvidenceDecision, ProofCatalog,
    ProofLeaf, RetainedCheckerReceipt,
};

use crate::{CheckObservation, MemorySubject, PackFailure, PackRequest, PackVerdict, find_pack};

const PHASES: usize = 29;
const MAX_BYTES: usize = 65_536;
const SCOPE: &str = "completion/replay";
const FACT: &str = "release.replay-body";
const RESULT: &str = "completion/claim-17";
const DOMAIN: &str = "completion/replay";

struct Leaf {
    name: String,
    digest: String,
}

/// Receipt the proof catalog admitted for one observed phase-union replay.
pub struct ObservedPhaseReplay {
    /// Subject the checker invocation and the receipt both name.
    pub subject: String,
    /// `completion/claim-17` for this subject.
    ///
    /// The digest stays in the admitted replay body. It is not a tail on
    /// this result.
    pub result: String,
    /// Catalog that admitted the evidence.
    pub catalog: ProofCatalog,
    /// Evidence `admit_evidence` accepted.
    pub admitted: AdmittedEvidence,
    /// Checker receipt that bound the admission.
    pub checker: CheckerReceiptId,
    /// Scope recorded on that receipt.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Checks one injected replay body and admits the derived claim receipt.
///
/// # Errors
///
/// Returns a pack refusal when the body is missing a required leaf, names a
/// publication or the construction specimen, or the catalog refuses the
/// constructed receipt.
pub fn observe_phase_replay(subject: &str, body: &str) -> Result<ObservedPhaseReplay, PackFailure> {
    let checked =
        CheckedSubjectId::from_text(subject).map_err(|_| fail("subject-identity", subject))?;
    let rendered = render(checked.content_id());
    let spec = find_pack("checker/c-release")
        .ok_or_else(|| fail("unknown-checker", "checker/c-release"))?;
    let binding = spec
        .checker_binding()
        .map_err(|error| fail("invalid-static-binding", &error.to_string()))?;
    let binding_id = render(binding.id().content_id());
    let evidence = MemorySubject::default().with(FACT, body);
    let verdict = super::check(&PackRequest {
        checker: "checker/c-release",
        binding: &binding_id,
        subject: &rendered,
        scope: SCOPE,
        evidence: &evidence,
    });
    let observations = match verdict {
        PackVerdict::Pass { observations, .. } => observations,
        PackVerdict::Refused(failure) => return Err(failure),
        PackVerdict::UnimplementedPack { .. } => return Err(fail("unimplemented", SCOPE)),
    };
    admit_parsed(subject, body, &observations)
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
        return Err(fail("missing-replay", FACT));
    };
    let leaves = parse_body(body)?;
    let observations = leaves
        .iter()
        .map(|leaf| observation("release.replay", leaf.line()))
        .collect::<Vec<_>>();
    super::refuse_claim_values(&observations)?;
    Ok(observations)
}

fn admit_parsed(
    subject: &str,
    body: &str,
    observations: &[CheckObservation],
) -> Result<ObservedPhaseReplay, PackFailure> {
    let leaves = parse_body(body)?;
    let lines = leaves.iter().map(Leaf::line).collect::<Vec<_>>();
    let mut observed = observations
        .iter()
        .filter(|item| item.key == "release.replay")
        .map(|item| item.value.clone())
        .collect::<Vec<_>>();
    observed.sort();
    let mut expected = lines.clone();
    expected.sort();
    if observed != expected || observations.len() != leaves.len() {
        return Err(fail("evidence-mismatch", "release.replay"));
    }
    let found = discovered(&leaves);
    if found.len() != leaves.len() {
        return Err(fail("missing-replay", FACT));
    }
    let leaf = ProofLeaf::ArtifactEquality {
        name: "replay-digest".into(),
        left: body.as_bytes().to_vec(),
        right: body.as_bytes().to_vec(),
    };
    let scope = CheckScopeId::from_text(SCOPE)
        .map_err(|error| fail("receipt-refused", &error.to_string()))?;
    let subject_id = CheckedSubjectId::from_text(subject)
        .map_err(|error| fail("subject-identity", &error.to_string()))?;
    let policy = PolicyId::from_text("policy/replay-digest")
        .map_err(|error| fail("receipt-refused", &error.to_string()))?;
    let mut shape_members = found
        .iter()
        .map(|row| row.semantic.clone())
        .collect::<Vec<_>>();
    shape_members.sort();
    shape_members.dedup();
    let claim = EvidenceClaim {
        proof: "replay-digest".into(),
        subject: subject_id.clone(),
        scope: scope.clone(),
        checker_receipt: CheckerReceiptId::from_text("receipt/unused")
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        projections: vec!["projection/replay".into()],
        environment: "environment/replay-digest".into(),
        source_text: source_text(&leaves),
        members: shape_members,
        domains: vec![DOMAIN.into()],
        policies: vec![policy.clone()],
        supports: Vec::new(),
        telemetry: String::new(),
        requested_grade: None,
    };
    let selected = found.iter().collect::<Vec<_>>();
    let bound = input_closure(&leaf, &claim, &selected)?;
    let binding = receipt_binding(&scope)?;
    let invocation = binding
        .instantiate(
            ProofCodeId::from_text("code/replay-digest")
                .map_err(|error| fail("receipt-refused", &error.to_string()))?,
            ConformancePackId::from_text("pack/release")
                .map_err(|error| fail("receipt-refused", &error.to_string()))?,
            subject_id,
            scope.clone(),
            bound,
        )
        .map_err(|error| fail("receipt-refused", &error.to_string()))?;
    let receipt = CheckerReceipt::passing(
        &invocation,
        CheckerResultId::from_text(RESULT)
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        EvidenceGrade::Release,
        EvidenceProvenanceId::from_text("provenance/replay-digest")
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        policy,
        EvidenceSetId::from_text("support/replay-digest")
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        RevocationStatus::Current,
    )
    .map_err(|error| fail("receipt-refused", &error.to_string()))?;
    receipt
        .verify(&binding, &invocation, RevocationStatus::Current)
        .map_err(|error| fail("receipt-refused", &error.to_string()))?;
    let checker = receipt.id().clone();
    let mut catalog = ProofCatalog::new([leaf])
        .map_err(|error| fail("receipt-refused", &error.to_string()))?
        .with_checker_receipt(
            RetainedCheckerReceipt::new(binding, invocation, receipt),
            RevocationStatus::Current,
        )
        .map_err(|error| fail("receipt-refused", &error.to_string()))?;
    let admitted = catalog
        .admit_evidence(claim_with(&claim, checker.clone()), current)
        .map_err(|error| fail("receipt-refused", &error.to_string()))?;
    match catalog.decide_evidence(&admitted.key, EvidenceGrade::Release, current) {
        EvidenceDecision::Reuse { receipt } if receipt == admitted.id => {}
        EvidenceDecision::Reuse { .. }
        | EvidenceDecision::Recheck { .. }
        | EvidenceDecision::Refuse { .. } => {
            return Err(fail("receipt-refused", "release-reuse"));
        }
    }
    Ok(ObservedPhaseReplay {
        subject: subject.to_owned(),
        result: RESULT.to_owned(),
        catalog,
        admitted,
        checker,
        scope,
        revocation: RevocationStatus::Current,
    })
}

fn claim_with(claim: &EvidenceClaim, checker: CheckerReceiptId) -> EvidenceClaim {
    EvidenceClaim {
        proof: claim.proof.clone(),
        subject: claim.subject.clone(),
        scope: claim.scope.clone(),
        checker_receipt: checker,
        projections: claim.projections.clone(),
        environment: claim.environment.clone(),
        source_text: claim.source_text.clone(),
        members: claim.members.clone(),
        domains: claim.domains.clone(),
        policies: claim.policies.clone(),
        supports: claim.supports.clone(),
        telemetry: claim.telemetry.clone(),
        requested_grade: claim.requested_grade,
    }
}

fn current(_: &CheckerReceiptId) -> RevocationStatus {
    RevocationStatus::Current
}

fn receipt_binding(scope: &CheckScopeId) -> Result<CheckerBinding, PackFailure> {
    CheckerBinding::new(
        "checker/c-release".into(),
        OwnerBindingId::from_text("owner/replay-digest")
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        "packs::release::check".into(),
        vec![
            ConformancePackId::from_text("pack/release")
                .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        ],
        OutputShapeId::from_text("shape/check-receipt")
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        RevocationSourceId::from_text("revocation/replay-digest")
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        CommandId::from_text("command/validate")
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        CommandId::from_text("command/docs")
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        BTreeSet::from([scope.clone()]),
        CheckTemplate::new(
            "check".into(),
            vec![
                CheckArgument::BindingSlot,
                CheckArgument::SubjectSlot,
                CheckArgument::ScopeSlot,
            ],
            WorkingDirectoryPolicyId::from_text("cwd/replay")
                .map_err(|error| fail("receipt-refused", &error.to_string()))?,
            EnvironmentPolicyId::from_text("environment/sealed")
                .map_err(|error| fail("receipt-refused", &error.to_string()))?,
            OutputShapeId::from_text("shape/check-result")
                .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        )
        .map_err(|error| fail("receipt-refused", &error.to_string()))?,
    )
    .map_err(|error| fail("receipt-refused", &error.to_string()))
}

fn parse_body(text: &str) -> Result<Vec<Leaf>, PackFailure> {
    if text.len() > MAX_BYTES {
        return Err(fail("bound", FACT));
    }
    if text.contains('\r') {
        return Err(fail("bad-record", FACT));
    }
    if text.contains("shade-1") {
        return Err(fail("construction-specimen", FACT));
    }
    if text.contains("PortableClosed")
        || text.contains("publication-confirmed")
        || text.contains("completion/claim-")
    {
        return Err(fail("local-publication-claim", FACT));
    }
    if text.is_empty() || text.chars().all(|ch| ch == '\n') {
        return Err(fail("missing-replay", FACT));
    }
    if !text.ends_with('\n') {
        return Err(fail("bad-record", FACT));
    }
    let mut parts: Vec<&str> = text.split('\n').collect();
    if parts.last().copied() == Some("") {
        parts.pop();
    }
    if parts.iter().any(|line| line.is_empty()) || parts.len() != PHASES {
        return Err(fail("bad-record", FACT));
    }
    parts
        .iter()
        .enumerate()
        .map(|(index, line)| checked_line(index, line))
        .collect()
}

fn checked_line(index: usize, line: &str) -> Result<Leaf, PackFailure> {
    let Some((name, digest)) = line.split_once(' ') else {
        return Err(fail("bad-record", "release.replay"));
    };
    if digest.contains(' ') || name != format!("phase/{index:02}") || !is_digest(digest) {
        return Err(fail("bad-record", "release.replay"));
    }
    Ok(Leaf {
        name: name.to_owned(),
        digest: digest.to_owned(),
    })
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

impl Leaf {
    fn line(&self) -> String {
        format!("{} {}", self.name, self.digest)
    }
}

fn discovered(leaves: &[Leaf]) -> Vec<DiscoveredDefinition> {
    leaves
        .iter()
        .map(|leaf| DiscoveredDefinition {
            class: "package",
            domain: DOMAIN.to_owned(),
            name: leaf.name.clone(),
            semantic: semantic("package", &[DOMAIN, "git", leaf.name.as_str(), "0", "leaf"]),
            storage: leaf.digest.clone(),
        })
        .collect()
}

fn source_text(leaves: &[Leaf]) -> String {
    let mut text = String::from(
        "sim-world-inputs-v1\nroot-package replay-digest\ndomain completion/replay . compiling replay\n",
    );
    for leaf in leaves {
        text.push_str("member completion/replay git ");
        text.push_str(&leaf.name);
        text.push_str(" 0 leaf ");
        text.push_str(&leaf.digest);
        text.push_str(" git leaf\n");
    }
    text
}

fn semantic(class: &str, fields: &[&str]) -> String {
    let mut out = String::new();
    push_field(&mut out, class);
    for field in fields {
        push_field(&mut out, field);
    }
    out
}

fn push_field(out: &mut String, field: &str) {
    out.push_str(&field.len().to_string());
    out.push(':');
    out.push_str(field);
    out.push(';');
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

fn render(id: &ContentId) -> String {
    format!(
        "{}:{}",
        id.algorithm.as_qualified_str(),
        hex_bytes(&id.bytes)
    )
}

fn input_closure(
    leaf: &ProofLeaf,
    claim: &EvidenceClaim,
    selected: &[&DiscoveredDefinition],
) -> Result<CheckInputClosureId, PackFailure> {
    let definition = proof_definition(leaf)?;
    let code = proof_code(leaf)?;
    let projections = digest_text(&sorted_join(&claim.projections));
    let environment = digest_text(&claim.environment);
    let universe = digest_text(&universe_text(selected));
    let text = canon(&[
        ("subject", &hex(claim.subject.content_id())),
        ("scope", &hex(claim.scope.content_id())),
        ("proof", &digest_text(&definition)),
        ("proof-code", &digest_text(&code)),
        ("projections", &projections),
        ("environment", &environment),
        ("universe", &universe),
    ]);
    CheckInputClosureId::from_text(&text)
        .map_err(|error| fail("receipt-refused", &error.to_string()))
}

fn proof_definition(leaf: &ProofLeaf) -> Result<String, PackFailure> {
    let ProofLeaf::ArtifactEquality { name, .. } = leaf else {
        return Err(fail("receipt-refused", "leaf"));
    };
    Ok(canon(&[
        ("kind", "artifact-equality"),
        ("name", name.as_str()),
    ]))
}

fn proof_code(leaf: &ProofLeaf) -> Result<String, PackFailure> {
    let ProofLeaf::ArtifactEquality { name, left, right } = leaf else {
        return Err(fail("receipt-refused", "leaf"));
    };
    let left = hex_bytes(left);
    let right = hex_bytes(right);
    Ok(canon(&[
        ("name", name.as_str()),
        ("left", &left),
        ("right", &right),
    ]))
}

fn universe_text(selected: &[&DiscoveredDefinition]) -> String {
    let mut rows = selected.to_vec();
    rows.sort_by(|left, right| left.semantic.cmp(&right.semantic));
    rows.dedup_by(|left, right| left.semantic == right.semantic);
    let rendered = rows
        .iter()
        .map(|row| {
            canon(&[
                ("semantic", row.semantic.as_str()),
                ("storage", row.storage.as_str()),
            ])
        })
        .collect::<Vec<_>>();
    length_prefixed(&rendered)
}

fn sorted_join(values: &[String]) -> String {
    let mut values = values.to_vec();
    values.sort();
    length_prefixed(&values)
}

fn length_prefixed(values: &[String]) -> String {
    let mut out = String::new();
    for value in values {
        out.push_str(&value.len().to_string());
        out.push(':');
        out.push_str(value);
        out.push(';');
    }
    out
}

fn canon(fields: &[(&str, &str)]) -> String {
    let mut out = String::new();
    for (name, value) in fields {
        out.push_str(name);
        out.push('=');
        out.push_str(&value.len().to_string());
        out.push(':');
        out.push_str(value);
        out.push(';');
    }
    out
}

fn hex(id: &ContentId) -> String {
    hex_bytes(&id.bytes)
}

fn digest_text(text: &str) -> String {
    hex_bytes(Sha256::digest(text.as_bytes()).as_slice())
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
#[path = "release_replay_tests.rs"]
mod tests;
