// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Journal completion observations for one executed export.
//!
//! The pack reads the injected export, republishes it, and checks replay,
//! reopen, snapshot, shared references, and telemetry removal by executing
//! them. It does not read a caller boolean, a phase digest list, a manifest,
//! or a completion claim. The export names one subject and carries that
//! subject's bytes as its shared object. The registered request subject is
//! the rendered content id of that subject, and the caller argument must
//! name the same subject. The result is `completion/claim-17`. A missing or
//! refused export produces no receipt.

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

#[path = "release_journal.rs"]
mod journal;

pub use journal::{ExecutedEntry, ExecutedJournal, format_executed_journal};

const SCOPE: &str = "completion/replay";
const RESULT: &str = "completion/claim-17";
const DOMAIN: &str = "completion/replay";
const PROOF: &str = "journal-replay";

/// Receipt the proof catalog admitted for one re-executed journal.
pub struct ObservedJournalCompletion {
    /// Subject the checker invocation and the receipt both name.
    pub subject: String,
    /// `completion/claim-17` for this subject.
    ///
    /// The replay encoding stays in the proof leaf. It is not a tail on
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

/// Re-executes one injected journal export and admits the derived receipt.
///
/// # Errors
///
/// Returns a pack refusal when the export is missing a journal law, names a
/// publication or the construction specimen, is a phase digest list, names a
/// subject other than `subject`, or the catalog refuses the constructed receipt.
pub fn observe_journal_completion(
    subject: &str,
    body: &str,
) -> Result<ObservedJournalCompletion, PackFailure> {
    let checked =
        CheckedSubjectId::from_text(subject).map_err(|_| fail("subject-identity", subject))?;
    let rendered = render(checked.content_id());
    let spec = find_pack("checker/c-release")
        .ok_or_else(|| fail("unknown-checker", "checker/c-release"))?;
    let binding = spec
        .checker_binding()
        .map_err(|error| fail("invalid-static-binding", &error.to_string()))?;
    let binding_id = render(binding.id().content_id());
    let evidence = MemorySubject::default().with(journal::FACT, body);
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
            journal::FACT => body = request.evidence.fact(key),
            other => return Err(fail("unexpected-fact", other)),
        }
    }
    let Some(body) = body else {
        return Err(fail("missing-journal", journal::FACT));
    };
    let inspected = journal::inspect(body)?;
    let bound = CheckedSubjectId::from_text(&inspected.subject)
        .map_err(|_| fail("subject-mismatch", journal::FACT))?;
    if render(bound.content_id()) != request.subject {
        return Err(fail("subject-mismatch", journal::FACT));
    }
    super::refuse_claim_values(&inspected.observations)?;
    Ok(inspected.observations)
}

fn admit_parsed(
    subject: &str,
    body: &str,
    observations: &[CheckObservation],
) -> Result<ObservedJournalCompletion, PackFailure> {
    let inspected = journal::inspect(body)?;
    if inspected.subject != subject {
        return Err(fail("subject-mismatch", journal::FACT));
    }
    let mut observed = observations
        .iter()
        .map(|item| (item.key.as_str(), item.value.as_str()))
        .collect::<Vec<_>>();
    observed.sort();
    let mut expected = inspected
        .observations
        .iter()
        .map(|item| (item.key.as_str(), item.value.as_str()))
        .collect::<Vec<_>>();
    expected.sort();
    if observed != expected || observations.len() != inspected.observations.len() {
        return Err(fail("evidence-mismatch", journal::FACT));
    }
    if inspected.left != inspected.right || inspected.left.as_slice() == body.as_bytes() {
        return Err(fail("bad-record", journal::FACT));
    }
    let text = source_text(&inspected.storage);
    let found = discovered(&inspected.storage);
    let selected = [&found];
    let leaf = ProofLeaf::ArtifactEquality {
        name: PROOF.into(),
        left: inspected.left,
        right: inspected.right,
    };
    let scope = CheckScopeId::from_text(SCOPE)
        .map_err(|error| fail("receipt-refused", &error.to_string()))?;
    let subject_id = CheckedSubjectId::from_text(subject)
        .map_err(|error| fail("subject-identity", &error.to_string()))?;
    let policy = PolicyId::from_text("policy/journal-replay")
        .map_err(|error| fail("receipt-refused", &error.to_string()))?;
    let claim = EvidenceClaim {
        proof: PROOF.into(),
        subject: subject_id.clone(),
        scope: scope.clone(),
        checker_receipt: CheckerReceiptId::from_text("receipt/unused")
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        projections: vec!["projection/journal-replay".into()],
        environment: "environment/journal-replay".into(),
        source_text: text,
        members: vec![found.semantic.clone()],
        domains: vec![DOMAIN.into()],
        policies: vec![policy.clone()],
        supports: Vec::new(),
        telemetry: String::new(),
        requested_grade: None,
    };
    let bound = input_closure(&leaf, &claim, &selected)?;
    let binding = receipt_binding(&scope)?;
    let invocation = binding
        .instantiate(
            ProofCodeId::from_text("code/journal-replay")
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
        EvidenceProvenanceId::from_text("provenance/journal-replay")
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        policy,
        EvidenceSetId::from_text("support/journal-replay")
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
    Ok(ObservedJournalCompletion {
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
        OwnerBindingId::from_text("owner/journal-replay")
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        "packs::release::check".into(),
        vec![
            ConformancePackId::from_text("pack/release")
                .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        ],
        OutputShapeId::from_text("shape/check-receipt")
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        RevocationSourceId::from_text("revocation/journal-replay")
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
            WorkingDirectoryPolicyId::from_text("cwd/journal-replay")
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

fn discovered(storage: &str) -> DiscoveredDefinition {
    DiscoveredDefinition {
        class: "package",
        domain: DOMAIN.to_owned(),
        name: "journal-export".to_owned(),
        semantic: semantic("package", &[DOMAIN, "git", "journal-export", "0", "leaf"]),
        storage: storage.to_owned(),
    }
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

fn source_text(storage: &str) -> String {
    format!(
        "sim-world-inputs-v1\nroot-package journal-replay\ndomain completion/replay . compiling replay\nmember completion/replay git journal-export 0 leaf {storage} git leaf\n"
    )
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
