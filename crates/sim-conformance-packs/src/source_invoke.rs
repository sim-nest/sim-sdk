// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Source admission through [`sim_lib_roadmap_runner::SourceDeckProvider`].
//!
//! The launcher, repository, and decoders are injected fakes. Path, head, and
//! size refusals are the provider's own results.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};

use sim_lib_exec::{
    ProcessCancellation, SandboxAttempt, SandboxEvidence, SandboxLauncher, SandboxReport,
    SandboxRequest, SandboxResult,
};
use sim_lib_roadmap_runner::{
    ArtifactCommand, ArtifactExcerpt, ArtifactProjection, ArtifactSpecimen, RepoContractDecoder,
    RepositoryIdentity, SourceDeckBounds, SourceDeckProvider, SourceDeckProviderError,
    SourceDeckReceipt, SourceDeckRequest, SourceRepository, TouchedPaths,
};
use sim_source_deck::{
    ByteContentId, ClaimCertificate, DeckLimits, FragmentDecoder, FragmentPin, IndexAnchor,
    IndexFragment, IndexSpecimen, SourceQuery,
};

use crate::{CheckObservation, PackFailure};

const GUEST_ROOT: &str = "/source";
const HEAD: &str = "head-1";
const LIB: &str = "src/lib.rs";
const SIGNATURE: &str = "tests/signature.txt";

/// Paths read by one successful provide, and the three refusals.
///
/// `source.subject` is an owner the provider wrote onto the returned deck.
pub(crate) fn admission(subject: &str) -> Result<Vec<CheckObservation>, PackFailure> {
    let escape = admitted_before_escape(subject)?;
    let head = head_mismatch(subject)?;
    let oversize = oversize(subject)?;
    let repo = files(subject);
    let receipt = provide_happy(subject, &repo)?;
    let owner = bound_owner(&receipt, subject)?;
    Ok(vec![
        obs("source.escape", escape.to_string()),
        obs("source.head", head),
        obs("source.oversize", oversize),
        obs("source.subject", owner),
    ])
}

/// Repository reads against the declared path count.
///
/// `source.subject` is an owner the provider wrote onto the returned deck.
pub(crate) fn performance(subject: &str) -> Result<Vec<CheckObservation>, PackFailure> {
    let (reads, declared, owner) = read_count(subject)?;
    if reads != declared {
        return Err(failure(&format!("reads {reads} declared {declared}")));
    }
    Ok(vec![
        obs("source.declared", declared.to_string()),
        obs("source.reads", reads.to_string()),
        obs("source.subject", owner),
    ])
}

/// Admission, the read boundary, and reuse after a touched path.
pub(crate) fn composed(subject: &str) -> Result<Vec<CheckObservation>, PackFailure> {
    let mut observations = admission(subject)?;
    observations.extend(performance(subject)?);
    observations.extend(reuse(subject)?);
    Ok(observations)
}

/// Reads completed before the provider refuses an escaping path.
pub(crate) fn admitted_before_escape(subject: &str) -> Result<usize, PackFailure> {
    let repo = files(subject);
    let result = SourceDeckProvider::new(
        &Launcher {
            stdout: b"artifact".to_vec(),
        },
        &repo,
        &Artifact {
            owner: subject.to_owned(),
            paths: vec![LIB.into(), "../secret".into()],
        },
        &Fragment {
            owner: subject.to_owned(),
        },
    )
    .provide(
        &request(subject, HEAD, 4096),
        &ProcessCancellation::default(),
    );
    match result {
        Err(SourceDeckProviderError::Path(_)) => Ok(repo.reads.load(Ordering::SeqCst)),
        Err(error) => Err(failure(&error.to_string())),
        Ok(_) => Err(failure("escaping path was admitted")),
    }
}

fn head_mismatch(subject: &str) -> Result<&'static str, PackFailure> {
    let repo = files(subject);
    match SourceDeckProvider::new(
        &Launcher {
            stdout: b"artifact".to_vec(),
        },
        &repo,
        &Artifact::happy(subject),
        &Fragment {
            owner: subject.to_owned(),
        },
    )
    .provide(
        &request(subject, "head-2", 4096),
        &ProcessCancellation::default(),
    ) {
        Err(SourceDeckProviderError::HeadMismatch) => Ok("head-mismatch"),
        Err(error) => Err(failure(&error.to_string())),
        Ok(_) => Err(failure("head mismatch was admitted")),
    }
}

fn oversize(subject: &str) -> Result<&'static str, PackFailure> {
    let repo = files(subject);
    match SourceDeckProvider::new(
        &Launcher {
            stdout: b"artifact".to_vec(),
        },
        &repo,
        &Artifact::happy(subject),
        &Fragment {
            owner: subject.to_owned(),
        },
    )
    .provide(&request(subject, HEAD, 4), &ProcessCancellation::default())
    {
        Err(SourceDeckProviderError::Oversize) => Ok("oversize"),
        Err(error) => Err(failure(&error.to_string())),
        Ok(_) => Err(failure("oversize output was admitted")),
    }
}

fn read_count(subject: &str) -> Result<(usize, usize, String), PackFailure> {
    let repo = files(subject);
    let receipt = provide_happy(subject, &repo)?;
    let owner = bound_owner(&receipt, subject)?;
    let declared = receipt
        .dependencies
        .iter()
        .filter(|path| path.as_str() == LIB || path.as_str() == SIGNATURE)
        .count();
    Ok((repo.reads.load(Ordering::SeqCst), declared, owner))
}

fn bound_owner(receipt: &SourceDeckReceipt, subject: &str) -> Result<String, PackFailure> {
    let mut owners = BTreeSet::new();
    for repository in receipt.deck.repositories() {
        owners.insert(repository.owner.clone());
    }
    for file in receipt.deck.files() {
        owners.insert(file.owner.clone());
    }
    for fragment in receipt.deck.fragments() {
        owners.insert(fragment.owner.clone());
    }
    if owners.len() != 1 {
        return Err(failure("source subject is not one returned owner"));
    }
    let owner = owners
        .into_iter()
        .next()
        .ok_or_else(|| failure("source subject is not one returned owner"))?;
    if owner != subject {
        return Err(failure("returned owner differs from the requested subject"));
    }
    Ok(owner)
}

fn reuse(subject: &str) -> Result<Vec<CheckObservation>, PackFailure> {
    let repo = files(subject);
    let receipt = provide_happy(subject, &repo)?;
    let touched = !receipt.reusable_after(&TouchedPaths(BTreeSet::from([LIB.into()])));
    let unrelated = receipt.reusable_after(&TouchedPaths(BTreeSet::from(["README.md".into()])));
    if !touched || !unrelated {
        return Err(failure("reuse did not follow the dependency set"));
    }
    Ok(vec![
        obs("source.reusable-touched", "false"),
        obs("source.reusable-unrelated", "true"),
    ])
}

fn provide_happy(subject: &str, repo: &Repo) -> Result<SourceDeckReceipt, PackFailure> {
    SourceDeckProvider::new(
        &Launcher {
            stdout: b"artifact".to_vec(),
        },
        repo,
        &Artifact::happy(subject),
        &Fragment {
            owner: subject.to_owned(),
        },
    )
    .provide(
        &request(subject, HEAD, 4096),
        &ProcessCancellation::default(),
    )
    .map_err(|error| failure(&error.to_string()))
}

fn files(subject: &str) -> Repo {
    let mut bytes = b"fn private(a: u32) -> bool { true }".to_vec();
    bytes.extend_from_slice(subject.as_bytes());
    Repo {
        files: BTreeMap::from([
            (LIB.into(), bytes),
            (SIGNATURE.into(), b"fn private(a: u32) -> bool".to_vec()),
        ]),
        reads: AtomicUsize::new(0),
    }
}

fn request(subject: &str, head: &str, output_bytes: usize) -> SourceDeckRequest {
    SourceDeckRequest {
        repository: RepositoryIdentity {
            owner: subject.into(),
            repository: "repo".into(),
            local_head: head.into(),
            root: "root-ref".into(),
        },
        allowed_roots: vec!["src".into(), "tests".into()],
        queries: vec![
            SourceQuery::Anchor("anchor/private".into()),
            SourceQuery::Excerpt("excerpt/private-signature".into()),
            SourceQuery::Specimen("spec/full-signature".into()),
        ],
        artifact_command: ArtifactCommand {
            identity: "xtask-repo-contract-v1".into(),
            program: "cargo".into(),
            argv: vec![
                "run".into(),
                "-p".into(),
                "xtask".into(),
                "--".into(),
                "repo-contract".into(),
                "--emit-stdout".into(),
                "--repo".into(),
                GUEST_ROOT.into(),
            ],
            working_directory: GUEST_ROOT.into(),
            environment: BTreeMap::from([("PATH".into(), "/toolchain/bin".into())]),
        },
        bounds: SourceDeckBounds {
            command_wall_time_ms: 10_000,
            command_output_bytes: output_bytes,
            deck: DeckLimits::strict(1, 1, 4, 4, 4, 4, 4096),
        },
    }
}

fn obs(key: &str, value: impl Into<String>) -> CheckObservation {
    CheckObservation {
        key: key.to_owned(),
        value: value.into(),
    }
}

fn failure(detail: &str) -> PackFailure {
    PackFailure {
        code: "missing-object",
        detail: detail.to_owned(),
    }
}

struct Launcher {
    stdout: Vec<u8>,
}

impl SandboxLauncher for Launcher {
    fn id(&self) -> &str {
        "fixture"
    }

    fn launch(&self, request: &SandboxRequest, _: &ProcessCancellation) -> SandboxAttempt {
        SandboxAttempt::Completed(SandboxResult {
            stdout: self.stdout.clone(),
            stderr: vec![],
            exit_code: 0,
            report: SandboxReport {
                launcher: "fixture".into(),
                controls: request
                    .policy
                    .requirements()
                    .keys()
                    .map(|control| SandboxEvidence {
                        control: *control,
                        achieved: true,
                        detail: "fixture proof".into(),
                    })
                    .collect(),
                limit_hits: vec![],
                cleanup: "complete".into(),
            },
        })
    }
}

struct Repo {
    files: BTreeMap<String, Vec<u8>>,
    reads: AtomicUsize,
}

impl SourceRepository for Repo {
    fn read_regular(
        &self,
        _: &RepositoryIdentity,
        path: &str,
    ) -> Result<Vec<u8>, SourceDeckProviderError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| SourceDeckProviderError::Read(path.into()))
    }
}

struct Fragment {
    owner: String,
}

impl FragmentDecoder for Fragment {
    fn decode(&self, _: &[u8]) -> Result<IndexFragment, sim_source_deck::Failure> {
        Ok(IndexFragment {
            owner: self.owner.clone(),
            anchors: vec![IndexAnchor {
                id: "anchor/private".into(),
                owner: self.owner.clone(),
                source_path: Some(LIB.into()),
            }],
            specimens: vec![IndexSpecimen {
                id: "spec/full-signature".into(),
                owner: self.owner.clone(),
            }],
        })
    }
}

struct Artifact {
    owner: String,
    paths: Vec<String>,
}

impl Artifact {
    fn happy(owner: &str) -> Self {
        Self {
            owner: owner.to_owned(),
            paths: vec![LIB.into()],
        }
    }
}

impl RepoContractDecoder for Artifact {
    fn decode(&self, bytes: &[u8]) -> Result<ArtifactProjection, SourceDeckProviderError> {
        let fragment_bytes = b"fragment".to_vec();
        let fragment_id = ByteContentId::of(&fragment_bytes)
            .map_err(|error| SourceDeckProviderError::Deck(error.to_string()))?;
        let mut certificate = ClaimCertificate {
            anchor: "anchor/private".into(),
            owner: self.owner.clone(),
            fragment_id: fragment_id.clone(),
            digest: ByteContentId::of(b"placeholder")
                .map_err(|error| SourceDeckProviderError::Deck(error.to_string()))?,
        };
        certificate.digest = certificate
            .expected_digest()
            .map_err(|error| SourceDeckProviderError::Deck(error.to_string()))?;
        Ok(ArtifactProjection {
            repository_head: HEAD.into(),
            fragment: FragmentPin {
                owner: self.owner.clone(),
                bytes: fragment_bytes,
                content_id: fragment_id,
            },
            certificates: vec![certificate],
            source_paths: self.paths.clone(),
            excerpts: vec![ArtifactExcerpt {
                id: "excerpt/private-signature".into(),
                path: LIB.into(),
                start: 0,
                end: 18,
            }],
            specimens: vec![ArtifactSpecimen {
                id: "spec/full-signature".into(),
                path: SIGNATURE.into(),
            }],
            limitations: vec![],
            artifact_id: ByteContentId::of(bytes)
                .map_err(|error| SourceDeckProviderError::Deck(error.to_string()))?,
        })
    }
}
