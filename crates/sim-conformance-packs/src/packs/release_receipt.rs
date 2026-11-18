// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Receipt admission for one checker execution.
//!
//! The caller supplies the leaf bytes and the package rows its own process
//! produced. This module does not spawn a process and does not accept a
//! completion claim as text.

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

use crate::{PackFailure, find_pack};

const RELEASE_SYMBOL: &str = "sim_conformance_packs::packs::release::check";
const ACTIVATION_OWNER: &str =
    "core/sha256-datum-v1:6fc0e9594fe3a592c1877d9034cc7a6b5f9aa9ce50e3251a705955b348ccf827";
const REVOCATION_SOURCE: &str = "sim-sdk/sim-conformance-packs-revocations-v1";
const OWNER_VALIDATION: &str = "cargo fmt --all --check && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo doc --workspace --no-deps && cargo clippy --workspace --all-features --all-targets -- -D warnings && cargo test --workspace --all-features";
const OWNER_DOCS: &str = "cargo run -p xtask -- simdoc --check";

pub(super) struct PackageRow {
    pub name: String,
    pub version: String,
    pub url: String,
    pub rev: String,
}

pub(super) struct ExecutedReceipt {
    pub catalog: ProofCatalog,
    pub admitted: AdmittedEvidence,
    pub checker: CheckerReceiptId,
    pub scope: CheckScopeId,
    pub revocation: RevocationStatus,
}

pub(super) struct AdmitExecuted<'a> {
    pub subject: &'a str,
    pub result: &'a str,
    pub scope: &'a str,
    pub proof: &'a str,
    pub domain: &'a str,
    pub policy: &'a str,
    pub provenance: &'a str,
    pub support: &'a str,
    pub projection: &'a str,
    pub environment: &'a str,
    pub leaf: &'a [u8],
    pub source_text: &'a str,
    pub definitions: &'a [DiscoveredDefinition],
}

pub(super) fn admit_executed(request: AdmitExecuted<'_>) -> Result<ExecutedReceipt, PackFailure> {
    if request.definitions.is_empty() {
        return Err(fail("missing-source", request.proof));
    }
    let leaf = ProofLeaf::ArtifactEquality {
        name: request.proof.to_owned(),
        left: request.leaf.to_vec(),
        right: request.leaf.to_vec(),
    };
    let scope = CheckScopeId::from_text(request.scope)
        .map_err(|error| fail("receipt-refused", &error.to_string()))?;
    let subject_id = CheckedSubjectId::from_text(request.subject)
        .map_err(|error| fail("subject-identity", &error.to_string()))?;
    let policy = PolicyId::from_text(request.policy)
        .map_err(|error| fail("receipt-refused", &error.to_string()))?;
    let mut members = request
        .definitions
        .iter()
        .map(|row| row.semantic.clone())
        .collect::<Vec<_>>();
    members.sort();
    members.dedup();
    let claim = EvidenceClaim {
        proof: request.proof.to_owned(),
        subject: subject_id.clone(),
        scope: scope.clone(),
        checker_receipt: CheckerReceiptId::from_text("receipt/unused")
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        projections: vec![request.projection.to_owned()],
        environment: request.environment.to_owned(),
        source_text: request.source_text.to_owned(),
        members,
        domains: vec![request.domain.to_owned()],
        policies: vec![policy.clone()],
        supports: Vec::new(),
        telemetry: String::new(),
        requested_grade: None,
    };
    let selected = request.definitions.iter().collect::<Vec<_>>();
    let bound = input_closure(&leaf, &claim, &selected)?;
    let spec = find_pack("checker/c-release")
        .ok_or_else(|| fail("unknown-checker", "checker/c-release"))?;
    let code = ProofCodeId::from_text(&format!(
        "sim-conformance-packs@{};build-inputs-sha256={}",
        env!("CARGO_PKG_VERSION"),
        spec.build_inputs_sha256(),
    ))
    .map_err(|error| fail("receipt-refused", &error.to_string()))?;
    let pack = ConformancePackId::from_text(&format!("{}@{}", spec.pack, spec.activation_binding))
        .map_err(|error| fail("receipt-refused", &error.to_string()))?;
    let binding = receipt_binding(&scope, pack.clone())?;
    let invocation = binding
        .instantiate(code, pack, subject_id, scope.clone(), bound)
        .map_err(|error| fail("receipt-refused", &error.to_string()))?;
    let receipt = CheckerReceipt::passing(
        &invocation,
        CheckerResultId::from_text(request.result)
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        EvidenceGrade::Release,
        EvidenceProvenanceId::from_text(request.provenance)
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        policy,
        EvidenceSetId::from_text(request.support)
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
        .admit_evidence(claim_with(&claim, checker.clone()), issued(&checker))
        .map_err(|error| fail("receipt-refused", &error.to_string()))?;
    match catalog.decide_evidence(&admitted.key, EvidenceGrade::Release, issued(&checker)) {
        EvidenceDecision::Reuse { receipt } if receipt == admitted.id => {}
        EvidenceDecision::Reuse { .. }
        | EvidenceDecision::Recheck { .. }
        | EvidenceDecision::Refuse { .. } => {
            return Err(fail("receipt-refused", "release-reuse"));
        }
    }
    Ok(ExecutedReceipt {
        catalog,
        admitted,
        checker,
        scope,
        revocation: RevocationStatus::Current,
    })
}

pub(super) fn bind_executed(
    subject: &str,
    result: &str,
    scope: &CheckScopeId,
    checker: &CheckerReceiptId,
    catalog: &ProofCatalog,
    admitted: &AdmittedEvidence,
    revocation: RevocationStatus,
) -> Result<(), PackFailure> {
    if revocation != RevocationStatus::Current {
        return Err(fail("revocation", "current"));
    }
    let receipt = catalog
        .checker_receipt(checker, revocation.clone())
        .map_err(|error| fail("evidence-not-verified", &error.to_string()))?;
    let subject_id = CheckedSubjectId::from_text(subject)
        .map_err(|error| fail("subject-identity", &error.to_string()))?;
    if receipt.subject() != &subject_id {
        return Err(fail("subject-mismatch", subject));
    }
    let result_id = CheckerResultId::from_text(result)
        .map_err(|error| fail("receipt-refused", &error.to_string()))?;
    if receipt.result() != &result_id {
        return Err(fail("claim-relabelled", result));
    }
    if receipt.scope() != scope {
        return Err(fail("scope-mismatch", "receipt"));
    }
    match catalog.decide_evidence(&admitted.key, EvidenceGrade::Release, |asked| {
        if asked == checker {
            RevocationStatus::Current
        } else {
            RevocationStatus::Unknown
        }
    }) {
        EvidenceDecision::Reuse { receipt } if receipt == admitted.id => Ok(()),
        _ => Err(fail("evidence-not-verified", "release-reuse")),
    }
}

pub(super) fn package_source(domain: &str, root: &str, rows: &[PackageRow]) -> String {
    let mut text =
        format!("sim-world-inputs-v1\nroot-package {root}\ndomain {domain} . compiling source\n");
    for row in rows {
        text.push_str("member ");
        text.push_str(domain);
        text.push_str(" git ");
        text.push_str(&row.name);
        text.push(' ');
        text.push_str(&row.version);
        text.push(' ');
        text.push_str(&row.url);
        text.push(' ');
        text.push_str(&row.rev);
        text.push_str(" git closure\n");
    }
    text
}

pub(super) fn package_definitions(domain: &str, rows: &[PackageRow]) -> Vec<DiscoveredDefinition> {
    rows.iter()
        .map(|row| DiscoveredDefinition {
            class: "package",
            domain: domain.to_owned(),
            name: row.name.clone(),
            semantic: semantic(
                "package",
                &[
                    domain,
                    "git",
                    row.name.as_str(),
                    row.version.as_str(),
                    row.url.as_str(),
                ],
            ),
            storage: row.rev.clone(),
        })
        .collect()
}

pub(super) fn push_field(out: &mut String, field: &str) {
    out.push_str(&field.len().to_string());
    out.push(':');
    out.push_str(field);
    out.push(';');
}

pub(super) fn take_field(input: &str) -> Result<(&str, &str), PackFailure> {
    let (len_text, rest) = input
        .split_once(':')
        .ok_or_else(|| fail("bad-record", "field"))?;
    if len_text.is_empty()
        || len_text.len() > 8
        || !len_text.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(fail("bad-record", "field"));
    }
    let len: usize = len_text.parse().map_err(|_| fail("bad-record", "field"))?;
    if !rest.is_char_boundary(len) || rest.len() < len + 1 || !rest.is_char_boundary(len + 1) {
        return Err(fail("bad-record", "field"));
    }
    if rest.as_bytes()[len] != b';' {
        return Err(fail("bad-record", "field"));
    }
    Ok((&rest[..len], &rest[len + 1..]))
}

pub(super) fn render(id: &ContentId) -> String {
    let digest = id
        .bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{}:{digest}", id.algorithm.as_qualified_str())
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

fn issued(checker: &CheckerReceiptId) -> impl Fn(&CheckerReceiptId) -> RevocationStatus + '_ {
    move |asked| {
        if asked == checker {
            RevocationStatus::Current
        } else {
            RevocationStatus::Unknown
        }
    }
}

fn receipt_binding(
    scope: &CheckScopeId,
    pack: ConformancePackId,
) -> Result<CheckerBinding, PackFailure> {
    CheckerBinding::new(
        "checker/c-release".into(),
        OwnerBindingId::from_text(ACTIVATION_OWNER)
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        RELEASE_SYMBOL.into(),
        vec![pack],
        OutputShapeId::from_text("check/receipt-v1")
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        RevocationSourceId::from_text(REVOCATION_SOURCE)
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        CommandId::from_text(OWNER_VALIDATION)
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        CommandId::from_text(OWNER_DOCS)
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        BTreeSet::from([scope.clone()]),
        CheckTemplate::new(
            RELEASE_SYMBOL.into(),
            vec![
                CheckArgument::BindingSlot,
                CheckArgument::SubjectSlot,
                CheckArgument::ScopeSlot,
            ],
            WorkingDirectoryPolicyId::from_text("call/no-working-directory")
                .map_err(|error| fail("receipt-refused", &error.to_string()))?,
            EnvironmentPolicyId::from_text("call/native-rust-v1")
                .map_err(|error| fail("receipt-refused", &error.to_string()))?,
            OutputShapeId::from_text("check/result-v1")
                .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        )
        .map_err(|error| fail("receipt-refused", &error.to_string()))?,
    )
    .map_err(|error| fail("receipt-refused", &error.to_string()))
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
        push_field(&mut out, value);
    }
    out
}

fn canon(fields: &[(&str, &str)]) -> String {
    let mut out = String::new();
    for (name, value) in fields {
        out.push_str(name);
        out.push('=');
        push_field(&mut out, value);
    }
    out
}

fn semantic(class: &str, fields: &[&str]) -> String {
    let mut out = String::new();
    push_field(&mut out, class);
    for field in fields {
        push_field(&mut out, field);
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

fn fail(code: &'static str, detail: &str) -> PackFailure {
    PackFailure {
        code,
        detail: detail.chars().take(256).collect(),
    }
}
