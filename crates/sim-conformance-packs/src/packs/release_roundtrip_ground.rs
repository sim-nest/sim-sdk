// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Deck and spec the authoring compiler reads for one selected member.

use std::collections::BTreeMap;

use sim_lib_roadmap_runner::{AuthorError, FamilyMember};
use sim_roadmap_core::{
    AcceptanceContract, CapabilityEnvelope, ChangeEnvelope, Charter, CheckpointId, CheckpointSpec,
    EffectEnvelope, ImplementationGuide, Limits, OwnerEnvelope, PhaseBody, PhaseId, PhaseOrigin,
    PhaseSpec, ProofPolicy, ResourceEnvelope, RoadmapId, RoadmapSpec, SchemaId,
};
use sim_source_deck::{
    ByteContentId, DeckInput, DeckLimits, FragmentDecoder, IndexFragment, RepositorySnapshot,
    SourceDeck, SourceFile,
};

use super::fail;
use crate::PackFailure;

pub(super) fn roadmap_spec(member: &FamilyMember) -> Result<RoadmapSpec, PackFailure> {
    let root_id =
        PhaseId::new("root").map_err(|error| fail("authoring-refused", &format!("{error:?}")))?;
    let leaf_id =
        PhaseId::new("values").map_err(|error| fail("authoring-refused", &format!("{error:?}")))?;
    let mut checkpoints = Vec::with_capacity(member.facts.len());
    for fact in &member.facts {
        checkpoints.push(CheckpointSpec {
            id: CheckpointId::new(fact).map_err(|_| fail("identity", "fact"))?,
            statement: "Named by the selected source.".to_owned(),
        });
    }
    let leaf = phase(
        leaf_id.clone(),
        Some(root_id.clone()),
        "Define values",
        "State one checkpoint",
        PhaseBody::Leaf { checkpoints },
    );
    let root = phase(
        root_id.clone(),
        None,
        "Value roadmap",
        "State bounded authored intent",
        PhaseBody::Composite {
            children: vec![leaf_id.clone()],
        },
    );
    Ok(RoadmapSpec {
        schema: SchemaId::new("roadmap-v1")
            .map_err(|error| fail("authoring-refused", &format!("{error:?}")))?,
        id: RoadmapId::new(&member.roadmap).map_err(|_| fail("identity", "roadmap"))?,
        charter: Charter {
            title: "Values".to_owned(),
            intent: "One roadmap states the checkpoint".to_owned(),
        },
        root: root_id,
        phases: [(root.id.clone(), root), (leaf.id.clone(), leaf)]
            .into_iter()
            .collect(),
        imports: BTreeMap::new(),
        limits: Limits::DEFAULT,
    })
}

fn phase(
    id: PhaseId,
    parent: Option<PhaseId>,
    title: &str,
    intent: &str,
    body: PhaseBody,
) -> PhaseSpec {
    PhaseSpec {
        id,
        parent,
        title: title.to_owned(),
        intent: intent.to_owned(),
        body,
        dependencies: Vec::new(),
        owners: OwnerEnvelope::default(),
        resources: ResourceEnvelope::default(),
        effects: EffectEnvelope::default(),
        capabilities: CapabilityEnvelope::default(),
        changes: ChangeEnvelope::default(),
        acceptance: AcceptanceContract {
            policy: ProofPolicy::All,
            statements: BTreeMap::new(),
        },
        coverage: Vec::new(),
        outputs: BTreeMap::new(),
        guide: ImplementationGuide::default(),
        origin: PhaseOrigin::Authored,
    }
}

pub(super) fn checkpoint_ids(spec: &RoadmapSpec) -> Vec<String> {
    let mut ids = Vec::new();
    for phase in spec.phases.values() {
        if let PhaseBody::Leaf { checkpoints } = &phase.body {
            ids.extend(checkpoints.iter().map(|item| item.id.as_str().to_owned()));
        }
    }
    ids
}

struct SilentDecoder;

impl FragmentDecoder for SilentDecoder {
    fn decode(&self, _: &[u8]) -> Result<IndexFragment, sim_source_deck::Failure> {
        Err(sim_source_deck::Failure::Decode("no fragment".into()))
    }
}

pub(super) fn source_deck(
    owner: &str,
    files: &[(String, Vec<u8>)],
) -> Result<SourceDeck, PackFailure> {
    let decoder = SilentDecoder;
    let mut built = Vec::with_capacity(files.len());
    let mut total = 0usize;
    for (path, bytes) in files {
        total += bytes.len();
        built.push(SourceFile {
            owner: owner.to_owned(),
            path: path.clone(),
            content_id: ByteContentId::of(bytes)
                .map_err(|error| fail("deck-mismatch", &format!("{error:?}")))?,
            bytes: bytes.clone(),
        });
    }
    sim_source_deck::build(DeckInput {
        repositories: vec![RepositorySnapshot {
            owner: owner.to_owned(),
            repository: owner.to_owned(),
            revision: "member".to_owned(),
        }],
        fragments: Vec::new(),
        certificates: Vec::new(),
        files: built,
        excerpts: Vec::new(),
        specimens: Vec::new(),
        queries: Vec::new(),
        limitations: Vec::new(),
        limits: DeckLimits::strict(1, 0, files.len(), 0, 0, 0, total),
        decoder: &decoder,
    })
    .map_err(|error| fail("deck-mismatch", &format!("{error:?}")))
}

pub(super) fn author_error(error: AuthorError) -> PackFailure {
    let detail = error.to_string();
    let code = match &error {
        AuthorError::Utf8
        | AuthorError::Header
        | AuthorError::Duplicate { .. }
        | AuthorError::UnknownField { .. }
        | AuthorError::Stance
        | AuthorError::Path
        | AuthorError::EmptyFamily
        | AuthorError::DuplicateMember { .. }
        | AuthorError::MemberMissing { .. } => "bad-record",
        AuthorError::Missing { .. } => "missing-section",
        AuthorError::Identity { .. } => "identity",
        AuthorError::SelectionCount { .. } => "selection-count",
        AuthorError::DeckMismatch { .. } => "deck-mismatch",
        AuthorError::RoadmapMismatch { .. } => "roadmap-mismatch",
        AuthorError::Ungrounded => "ungrounded",
        AuthorError::Unsatisfied { .. } => "unsatisfied",
        AuthorError::Authority => "selection-authority",
        AuthorError::Consult { .. } => "consult",
        AuthorError::Roadmap(_)
        | AuthorError::Revision(_)
        | AuthorError::Proof(_)
        | AuthorError::Mutation(_) => "authoring-refused",
    };
    let detail = match &error {
        AuthorError::Missing { section } => (*section).to_owned(),
        AuthorError::Identity { kind } => (*kind).to_owned(),
        _ => detail,
    };
    fail(code, &detail)
}
