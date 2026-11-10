// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Portable checker scopes that call the frozen executor.
//!
//! Each scope checks a result the executor returned. A scope that cannot call
//! that executor stays out of the implemented list.

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};

use sim_artifact_facet::{
    ArtifactFacet, ArtifactId, DisclosureDecisionId, MergePolicy, OwnerId, ProjectionId,
    RegionOwnership, RegionSelector,
};
use sim_kernel::ContentId;
use sim_lib_roadmap_runner::{
    Admit, CapabilityGrant, ControlIntent, DriveResult, FactGenerator, Fence, FixedStep,
    InputBatch, Liveness, LogicalTime, Mode, ProofOutcome, ProofUnit, Proposal, ProposalBody,
    ResourceAsk, RevisionOpen, SOURCE_SEAL, Session, Step, WakeEvent, failure_frontier,
    legal_order, open_revision, propose, resume_frontier, seal_fixed_point, seal_id,
};

use crate::executor::{self, Drive};
use crate::{CheckObservation, PackFailure};

pub(crate) fn closure(scope: &str, subject: &str) -> Result<Vec<CheckObservation>, PackFailure> {
    bind_closure(match scope {
        "closure/projection" => projection(subject),
        "closure/plan" => plan(subject),
        "closure/frontier" => frontier(subject),
        "closure/resources" => resources(subject),
        "closure/composed" => {
            let mut observations = projection(subject)?;
            observations.extend(plan(subject)?);
            observations.extend(frontier(subject)?);
            observations.extend(resources(subject)?);
            Ok(observations)
        }
        other => Err(wrong(other)),
    })
}

pub(crate) fn control(scope: &str, subject: &str) -> Result<Vec<CheckObservation>, PackFailure> {
    bind_closure(match scope {
        "control/liveness" => stopped(subject),
        "control/composed" => composed_control(subject),
        other => Err(wrong(other)),
    })
}

pub(crate) fn drive(scope: &str, subject: &str) -> Result<Vec<CheckObservation>, PackFailure> {
    bind_closure(match scope {
        "drive/hot-no-op" => hot_noop(subject),
        "drive/composed" => composed_drive(subject),
        other => Err(wrong(other)),
    })
}

pub(crate) fn converge(scope: &str, subject: &str) -> Result<Vec<CheckObservation>, PackFailure> {
    bind_closure(match scope {
        "convergence/candidate" => candidate(subject),
        "convergence/fixed-point" => fixed_point(subject),
        "convergence/composed" => {
            let mut observations = candidate(subject)?;
            observations.extend(fixed_point(subject)?);
            Ok(observations)
        }
        other => Err(wrong(other)),
    })
}

fn projection(subject: &str) -> Result<Vec<CheckObservation>, PackFailure> {
    let drive = Drive::open(executor::with_direction(subject)).map_err(show)?;
    let published = drive
        .conduct(&InputBatch::conduct(), &executor::grant("decide"))
        .map_err(show)?;
    let DriveResult::NeedDirection { request } = published else {
        return Err(show_text("direction was not published"));
    };
    if request.question != subject {
        return Err(show_text("direction dropped the subject"));
    }
    let mut batch = InputBatch::no_change();
    batch.reply = Some(executor::decision_reply(&request, "yes"));
    drive
        .conduct(&batch, &executor::grant("decide"))
        .map_err(show)?;
    let snapshot = drive
        .snapshots()
        .map_err(show)?
        .into_iter()
        .next_back()
        .ok_or_else(|| show_text("no retained snapshot"))?;
    let first = drive
        .read_snapshot(&snapshot.id, 0, 1, None)
        .map_err(show)?;
    let (revision, next, first_key) = match first {
        sim_lib_roadmap_runner::SnapshotRead::Page {
            revision,
            rows,
            continuation: Some(next),
            ..
        } if rows.len() == 1 && revision == snapshot.revision => {
            let expected_offset = rows.len();
            if usize::try_from(next).ok() != Some(expected_offset) {
                return Err(show_text("continuation skipped a row"));
            }
            let Some(first_row) = snapshot.rows.first() else {
                return Err(show_text("empty snapshot"));
            };
            if first_row.key != rows[0].key || first_row.value != rows[0].value {
                return Err(show_text("first page did not match the snapshot"));
            }
            (revision, next, rows[0].key.clone())
        }
        other => return Err(show_text(&format!("first page {other:?}"))),
    };
    let second = drive
        .read_snapshot(&snapshot.id, next, 1, None)
        .map_err(show)?;
    let followed = match &second {
        sim_lib_roadmap_runner::SnapshotRead::Page {
            revision: held,
            rows,
            ..
        } => {
            let start = usize::try_from(next).unwrap_or(usize::MAX);
            let expected = snapshot.rows.get(start..start + rows.len()).unwrap_or(&[]);
            held == &revision
                && rows.len() == 1
                && rows.len() == expected.len()
                && rows[0].key != first_key
                && rows
                    .iter()
                    .zip(expected)
                    .all(|(got, want)| got.key == want.key && got.value == want.value)
        }
        _ => false,
    };
    if !followed {
        return Err(show_text(&format!("second page {second:?}")));
    }
    let mut pause = InputBatch::no_change();
    pause.wakes.push(WakeEvent::Control(ControlIntent::Pause));
    drive
        .conduct(&pause, &CapabilityGrant::default())
        .map_err(show)?;
    let live = drive.revision().map_err(show)?;
    let stale = drive
        .read_snapshot(&snapshot.id, 0, 1, Some(&live))
        .map_err(show)?;
    if !matches!(stale, sim_lib_roadmap_runner::SnapshotRead::Stale { .. }) {
        return Err(show_text("moved revision was not stale"));
    }
    Ok(vec![
        obs("closure.page", "1"),
        obs("closure.revision", executor::hex_id(&revision)),
        obs("closure.stale", "stale"),
        obs("closure.subject", subject),
    ])
}

fn plan(subject: &str) -> Result<Vec<CheckObservation>, PackFailure> {
    let order =
        legal_order(&units(subject), &BTreeMap::new(), &caps(), &BTreeSet::new()).map_err(show)?;
    if order != [subject.to_owned()] {
        return Err(show_text(&format!("order {order:?}")));
    }
    Ok(vec![
        obs("closure.order", order[0].clone()),
        obs("closure.subject", subject),
    ])
}

fn frontier(subject: &str) -> Result<Vec<CheckObservation>, PackFailure> {
    let declared = units(subject);
    let mut outcomes = BTreeMap::new();
    outcomes.insert(subject.to_owned(), ProofOutcome::Failed);
    let first =
        failure_frontier(&declared, &outcomes, &caps(), &BTreeSet::new(), 1).map_err(show)?;
    let token = first
        .continuation
        .as_ref()
        .ok_or_else(|| show_text("frontier had no continuation"))?;
    let resumed = resume_frontier(
        &declared,
        &outcomes,
        &caps(),
        &BTreeSet::new(),
        &token.text,
        8,
    )
    .map_err(show)?;
    let failed = first
        .failures
        .iter()
        .chain(resumed.failures.iter())
        .any(|item| item.class == "proof-failed" && item.unit == subject);
    if !failed {
        return Err(show_text("failed proof was not on the frontier"));
    }
    Ok(vec![
        obs("closure.frontier", "proof-failed"),
        obs("closure.continued", "true"),
        obs("closure.subject", subject),
    ])
}

fn resources(subject: &str) -> Result<Vec<CheckObservation>, PackFailure> {
    let mut session =
        Session::new(vec![waiting_unit(subject)], caps(), BTreeMap::new()).map_err(show)?;
    match session.admit_next(LogicalTime(0)).map_err(show)? {
        Admit::Waiting { unit, kind, .. } if kind == "slot" && unit == subject => Ok(vec![
            obs("closure.waiting", unit),
            obs("closure.subject", subject),
        ]),
        other => Err(show_text(&format!("admit {other:?}"))),
    }
}

fn stopped(subject: &str) -> Result<Vec<CheckObservation>, PackFailure> {
    let mut session =
        Session::new(vec![ready_unit(subject)], caps(), BTreeMap::new()).map_err(show)?;
    match session.admit_next(LogicalTime(0)).map_err(show)? {
        Admit::Admitted { unit } if unit == subject => {}
        other => return Err(show_text(&format!("admit {other:?}"))),
    }
    let ack = session
        .submit(ControlIntent::Stop, LogicalTime(0))
        .map_err(show)?;
    if ack.mode != Mode::Stopped {
        return Err(show_text("stop left modeled work open"));
    }
    match session.observe_time(LogicalTime(0)).map_err(show)? {
        Liveness::Stopped {
            modeled_resources_closed: true,
        } => {}
        other => return Err(show_text(&format!("liveness {other:?}"))),
    }
    Ok(vec![
        obs("control.mode", "stopped"),
        obs("control.os-terminated", "false"),
        obs("control.subject", subject),
    ])
}

fn composed_control(subject: &str) -> Result<Vec<CheckObservation>, PackFailure> {
    let mut session =
        Session::new(vec![ready_unit(subject)], caps(), BTreeMap::new()).map_err(show)?;
    match session.admit_next(LogicalTime(0)).map_err(show)? {
        Admit::Admitted { unit } if unit == subject => {}
        other => return Err(show_text(&format!("admit {other:?}"))),
    }
    let stalled = session.observe_time(LogicalTime(30)).map_err(show)?;
    if !matches!(stalled, Liveness::Stalled { .. }) {
        return Err(show_text(&format!("stall {stalled:?}")));
    }
    let paused = session
        .submit(ControlIntent::Pause, LogicalTime(31))
        .map_err(show)?;
    if paused.mode != Mode::Paused {
        return Err(show_text("pause did not hold"));
    }
    let resumed = session
        .submit(ControlIntent::Resume, LogicalTime(32))
        .map_err(show)?;
    if resumed.mode != Mode::Running {
        return Err(show_text("resume did not return to running"));
    }
    let drained = session
        .submit(ControlIntent::Drain, LogicalTime(33))
        .map_err(show)?;
    if drained.mode != Mode::Draining {
        return Err(show_text("drain still admitted work"));
    }
    let stopped = session
        .submit(ControlIntent::Stop, LogicalTime(34))
        .map_err(show)?;
    if stopped.mode != Mode::Stopped {
        return Err(show_text("stop left modeled work open"));
    }
    Ok(vec![
        obs("control.drained", "draining"),
        obs("control.mode", "stopped"),
        obs("control.os-terminated", "false"),
        obs("control.paused", "paused"),
        obs("control.resumed", "running"),
        obs("control.stalled", "stalled"),
        obs("control.subject", subject),
    ])
}

fn hot_noop(subject: &str) -> Result<Vec<CheckObservation>, PackFailure> {
    let measured = crate::hot_noop::measure().map_err(|detail| show_text(&detail))?;
    if measured.effect_delta != 0 || measured.samples_us.len() != 20 {
        return Err(show_text("hot status performed work"));
    }
    let raw_us = measured
        .samples_us
        .iter()
        .map(u64::to_string)
        .collect::<Vec<_>>()
        .join(" ");
    Ok(vec![
        obs("drive.hot-delta", measured.effect_delta.to_string()),
        obs("drive.samples", measured.samples_us.len().to_string()),
        obs("drive.raw-us", raw_us),
        obs("drive.subject", subject),
    ])
}

fn composed_drive(subject: &str) -> Result<Vec<CheckObservation>, PackFailure> {
    let mut observations = hot_noop(subject)?;
    let drive = Drive::open(executor::with_direction(subject)).map_err(show)?;
    let published = drive
        .conduct(&InputBatch::conduct(), &executor::grant("decide"))
        .map_err(show)?;
    if !matches!(published, DriveResult::NeedDirection { .. }) || drive.calls() != 0 {
        return Err(show_text("direction request performed an effect"));
    }
    observations.push(obs("drive.direction", "need-direction"));
    Ok(observations)
}

fn candidate(subject: &str) -> Result<Vec<CheckObservation>, PackFailure> {
    let mut revision = world(subject)?;
    let before = revision.id.clone();
    let mut post = subject.as_bytes().to_vec();
    post.push(b'x');
    let step = propose(
        &mut revision,
        Proposal {
            author: "owner/outside".into(),
            checker: "checker/converge".into(),
            fence: Fence::BeforeCommit,
            body: ProposalBody::Complete {
                id: "proposal/subject".into(),
                facts: BTreeMap::from([("fact/src".into(), post)]),
                limit: subject.len().saturating_add(64),
            },
        },
    )
    .map_err(show)?;
    if !matches!(step, Step::Held { .. }) || revision.id != before {
        return Err(show_text(&format!("candidate {step:?}")));
    }
    Ok(vec![
        obs("converge.held", "held"),
        obs("converge.revision-stable", "true"),
        obs("converge.subject", subject),
    ])
}

fn fixed_point(subject: &str) -> Result<Vec<CheckObservation>, PackFailure> {
    let revision = world(subject)?;
    let generator = Gen::new(subject);
    let generators: [&dyn FactGenerator; 1] = [&generator];
    let first = seal_fixed_point(
        &revision,
        &facets(&generator.owner),
        &generators,
        &proofs(),
        &BTreeSet::new(),
        None,
    )
    .map_err(show)?;
    let FixedStep::Sealed { seal, work } = first else {
        return Err(show_text(&format!("seal {first:?}")));
    };
    if work == 0 || generator.calls.get() != work {
        return Err(show_text("seal did not run the generator"));
    }
    let second = seal_fixed_point(
        &revision,
        &facets(&generator.owner),
        &generators,
        &proofs(),
        &BTreeSet::new(),
        Some(&seal),
    )
    .map_err(show)?;
    if !matches!(second, FixedStep::ZeroWork { .. }) || generator.calls.get() != work {
        return Err(show_text(&format!("second seal {second:?}")));
    }
    Ok(vec![
        obs("converge.calls", generator.calls.get().to_string()),
        obs("converge.sealed", "sealed"),
        obs("converge.subject", subject),
        obs("converge.zero-work", "zero-work"),
    ])
}

fn world(subject: &str) -> Result<sim_lib_roadmap_runner::Revision, PackFailure> {
    let seal = subject.as_bytes().to_vec();
    let mut images = BTreeMap::new();
    images.insert(SOURCE_SEAL.to_owned(), seal.clone());
    images.insert("fact/src".into(), subject.as_bytes().to_vec());
    images.insert("fact/alpha".into(), subject.as_bytes().to_vec());
    open_revision(RevisionOpen {
        seal_id: seal_id(&seal).map_err(show)?,
        seal,
        images,
        claims: vec![
            sim_lib_roadmap_runner::OpenClaim {
                id: "claim/src".into(),
                inputs: BTreeSet::from(["fact/src".into()]),
            },
            sim_lib_roadmap_runner::OpenClaim {
                id: "claim/alpha".into(),
                inputs: BTreeSet::from(["fact/alpha".into()]),
            },
        ],
    })
    .map_err(show)
}

fn facets(owner: &OwnerId) -> Vec<ArtifactFacet> {
    vec![
        facet("artifact/src", owner, "fact/src", false),
        facet("artifact/alpha", owner, "fact/alpha", true),
    ]
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

fn proofs() -> BTreeMap<String, ProofOutcome> {
    BTreeMap::from([
        ("claim/src".into(), ProofOutcome::Passed),
        ("claim/alpha".into(), ProofOutcome::Passed),
    ])
}

struct Gen {
    owner: OwnerId,
    reads: BTreeSet<String>,
    writes: BTreeSet<String>,
    implementation: ContentId,
    calls: Cell<u32>,
}

impl Gen {
    fn new(subject: &str) -> Self {
        let _ = subject;
        Self {
            owner: OwnerId::from_text("owner/outside").expect("owner"),
            reads: BTreeSet::from([SOURCE_SEAL.to_owned(), "fact/src".into()]),
            writes: BTreeSet::from(["fact/alpha".into()]),
            implementation: executor::cid(9),
            calls: Cell::new(0),
        }
    }
}

impl FactGenerator for Gen {
    fn owner(&self) -> &OwnerId {
        &self.owner
    }
    fn reads(&self) -> &BTreeSet<String> {
        &self.reads
    }
    fn writes(&self) -> &BTreeSet<String> {
        &self.writes
    }
    fn implementation(&self) -> ContentId {
        self.implementation.clone()
    }
    fn generate(&self, source: &BTreeMap<String, Vec<u8>>) -> BTreeMap<String, Vec<u8>> {
        self.calls.set(self.calls.get().saturating_add(1));
        let bytes = source.get("fact/src").cloned().unwrap_or_default();
        BTreeMap::from([("fact/alpha".into(), bytes)])
    }
}

fn units(subject: &str) -> Vec<ProofUnit> {
    vec![
        ready_unit(subject),
        ProofUnit {
            id: "linux".into(),
            capability: "domain/linux".into(),
            ..ready_unit("linux")
        },
    ]
}

fn ready_unit(id: &str) -> ProofUnit {
    ProofUnit {
        id: id.to_owned(),
        depends_on: vec![],
        capability: "review".into(),
        invalidation_mass: 1,
        learned_cost: 1,
        critical_path_gain: 0,
        novelty_gain: 0,
        resources: vec![],
    }
}

fn waiting_unit(subject: &str) -> ProofUnit {
    ProofUnit {
        resources: vec![ResourceAsk {
            kind: "slot".into(),
            amount: 1,
        }],
        ..ready_unit(subject)
    }
}

fn caps() -> BTreeSet<String> {
    BTreeSet::from(["review".into()])
}

fn bind_closure(
    result: Result<Vec<CheckObservation>, PackFailure>,
) -> Result<Vec<CheckObservation>, PackFailure> {
    result.map(|mut observations| {
        observations.push(obs("executor.closure", crate::compiled_executor_closure()));
        observations
    })
}

fn obs(key: &str, value: impl Into<String>) -> CheckObservation {
    CheckObservation {
        key: key.to_owned(),
        value: value.into(),
    }
}

fn wrong(scope: &str) -> PackFailure {
    PackFailure {
        code: "wrong-scope",
        detail: scope.to_owned(),
    }
}

fn show(error: impl std::fmt::Display) -> PackFailure {
    show_text(&error.to_string())
}

fn show_text(detail: &str) -> PackFailure {
    PackFailure {
        code: "missing-object",
        detail: detail.to_owned(),
    }
}
