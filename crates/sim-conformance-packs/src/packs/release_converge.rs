// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Claim 12 for a sealed fixed point.
//!
//! The check calls `seal_fixed_point`. The result is `completion/claim-12`
//! only when the first pass seals one owner and an exact postimage and the
//! second pass does no work. A wrong or empty postimage, a duplicated
//! generator, and a split owner refuse for the seal's own reasons. A changed
//! closure that generates again is not that second pass. A caller label is
//! not this result.

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};
use sim_artifact_facet::{
    ArtifactFacet, ArtifactId, DisclosureDecisionId, MergePolicy, OwnerId, ProjectionId,
    RegionOwnership, RegionSelector,
};
use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};
use sim_lib_roadmap_runner::{
    AdmittedEvidence, FactGenerator, FixedStep, OpenClaim, ProofCatalog, ProofOutcome, Revision,
    RevisionOpen, SOURCE_SEAL, open_revision, seal_fixed_point, seal_id,
};

use super::receipt::{self, AdmitExecuted, PackageRow};
use crate::executor;
use crate::{CheckObservation, MemorySubject, PackFailure, PackRequest, PackVerdict, find_pack};

const SCOPE: &str = "completion/sealed-convergence";
const RESULT: &str = "completion/claim-12";
const FACT: &str = "completion.sealed-convergence";
const PROOF: &str = "sealed-convergence";
const DOMAIN: &str = "completion/sealed-convergence";
const EXPECTED: &str = concat!(
    "sealed one-owner exact-postimage zero-work gap ",
    "duplicate-generator split-generator"
);

/// Receipt admitted after the fixed-point law holds.
#[derive(Debug)]
pub struct ObservedSealedConvergence {
    /// Subject the checker invocation and the receipt both name.
    pub subject: String,
    /// `completion/claim-12` for this subject.
    pub result: String,
    /// Tokens the fixed-point law recorded.
    pub execution: String,
    /// Catalog that admitted the evidence.
    pub catalog: ProofCatalog,
    /// Evidence the catalog admitted.
    pub admitted: AdmittedEvidence,
    /// Checker receipt that bound the admission.
    pub checker: sim_conformance_v02::CheckerReceiptId,
    /// Scope `completion/sealed-convergence`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Runs the fixed-point law and admits claim 12.
///
/// # Errors
///
/// Returns a refusal when the subject is not a token, the fact carries a
/// completion label, or the seal leaves an owner, postimage, or second-pass gap.
pub fn observe_sealed_convergence(subject: &str) -> Result<ObservedSealedConvergence, PackFailure> {
    let checked = CheckedSubjectId::from_text(subject)
        .map_err(|error| fail("subject-identity", &error.to_string()))?;
    let spec = find_pack("checker/c-release")
        .ok_or_else(|| fail("unknown-checker", "checker/c-release"))?;
    let binding = spec
        .checker_binding()
        .map_err(|error| fail("invalid-static-binding", &error.to_string()))?;
    let evidence = MemorySubject::default().with(FACT, encode_request(subject)?);
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
    admit(subject, &observations)
}

/// Re-checks a convergence receipt against the catalog that admitted it.
///
/// # Errors
///
/// Returns a refusal when the result or scope is no longer claim 12.
pub fn bind_sealed_convergence(observation: &ObservedSealedConvergence) -> Result<(), PackFailure> {
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
    if body.contains("completion/claim-") {
        return Err(fail("claim-relabelled", FACT));
    }
    let subject = decode_request(body)?;
    let bound =
        CheckedSubjectId::from_text(&subject).map_err(|_| fail("subject-mismatch", FACT))?;
    if receipt::render(bound.content_id()) != request.subject {
        return Err(fail("subject-mismatch", FACT));
    }
    let executed = execute()?;
    Ok(vec![observation("completion.execution", executed)])
}

fn admit(
    subject: &str,
    observations: &[CheckObservation],
) -> Result<ObservedSealedConvergence, PackFailure> {
    let executed = execution(observations)?;
    let again = execute()?;
    if executed != again {
        return Err(fail("evidence-mismatch", FACT));
    }
    let rows = vec![PackageRow {
        name: PROOF.to_owned(),
        version: "0".to_owned(),
        url: format!("git://{PROOF}"),
        rev: hex_bytes(Sha256::digest(executed.as_bytes()).as_slice()),
    }];
    let admitted = receipt::admit_executed(AdmitExecuted {
        subject,
        result: RESULT,
        scope: SCOPE,
        proof: PROOF,
        domain: DOMAIN,
        policy: "policy/sealed-convergence",
        provenance: "provenance/sealed-convergence",
        support: "support/sealed-convergence",
        projection: "projection/sealed-convergence",
        environment: "environment/sealed-convergence",
        leaf: executed.as_bytes(),
        source_text: &receipt::package_source(DOMAIN, PROOF, &rows),
        definitions: &receipt::package_definitions(DOMAIN, &rows),
    })?;
    Ok(ObservedSealedConvergence {
        subject: subject.to_owned(),
        result: RESULT.to_owned(),
        execution: executed.to_owned(),
        catalog: admitted.catalog,
        admitted: admitted.admitted,
        checker: admitted.checker,
        scope: admitted.scope,
        revocation: admitted.revocation,
    })
}

fn execute() -> Result<String, PackFailure> {
    let mut tokens = Vec::new();
    prove_seal(&mut tokens)?;
    prove_gap(&mut tokens)?;
    prove_duplicate(&mut tokens)?;
    prove_split(&mut tokens)?;
    prove_second_pass()?;
    let executed = tokens.join(" ");
    if executed.contains("completion/claim-") {
        return Err(fail("claim-relabelled", SCOPE));
    }
    if executed != EXPECTED {
        return Err(fail("convergence-gap", SCOPE));
    }
    Ok(executed)
}

fn prove_seal(tokens: &mut Vec<&str>) -> Result<(), PackFailure> {
    let revision = standard_revision()?;
    let owner = owner("owner/alpha");
    let facets = success_facets(&owner);
    let generator = CopyGen::new(owner.clone(), executor::cid(12), Written::Copy);
    let owned = one_owner(&facets, &generator)?;
    let source = source_of(&revision, &generator)?;
    let produced = generator.postimage(&source);
    if produced.keys().cloned().collect::<BTreeSet<_>>() != owned {
        return Err(fail("convergence-gap", "exact-postimage"));
    }
    for (path, bytes) in &produced {
        if revision.images().get(path) != Some(bytes) {
            return Err(fail("convergence-gap", "exact-postimage"));
        }
    }
    let generators: [&dyn FactGenerator; 1] = [&generator];
    let first = judge(&revision, &facets, &generators, None)?;
    let FixedStep::Sealed { seal, work } = first else {
        return Err(fail("convergence-gap", "sealed"));
    };
    if work == 0 || generator.calls.get() != work {
        return Err(fail("convergence-gap", "sealed"));
    }
    let generators: [&dyn FactGenerator; 1] = [&generator];
    let second = judge(&revision, &facets, &generators, Some(&seal))?;
    let FixedStep::ZeroWork { seal: again } = second else {
        return Err(fail("convergence-gap", "zero-work"));
    };
    if again.id != seal.id || generator.calls.get() != work {
        return Err(fail("convergence-gap", "zero-work"));
    }
    tokens.extend(["sealed", "one-owner", "exact-postimage", "zero-work"]);
    Ok(())
}

fn prove_gap(tokens: &mut Vec<&str>) -> Result<(), PackFailure> {
    refused_postimage(b"other")?;
    refused_postimage(b"")?;
    tokens.push("gap");
    Ok(())
}

fn refused_postimage(bytes: &'static [u8]) -> Result<(), PackFailure> {
    let revision = standard_revision()?;
    let owner = owner("owner/alpha");
    let facets = success_facets(&owner);
    let generator = CopyGen::new(owner, executor::cid(17), Written::Fixed(bytes));
    let before = revision.clone();
    let generators: [&dyn FactGenerator; 1] = [&generator];
    let step = judge(&revision, &facets, &generators, None)?;
    expect_blocked(step, "gap", &["fact/alpha"])?;
    if generator.calls.get() != 1 || revision != before {
        return Err(fail("convergence-gap", "gap"));
    }
    Ok(())
}

fn prove_duplicate(tokens: &mut Vec<&str>) -> Result<(), PackFailure> {
    let revision = standard_revision()?;
    let owner = owner("owner/alpha");
    let facets = success_facets(&owner);
    let first = CopyGen::new(owner.clone(), executor::cid(15), Written::Copy);
    let second = CopyGen::new(owner, executor::cid(16), Written::Copy);
    let before = revision.clone();
    let generators: [&dyn FactGenerator; 2] = [&first, &second];
    let step = judge(&revision, &facets, &generators, None)?;
    expect_blocked(step, "duplicate-generator", &["fact/alpha"])?;
    if first.calls.get() != 0 || second.calls.get() != 0 || revision != before {
        return Err(fail("convergence-gap", "duplicate-generator"));
    }
    tokens.push("duplicate-generator");
    Ok(())
}

fn prove_split(tokens: &mut Vec<&str>) -> Result<(), PackFailure> {
    let revision = standard_revision()?;
    let alpha = owner("owner/alpha");
    let beta = owner("owner/beta");
    let facets = vec![
        facet("artifact/alpha", &alpha, "fact/alpha", true),
        facet("artifact/alpha", &beta, "fact/extra", true),
    ];
    let before = revision.clone();
    let generators: [&dyn FactGenerator; 0] = [];
    let step = judge(&revision, &facets, &generators, None)?;
    expect_blocked(step, "split-generator", &["fact/alpha", "fact/extra"])?;
    if revision != before {
        return Err(fail("convergence-gap", "split-generator"));
    }
    tokens.push("split-generator");
    Ok(())
}

fn prove_second_pass() -> Result<(), PackFailure> {
    let revision = standard_revision()?;
    let owner = owner("owner/alpha");
    let facets = success_facets(&owner);
    let generator = CopyGen::new(owner.clone(), executor::cid(12), Written::Copy);
    let generators: [&dyn FactGenerator; 1] = [&generator];
    let first = judge(&revision, &facets, &generators, None)?;
    let FixedStep::Sealed { seal: prior, work } = first else {
        return Err(fail("convergence-gap", "second-pass"));
    };
    if work == 0 || generator.calls.get() != work {
        return Err(fail("convergence-gap", "second-pass"));
    }
    let changed = CopyGen::new(owner.clone(), executor::cid(13), Written::Copy);
    let generators: [&dyn FactGenerator; 1] = [&changed];
    let resealed = judge(&revision, &facets, &generators, Some(&prior))?;
    let FixedStep::Sealed {
        seal: fresh,
        work: again,
    } = resealed
    else {
        return Err(fail("convergence-gap", "second-pass"));
    };
    if again == 0
        || fresh.id == prior.id
        || changed.calls.get() != 1
        || generator.calls.get() != work
    {
        return Err(fail("convergence-gap", "second-pass"));
    }
    let wrong = CopyGen::new(owner, executor::cid(14), Written::Fixed(b"other"));
    let before = revision.clone();
    let generators: [&dyn FactGenerator; 1] = [&wrong];
    let step = judge(&revision, &facets, &generators, Some(&prior))?;
    expect_blocked(step, "gap", &["fact/alpha"])?;
    if wrong.calls.get() != 1 || generator.calls.get() != work || revision != before {
        return Err(fail("convergence-gap", "second-pass"));
    }
    Ok(())
}

fn judge(
    revision: &Revision,
    facets: &[ArtifactFacet],
    generators: &[&dyn FactGenerator],
    prior: Option<&sim_lib_roadmap_runner::FixedSeal>,
) -> Result<FixedStep, PackFailure> {
    seal_fixed_point(
        revision,
        facets,
        generators,
        &proofs(),
        &BTreeSet::new(),
        prior,
    )
    .map_err(|error| fail("convergence-gap", &error.to_string()))
}

fn expect_blocked(step: FixedStep, reason: &str, paths: &[&str]) -> Result<(), PackFailure> {
    let expected: BTreeSet<String> = paths.iter().map(|path| (*path).to_owned()).collect();
    match step {
        FixedStep::Blocked {
            reason: got,
            closure,
            repeated: None,
        } if got == reason && closure == expected => Ok(()),
        FixedStep::Blocked {
            reason: got,
            closure,
            ..
        } => Err(fail(
            "convergence-gap",
            &format!(
                "{got}:{}",
                closure.into_iter().collect::<Vec<_>>().join(",")
            ),
        )),
        _ => Err(fail("convergence-gap", reason)),
    }
}

fn one_owner(
    facets: &[ArtifactFacet],
    generator: &CopyGen,
) -> Result<BTreeSet<String>, PackFailure> {
    let mut owners = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for facet in facets {
        let RegionOwnership::Generated {
            generator: declared,
        } = facet.ownership()
        else {
            continue;
        };
        let RegionSelector::ExactPath(path) = facet.region() else {
            return Err(fail("convergence-gap", "one-owner"));
        };
        if facet.owner() != declared || facet.owner() != &generator.owner {
            return Err(fail("convergence-gap", "one-owner"));
        }
        owners.insert(facet.owner().clone());
        if !paths.insert(path.clone()) {
            return Err(fail("convergence-gap", "one-owner"));
        }
    }
    if owners.len() != 1 || paths.is_empty() {
        return Err(fail("convergence-gap", "one-owner"));
    }
    Ok(paths)
}

fn standard_revision() -> Result<Revision, PackFailure> {
    let seal = b"seal-v1".to_vec();
    let mut images = BTreeMap::new();
    images.insert(SOURCE_SEAL.to_owned(), seal.clone());
    images.insert("fact/src".into(), b"alpha".to_vec());
    images.insert("fact/alpha".into(), b"alpha".to_vec());
    images.insert("fact/extra".into(), b"extra".to_vec());
    let seal_id = seal_id(&seal).map_err(|error| fail("convergence-gap", &error.to_string()))?;
    open_revision(RevisionOpen {
        seal_id,
        seal,
        images,
        claims: vec![
            OpenClaim {
                id: "claim/src".into(),
                inputs: BTreeSet::from(["fact/src".into()]),
            },
            OpenClaim {
                id: "claim/alpha".into(),
                inputs: BTreeSet::from(["fact/alpha".into()]),
            },
        ],
    })
    .map_err(|error| fail("convergence-gap", &error.to_string()))
}

fn success_facets(generated: &OwnerId) -> Vec<ArtifactFacet> {
    let authored = owner("owner/src");
    vec![
        facet("artifact/src", &authored, "fact/src", false),
        facet("artifact/alpha", generated, "fact/alpha", true),
    ]
}

fn proofs() -> BTreeMap<String, ProofOutcome> {
    BTreeMap::from([
        ("claim/src".into(), ProofOutcome::Passed),
        ("claim/alpha".into(), ProofOutcome::Passed),
    ])
}

fn source_of(
    revision: &Revision,
    generator: &CopyGen,
) -> Result<BTreeMap<String, Vec<u8>>, PackFailure> {
    let mut source = BTreeMap::new();
    for path in &generator.reads {
        let Some(bytes) = revision.images().get(path) else {
            return Err(fail("convergence-gap", "unknown-fact"));
        };
        source.insert(path.clone(), bytes.clone());
    }
    Ok(source)
}

fn facet(artifact: &str, owner: &OwnerId, path: &str, generated: bool) -> ArtifactFacet {
    let ownership = if generated {
        RegionOwnership::Generated {
            generator: owner.clone(),
        }
    } else {
        RegionOwnership::Authored
    };
    ArtifactFacet::new(
        ArtifactId::from_text(artifact).expect("artifact id"),
        owner.clone(),
        RegionSelector::ExactPath(path.to_owned()),
        ProjectionId::from_text("projection/file").expect("projection id"),
        MergePolicy::Exact,
        DisclosureDecisionId::from_text("disclosure/model").expect("disclosure id"),
        ownership,
    )
    .expect("facet")
}

fn owner(name: &str) -> OwnerId {
    OwnerId::from_text(name).expect("owner")
}

enum Written {
    Copy,
    Fixed(&'static [u8]),
}

struct CopyGen {
    owner: OwnerId,
    reads: BTreeSet<String>,
    writes: BTreeSet<String>,
    implementation: sim_kernel::ContentId,
    calls: Cell<u32>,
    written: Written,
}

impl CopyGen {
    fn new(owner: OwnerId, implementation: sim_kernel::ContentId, written: Written) -> Self {
        Self {
            owner,
            reads: BTreeSet::from([SOURCE_SEAL.to_owned(), "fact/src".into()]),
            writes: BTreeSet::from(["fact/alpha".into()]),
            implementation,
            calls: Cell::new(0),
            written,
        }
    }

    fn postimage(&self, source: &BTreeMap<String, Vec<u8>>) -> BTreeMap<String, Vec<u8>> {
        let bytes = match self.written {
            Written::Copy => source.get("fact/src").cloned().unwrap_or_else(Vec::new),
            Written::Fixed(bytes) => bytes.to_vec(),
        };
        BTreeMap::from([("fact/alpha".into(), bytes)])
    }
}

impl FactGenerator for CopyGen {
    fn owner(&self) -> &OwnerId {
        &self.owner
    }
    fn reads(&self) -> &BTreeSet<String> {
        &self.reads
    }
    fn writes(&self) -> &BTreeSet<String> {
        &self.writes
    }
    fn implementation(&self) -> sim_kernel::ContentId {
        self.implementation.clone()
    }
    fn generate(&self, source: &BTreeMap<String, Vec<u8>>) -> BTreeMap<String, Vec<u8>> {
        self.calls.set(self.calls.get().saturating_add(1));
        self.postimage(source)
    }
}

fn encode_request(subject: &str) -> Result<String, PackFailure> {
    if !token(subject) {
        return Err(fail("bad-record", FACT));
    }
    let mut body = String::new();
    receipt::push_field(&mut body, subject);
    Ok(body)
}

fn decode_request(body: &str) -> Result<String, PackFailure> {
    let (subject, rest) = receipt::take_field(body)?;
    if !rest.is_empty() || !token(subject) {
        return Err(fail("bad-record", FACT));
    }
    Ok(subject.to_owned())
}

fn one_fact<'a>(request: &'a PackRequest<'a>) -> Result<&'a str, PackFailure> {
    let mut body = None;
    for key in request.evidence.fact_keys() {
        match key {
            FACT => body = request.evidence.fact(key),
            other => return Err(fail("unexpected-fact", other)),
        }
    }
    body.ok_or_else(|| fail("missing-convergence", FACT))
}

fn execution(observations: &[CheckObservation]) -> Result<&str, PackFailure> {
    observations
        .iter()
        .find(|item| item.key == "completion.execution")
        .map(|item| item.value.as_str())
        .ok_or_else(|| fail("missing-convergence", FACT))
}

fn token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !value.contains("completion/claim-")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && byte != b' ')
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
#[path = "release_converge_tests.rs"]
mod tests;
