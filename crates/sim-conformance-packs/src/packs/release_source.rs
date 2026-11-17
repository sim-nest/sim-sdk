// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Source-identity observations for one unpublished git manifest.
//!
//! The pack reads the manifest text. It does not read a caller boolean, a
//! source-deck label, or a completion claim. The harness builds the receipt
//! from the identities that parse produced, verifies that construction, and
//! the proof catalog admits the evidence. A missing or refused manifest
//! produces no receipt.

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

const MAX_BYTES: usize = 65_536;
const SCOPE: &str = "release/source-identity";

struct Dep {
    name: String,
    version: String,
    url: String,
    rev: String,
}

/// Receipt the proof catalog admitted for one observed source manifest.
pub struct ObservedSourceRelease {
    /// Subject the checker invocation and the receipt both name.
    pub subject: String,
    /// `completion/claim-21` plus the sorted identities the manifest parsed to.
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

/// Checks one injected manifest and admits the derived source receipt.
///
/// # Errors
///
/// Returns a pack refusal when the manifest is missing a required fact,
/// names a publication or the construction specimen, or the catalog
/// refuses the constructed receipt.
pub fn observe_source_identity(
    subject: &str,
    manifest: &str,
) -> Result<ObservedSourceRelease, PackFailure> {
    let checked =
        CheckedSubjectId::from_text(subject).map_err(|_| fail("subject-identity", subject))?;
    let rendered = render(checked.content_id());
    let spec = find_pack("checker/c-release")
        .ok_or_else(|| fail("unknown-checker", "checker/c-release"))?;
    let binding = spec
        .checker_binding()
        .map_err(|error| fail("invalid-static-binding", &error.to_string()))?;
    let binding_id = render(binding.id().content_id());
    let evidence = MemorySubject::default().with("release.manifest", manifest);
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
        PackVerdict::UnimplementedPack { .. } => {
            return Err(fail("unimplemented", SCOPE));
        }
    };
    admit_parsed(subject, manifest, &observations)
}

pub(super) fn observations(
    request: &PackRequest<'_>,
) -> Result<Vec<CheckObservation>, PackFailure> {
    let mut manifest = None;
    for key in request.evidence.fact_keys() {
        match key {
            "release.manifest" => manifest = request.evidence.fact(key),
            other => return Err(fail("unexpected-fact", other)),
        }
    }
    let Some(manifest) = manifest else {
        return Err(fail("missing-manifest", "release.manifest"));
    };
    let deps = parse_manifest(manifest)?;
    let mut observations = vec![observation("release.unpublished", "publish-false")];
    for dep in &deps {
        observations.push(observation("release.source", identity(dep)));
    }
    Ok(observations)
}

fn admit_parsed(
    subject: &str,
    manifest: &str,
    observations: &[CheckObservation],
) -> Result<ObservedSourceRelease, PackFailure> {
    let deps = parse_manifest(manifest)?;
    let identities = deps.iter().map(identity).collect::<Vec<_>>();
    let observed = observations
        .iter()
        .filter(|item| item.key == "release.source")
        .map(|item| item.value.clone())
        .collect::<Vec<_>>();
    if observed != identities {
        return Err(fail("evidence-mismatch", "release.source"));
    }
    for value in &identities {
        if !exact_identity(value) {
            return Err(fail("evidence-mismatch", "release.source"));
        }
    }
    let mut result = String::from("completion/claim-21");
    if !identities.is_empty() {
        result.push(' ');
        result.push_str(&identities.join(" "));
    }
    let unpublished = observations
        .iter()
        .filter(|item| item.key == "release.unpublished")
        .map(|item| item.value.as_str())
        .collect::<Vec<_>>();
    if unpublished != ["publish-false"] {
        return Err(fail("evidence-mismatch", "release.unpublished"));
    }
    let source = source_text(&deps);
    let found = discovered(&deps);
    if found.is_empty() {
        return Err(fail("missing-source", "release.manifest"));
    }
    let leaf = ProofLeaf::ArtifactEquality {
        name: "source-identity".into(),
        left: manifest.as_bytes().to_vec(),
        right: manifest.as_bytes().to_vec(),
    };
    let scope = CheckScopeId::from_text(SCOPE)
        .map_err(|error| fail("receipt-refused", &error.to_string()))?;
    let subject_id = CheckedSubjectId::from_text(subject)
        .map_err(|error| fail("subject-identity", &error.to_string()))?;
    let policy = PolicyId::from_text("policy/source-identity")
        .map_err(|error| fail("receipt-refused", &error.to_string()))?;
    let mut shape_members = found
        .iter()
        .map(|row| row.semantic.clone())
        .collect::<Vec<_>>();
    shape_members.sort();
    shape_members.dedup();
    let claim = EvidenceClaim {
        proof: "source-identity".into(),
        subject: subject_id.clone(),
        scope: scope.clone(),
        checker_receipt: CheckerReceiptId::from_text("receipt/unused")
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        projections: vec!["projection/manifest".into()],
        environment: "environment/source-identity".into(),
        source_text: source,
        members: shape_members,
        domains: vec!["resolver/source".into()],
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
            ProofCodeId::from_text("code/source-identity")
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
        CheckerResultId::from_text(&result)
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        EvidenceGrade::Release,
        EvidenceProvenanceId::from_text("provenance/source-identity")
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        policy,
        EvidenceSetId::from_text("support/source-identity")
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
    Ok(ObservedSourceRelease {
        subject: subject.to_owned(),
        result,
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
        OwnerBindingId::from_text("owner/source-identity")
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        "packs::release::check".into(),
        vec![
            ConformancePackId::from_text("pack/release")
                .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        ],
        OutputShapeId::from_text("shape/check-receipt")
            .map_err(|error| fail("receipt-refused", &error.to_string()))?,
        RevocationSourceId::from_text("revocation/source-identity")
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
            WorkingDirectoryPolicyId::from_text("cwd/source")
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

fn parse_manifest(text: &str) -> Result<Vec<Dep>, PackFailure> {
    if text.len() > MAX_BYTES {
        return Err(fail("bound", "release.manifest"));
    }
    if text.contains('\r') {
        return Err(fail("bad-record", "release.manifest"));
    }
    if text.contains("shade-1") {
        return Err(fail("construction-specimen", "release.manifest"));
    }
    if text.contains("PortableClosed") || text.contains("publication-confirmed") {
        return Err(fail("local-publication-claim", "release.manifest"));
    }
    let mut deps = Vec::new();
    let mut unpublished = false;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((lhs, rhs)) = line.split_once('=') else {
            continue;
        };
        let key = lhs.trim();
        let rhs = rhs.trim();
        if key == "publish" {
            match rhs {
                "false" => unpublished = true,
                "true" => return Err(fail("local-publication-claim", "publish")),
                _ => return Err(fail("bad-record", "publish")),
            }
            continue;
        }
        if !rhs.starts_with('{') || !rhs.contains("git") {
            continue;
        }
        let name = quoted(rhs, "package").unwrap_or(key);
        let version = quoted(rhs, "version").ok_or_else(|| fail("missing-source", "version"))?;
        let url = quoted(rhs, "git").ok_or_else(|| fail("missing-source", "git"))?;
        let rev = quoted(rhs, "rev").ok_or_else(|| fail("missing-source", "rev"))?;
        if !is_name(name) || !is_version(version) || !is_rev(rev) || !is_url(url) {
            return Err(fail("bad-record", name));
        }
        deps.push(Dep {
            name: name.to_owned(),
            version: version.to_owned(),
            url: url.to_owned(),
            rev: rev.to_owned(),
        });
    }
    if !unpublished {
        return Err(fail("missing-observation", "publish = false"));
    }
    if deps.is_empty() {
        return Err(fail("missing-source", "release.manifest"));
    }
    let mut names = BTreeSet::new();
    for dep in &deps {
        if !names.insert(dep.name.as_str()) {
            return Err(fail("duplicate-crate", &dep.name));
        }
    }
    deps.sort_by_key(identity);
    Ok(deps)
}

fn discovered(deps: &[Dep]) -> Vec<DiscoveredDefinition> {
    deps.iter()
        .map(|dep| DiscoveredDefinition {
            class: "package",
            domain: "resolver/source".to_owned(),
            name: dep.name.clone(),
            semantic: semantic(
                "package",
                &[
                    "resolver/source",
                    "git",
                    dep.name.as_str(),
                    dep.version.as_str(),
                    dep.url.as_str(),
                ],
            ),
            storage: dep.rev.clone(),
        })
        .collect()
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

fn source_text(deps: &[Dep]) -> String {
    let mut text = String::from(
        "sim-world-inputs-v1\nroot-package source-release\ndomain resolver/source . compiling Cargo.lock\n",
    );
    for dep in deps {
        text.push_str("member resolver/source git ");
        text.push_str(&dep.name);
        text.push(' ');
        text.push_str(&dep.version);
        text.push(' ');
        text.push_str(&dep.url);
        text.push(' ');
        text.push_str(&dep.rev);
        text.push_str(" git closure\n");
    }
    text
}

fn identity(dep: &Dep) -> String {
    format!("git+{}:{}@{}", dep.rev, dep.name, dep.version)
}

fn exact_identity(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("git+") else {
        return false;
    };
    let Some((rev, spec)) = rest.split_once(':') else {
        return false;
    };
    let Some((name, version)) = spec.split_once('@') else {
        return false;
    };
    is_rev(rev) && is_name(name) && is_version(version)
}

fn quoted<'a>(body: &'a str, name: &str) -> Option<&'a str> {
    let spaced = format!("{name} = \"");
    let tight = format!("{name}=\"");
    let start = body
        .find(&spaced)
        .map(|at| at + spaced.len())
        .or_else(|| body.find(&tight).map(|at| at + tight.len()))?;
    let rest = body.get(start..)?;
    let end = rest.find('"')?;
    Some(&rest[..end])
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

fn is_rev(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_url(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !value
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
        && (value.starts_with("https://")
            || value.starts_with("ssh://")
            || value.starts_with("git://")
            || value.starts_with("git+"))
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
    let digest = id
        .bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{}:{digest}", id.algorithm.as_qualified_str())
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
#[path = "release_source_tests.rs"]
mod tests;
