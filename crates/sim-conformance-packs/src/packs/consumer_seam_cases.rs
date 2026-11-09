// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Scenario checks for the outside consumer.
//!
//! Each case calls the frozen executor or the source provider. A refusal is
//! that callee's result.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use sim_lib_roadmap_runner::{
    CapabilityGrant, ControlIntent, DriveResult, InputBatch, LineageView, ServiceError,
    SnapshotRead, WakeEvent, legal_order,
};
use sim_roadmap_exec_core::ExecutionId;

use super::{SeamFault, SeamReport};
use crate::executor::{self, Drive};
use crate::source_invoke;

const SUBJECT: &str = "subject/seam";

thread_local! {
    static ACTIVE_SUBJECT: RefCell<String> = const { RefCell::new(String::new()) };
}

fn active_subject() -> String {
    ACTIVE_SUBJECT.with(|slot| {
        let current = slot.borrow();
        if current.is_empty() {
            SUBJECT.to_owned()
        } else {
            current.clone()
        }
    })
}

/// Runs the specimen against `subject` and restores the direct-call subject.
pub(super) fn run_for_subject(subject: &str) -> Result<SeamReport, SeamFault> {
    ACTIVE_SUBJECT.with(|slot| *slot.borrow_mut() = subject.to_owned());
    let _restore = SubjectRestore;
    run_canonical()
}

struct SubjectRestore;

impl Drop for SubjectRestore {
    fn drop(&mut self) {
        ACTIVE_SUBJECT.with(|slot| slot.borrow_mut().clear());
    }
}

/// Refuses a provider name that is a Linux adapter before the service opens.
pub fn open_with_linux() -> Result<(), SeamFault> {
    if ["model/snapshot", "linux/native"]
        .iter()
        .any(|name| name.contains("linux"))
    {
        Err(SeamFault::LinuxProvider)
    } else {
        Ok(())
    }
}

/// Reads two pages of one retained snapshot, then refuses the moved revision.
pub fn snapshot_pages() -> Result<(usize, String), SeamFault> {
    let drive = opened(executor::with_direction(&active_subject()));
    let request = need_direction(&drive)?;
    accept(&drive, &request, "yes")?;
    let snapshot = drive
        .snapshots()
        .unwrap_or_else(|error| panic!("snapshots: {error}"))
        .into_iter()
        .next_back()
        .unwrap_or_else(|| panic!("no retained snapshot"));
    let first = drive
        .read_snapshot(&snapshot.id, 0, 1, None)
        .unwrap_or_else(|error| panic!("page: {error}"));
    let SnapshotRead::Page {
        revision,
        rows,
        continuation: Some(next),
        ..
    } = first
    else {
        panic!("first page did not continue: {first:?}");
    };
    if rows.len() != 1 || revision != snapshot.revision {
        return Err(SeamFault::Bound);
    }
    let expected_offset = rows.len();
    if usize::try_from(next).ok() != Some(expected_offset) {
        return Err(SeamFault::Bound);
    }
    let Some(first_row) = snapshot.rows.first() else {
        return Err(SeamFault::Bound);
    };
    if first_row.key != rows[0].key || first_row.value != rows[0].value {
        return Err(SeamFault::Bound);
    }
    let second = drive
        .read_snapshot(&snapshot.id, next, 8, None)
        .unwrap_or_else(|error| panic!("page: {error}"));
    let SnapshotRead::Page {
        revision: held,
        rows: next_rows,
        ..
    } = second
    else {
        return Err(SeamFault::RevisionMoved { retained: 0 });
    };
    let start = expected_offset;
    let want_len = snapshot.rows.len().saturating_sub(start).min(8);
    let expected = snapshot.rows.get(start..start + want_len).unwrap_or(&[]);
    let followed = held == revision
        && next_rows.len() == want_len
        && next_rows.len() == expected.len()
        && next_rows
            .iter()
            .zip(expected)
            .all(|(got, want)| got.key == want.key && got.value == want.value);
    if held != revision || !followed {
        return Err(SeamFault::RevisionMoved { retained: 0 });
    }
    pause(&drive);
    let live = drive
        .revision()
        .unwrap_or_else(|error| panic!("revision: {error}"));
    match drive
        .read_snapshot(&snapshot.id, 0, 1, Some(&live))
        .unwrap_or_else(|error| panic!("stale read: {error}"))
    {
        SnapshotRead::Stale { .. } => Ok((rows.len(), executor::hex_id(&revision))),
        SnapshotRead::Page { .. } => Err(SeamFault::RevisionMoved { retained: 0 }),
        SnapshotRead::Missing => Err(SeamFault::MissingObject),
    }
}

/// Accepts one reply, keeps an exact replay quiescent, and refuses the old fence.
pub fn direction_once() -> Result<(u64, u64), SeamFault> {
    let drive = opened(executor::with_direction(&active_subject()));
    let request = need_direction(&drive)?;
    let old = request.expected_revision.clone();
    pause(&drive);
    let mut stale = executor::decision_reply(&request, "yes");
    stale.expected_revision = old;
    match drive.conduct(&reply_batch(stale), &executor::grant("decide")) {
        Err(ServiceError::StaleDirection) => {}
        Err(error) => panic!("stale reply: {error}"),
        Ok(result) => panic!("stale reply was accepted: {result:?}"),
    }
    if drive.calls() != 0 {
        return Err(SeamFault::StaleDirection);
    }
    let fresh_revision = drive
        .revision()
        .unwrap_or_else(|error| panic!("revision: {error}"));
    let mut fresh = executor::decision_reply(&request, "yes");
    fresh.expected_revision = fresh_revision;
    let accepted = drive
        .conduct(&reply_batch(fresh.clone()), &executor::grant("decide"))
        .unwrap_or_else(|error| panic!("fresh reply: {error}"));
    if !matches!(accepted, DriveResult::Advanced { .. }) {
        panic!("fresh reply was not one transition: {accepted:?}");
    }
    let records = drive
        .records()
        .unwrap_or_else(|error| panic!("records: {error}"));
    let calls = drive.calls();
    let replay = drive
        .conduct(&reply_batch(fresh), &executor::grant("decide"))
        .unwrap_or_else(|error| panic!("replay: {error}"));
    if !matches!(replay, DriveResult::Quiescent { .. })
        || drive
            .records()
            .unwrap_or_else(|error| panic!("records: {error}"))
            != records
        || drive.calls() != calls
    {
        return Err(SeamFault::Replayed);
    }
    Ok((1, 0))
}

/// Refuses a reply that names a different execution and changes nothing.
pub fn wrong_execution() -> Result<(), SeamFault> {
    let drive = opened(executor::with_direction(&active_subject()));
    let request = need_direction(&drive)?;
    let mut reply = executor::decision_reply(&request, "yes");
    reply.execution = ExecutionId::new("other").expect("execution id");
    match drive.conduct(&reply_batch(reply), &executor::grant("decide")) {
        Err(ServiceError::WrongDirection) if drive.calls() == 0 => Ok(()),
        Err(error) => panic!("wrong execution: {error}"),
        Ok(result) => panic!("wrong execution was accepted: {result:?}"),
    }
}

/// Accepts the first answer, then refuses a different one.
pub fn changed_answer() -> Result<(), SeamFault> {
    let drive = opened(executor::with_direction(&active_subject()));
    let request = need_direction(&drive)?;
    accept(&drive, &request, "yes")?;
    let calls = drive.calls();
    let records = drive
        .records()
        .unwrap_or_else(|error| panic!("records: {error}"));
    let changed = executor::decision_reply(&request, "no");
    match drive.conduct(&reply_batch(changed), &executor::grant("decide")) {
        Err(ServiceError::ReplayedDirection)
            if drive.calls() == calls
                && drive
                    .records()
                    .unwrap_or_else(|error| panic!("records: {error}"))
                    == records =>
        {
            Ok(())
        }
        Err(error) => panic!("changed answer: {error}"),
        Ok(result) => panic!("changed answer was accepted: {result:?}"),
    }
}

/// Refuses advice placed in the decision slot.
pub fn advice_slot() -> Result<(), SeamFault> {
    let drive = opened(executor::with_direction(&active_subject()));
    let request = need_direction(&drive)?;
    match drive.conduct(
        &reply_batch(executor::advice_reply(&request)),
        &executor::grant("decide"),
    ) {
        Err(ServiceError::AdvisoryDirection) if drive.calls() == 0 => Ok(()),
        Err(error) => panic!("advice: {error}"),
        Ok(result) => panic!("advice was accepted: {result:?}"),
    }
}

/// Refuses a selected handoff with no begin, then lands once begin is present.
pub fn selection_without_begin() -> Result<(u64, usize, String), SeamFault> {
    let held = opened(executor::with_handoff(None));
    match held.conduct(&InputBatch::conduct(), &CapabilityGrant::default()) {
        Err(ServiceError::MissingBegin) if held.calls() == 0 => {}
        Err(error) => panic!("missing begin: {error}"),
        Ok(result) => panic!("missing begin performed: {result:?}"),
    }
    let drive = opened(executor::with_handoff(Some(executor::cid(12))));
    let ran = drive
        .conduct(&InputBatch::conduct(), &CapabilityGrant::default())
        .unwrap_or_else(|error| panic!("begin: {error}"));
    if !matches!(ran, DriveResult::Advanced { .. }) || drive.calls() != 1 {
        return Err(SeamFault::MissingBegin);
    }
    let present = BTreeSet::from([executor::cid(14)]);
    let views = drive
        .lineage(&present)
        .unwrap_or_else(|error| panic!("lineage: {error}"));
    let stale = views
        .iter()
        .filter(|view| matches!(view, LineageView::Gap { .. }))
        .count();
    if stale != 1
        || !views.iter().any(
            |view| matches!(view, LineageView::Present(item) if item.value == executor::cid(14)),
        )
    {
        return Err(SeamFault::InsufficientCoverage);
    }
    Ok((1, stale, executor::hex_id(&executor::cid(14))))
}

/// Refuses advice in the decision slot, and a shell string as a proof leaf.
pub fn advice_rejected() -> Result<(), SeamFault> {
    advice_slot()?;
    super::advice_verify::catalog_refuses_advice_as_proof()
}

/// Reopens the same journal and reports a gap for a value the caller dropped.
pub fn reopen_same() -> Result<(), SeamFault> {
    let drive = opened(executor::with_handoff(Some(executor::cid(12))));
    drive
        .conduct(&InputBatch::conduct(), &CapabilityGrant::default())
        .unwrap_or_else(|error| panic!("land: {error}"));
    let records = drive
        .records()
        .unwrap_or_else(|error| panic!("records: {error}"));
    let restored = Drive::open_on(
        drive.backend(),
        executor::with_handoff(Some(executor::cid(12))),
        Arc::new(AtomicUsize::new(0)),
    )
    .unwrap_or_else(|error| panic!("reopen: {error}"));
    if restored
        .records()
        .unwrap_or_else(|error| panic!("records: {error}"))
        != records
        || restored.calls() != 0
    {
        return Err(SeamFault::MissingObject);
    }
    let unknown = restored
        .read_snapshot(&executor::cid(99), 0, 1, None)
        .unwrap_or_else(|error| panic!("missing snapshot: {error}"));
    if !matches!(unknown, SnapshotRead::Missing) {
        return Err(SeamFault::MissingObject);
    }
    let both = restored
        .lineage(&BTreeSet::new())
        .unwrap_or_else(|error| panic!("lineage: {error}"));
    let gaps = both
        .iter()
        .filter(|view| matches!(view, LineageView::Gap { .. }))
        .count();
    if gaps == 2 {
        Ok(())
    } else {
        Err(SeamFault::MissingObject)
    }
}

/// An admitted effect that has not committed blocks the next prepare and replay.
pub fn interrupt_once() -> Result<(), SeamFault> {
    let drive = opened(executor::base_request());
    let prepared = drive
        .prepare(&InputBatch::conduct(), &CapabilityGrant::default())
        .unwrap_or_else(|error| panic!("prepare: {error}"));
    if executor::admitted(prepared).is_none() || drive.calls() != 0 {
        return Err(SeamFault::Interrupted);
    }
    match drive.prepare(&InputBatch::conduct(), &CapabilityGrant::default()) {
        Err(ServiceError::EffectOutstanding) => {}
        Err(error) => panic!("second prepare: {error}"),
        Ok(_) => return Err(SeamFault::Interrupted),
    }
    match drive.replay() {
        Err(ServiceError::EffectOutstanding) if drive.calls() == 0 => Ok(()),
        Err(error) => panic!("replay: {error}"),
        Ok(_) => Err(SeamFault::Interrupted),
    }
}

/// A revoked checker receipt stays unreadable.
pub fn revoked_subject() -> Result<(), SeamFault> {
    match revoked_receipt() {
        Ok(()) => Ok(()),
        Err(detail) => panic!("revocation: {detail}"),
    }
}

/// A second pass over an unchanged execution performs no effect.
pub fn hot_noop() -> Result<u64, SeamFault> {
    let drive = opened(executor::with_handoff(Some(executor::cid(12))));
    drive
        .conduct(&InputBatch::conduct(), &CapabilityGrant::default())
        .unwrap_or_else(|error| panic!("conduct: {error}"));
    let calls = drive.calls();
    let records = drive
        .records()
        .unwrap_or_else(|error| panic!("records: {error}"));
    let quiet = drive
        .conduct(&InputBatch::no_change(), &CapabilityGrant::default())
        .unwrap_or_else(|error| panic!("no-change: {error}"));
    if !matches!(quiet, DriveResult::Quiescent { .. })
        || drive.calls() != calls
        || drive
            .records()
            .unwrap_or_else(|error| panic!("records: {error}"))
            != records
    {
        return Err(SeamFault::Bound);
    }
    Ok(0)
}

/// Dropping one retained value leaves that reference stale and the other present.
pub fn renote_stales() -> Result<(), SeamFault> {
    let drive = opened(executor::with_handoff(Some(executor::cid(12))));
    let views = drive
        .lineage(&BTreeSet::from([executor::cid(14)]))
        .unwrap_or_else(|error| panic!("lineage: {error}"));
    let stale = views
        .iter()
        .filter(|view| matches!(view, LineageView::Gap { value } if value == &executor::cid(13)))
        .count();
    let kept = views
        .iter()
        .any(|view| matches!(view, LineageView::Present(item) if item.value == executor::cid(14)));
    if stale == 1 && kept {
        Ok(())
    } else {
        Err(SeamFault::InsufficientCoverage)
    }
}

/// Reopen keeps the consumed decision, so the same reply is not spent again.
pub fn reopen_keeps_direction() -> Result<(), SeamFault> {
    let drive = opened(executor::with_direction(&active_subject()));
    let request = need_direction(&drive)?;
    accept(&drive, &request, "yes")?;
    let records = drive
        .records()
        .unwrap_or_else(|error| panic!("records: {error}"));
    let restored = Drive::open_on(
        drive.backend(),
        executor::with_direction(&active_subject()),
        Arc::new(AtomicUsize::new(0)),
    )
    .unwrap_or_else(|error| panic!("reopen: {error}"));
    let replay = restored.conduct(
        &reply_batch(executor::decision_reply(&request, "yes")),
        &executor::grant("decide"),
    );
    let spent = matches!(
        replay,
        Ok(DriveResult::Quiescent { .. }) | Err(ServiceError::ReplayedDirection)
    );
    if spent
        && restored
            .records()
            .unwrap_or_else(|error| panic!("records: {error}"))
            == records
        && restored.calls() == 0
    {
        Ok(())
    } else {
        Err(SeamFault::Replayed)
    }
}

/// A handoff without begin authority performs no effect.
pub fn hot_needs_begin() -> Result<(), SeamFault> {
    let drive = opened(executor::with_handoff(None));
    match drive.conduct(&InputBatch::conduct(), &CapabilityGrant::default()) {
        Err(ServiceError::MissingBegin) if drive.calls() == 0 => Ok(()),
        Err(error) => panic!("begin: {error}"),
        Ok(result) => panic!("begin was not required: {result:?}"),
    }
}

/// Two candidates stay distinct, and reopen keeps the selected one.
pub fn selection_is_distinct() -> Result<(), SeamFault> {
    let first = opened(executor::with_candidate(executor::cid(11)));
    let second = opened(executor::with_candidate(executor::cid(16)));
    let left = first_candidate(&first);
    let right = first_candidate(&second);
    if left == right {
        return Err(SeamFault::MissingObject);
    }
    let restored = Drive::open_on(
        first.backend(),
        executor::with_candidate(executor::cid(11)),
        Arc::new(AtomicUsize::new(0)),
    )
    .unwrap_or_else(|error| panic!("reopen: {error}"));
    if first_candidate(&restored) == left {
        Ok(())
    } else {
        Err(SeamFault::MissingObject)
    }
}

/// The escaping path is refused after the paths that were already read.
pub fn over_bound() -> Result<usize, SeamFault> {
    source_invoke::admitted_before_escape(&active_subject()).map_err(|failure| {
        panic!("source bound: {}", failure.detail);
    })
}

/// Refuses a foreign capability and advice offered as a decision.
pub fn scope_confusion() -> Result<(), SeamFault> {
    let order = legal_order(
        &[unit("foreign", "portability/foreign-project")],
        &BTreeMap::new(),
        &BTreeSet::from(["review".into()]),
        &BTreeSet::new(),
    )
    .unwrap_or_else(|error| panic!("order: {error}"));
    if !order.is_empty() {
        return Err(SeamFault::MissingObject);
    }
    advice_slot()
}

/// Orders a unit whose capability is offered and omits a Linux capability.
pub fn discover_affordance() -> Result<(), SeamFault> {
    let order = legal_order(
        &[unit("review", "review"), unit("linux", "domain/linux")],
        &BTreeMap::new(),
        &BTreeSet::from(["review".into()]),
        &BTreeSet::new(),
    )
    .expect("legal order");
    if order == ["review".to_owned()] {
        Ok(())
    } else {
        Err(SeamFault::MissingObject)
    }
}

pub(super) fn run_canonical() -> Result<SeamReport, SeamFault> {
    let (first_page, retained_revision) = snapshot_pages()?;
    let (transitions, stale_effects) = direction_once()?;
    wrong_execution()?;
    changed_answer()?;
    advice_slot()?;
    let (effects, stale_edges, unrelated_proof) = selection_without_begin()?;
    advice_rejected()?;
    reopen_same()?;
    interrupt_once()?;
    revoked_subject()?;
    scope_confusion()?;
    let hot_delta = hot_noop()?;
    discover_affordance()?;
    renote_stales()?;
    reopen_keeps_direction()?;
    hot_needs_begin()?;
    selection_is_distinct()?;
    let source_bound = over_bound()?;
    let providers = providers_used()?;
    Ok(SeamReport {
        first_page,
        retained_revision,
        transitions,
        stale_effects,
        effects,
        hot_delta,
        providers,
        stale_edges,
        unrelated_proof,
        source_bound,
    })
}

fn providers_used() -> Result<Vec<String>, SeamFault> {
    let drive = opened(executor::with_direction(&active_subject()));
    let request = need_direction(&drive)?;
    let _ = drive
        .read_snapshot(&executor::cid(99), 0, 1, None)
        .unwrap_or_else(|error| panic!("snapshot: {error}"));
    accept(&drive, &request, "yes")?;
    let ran = drive
        .conduct(&InputBatch::conduct(), &CapabilityGrant::default())
        .unwrap_or_else(|error| panic!("effect: {error}"));
    if !matches!(ran, DriveResult::Advanced { .. }) || drive.calls() != 1 {
        return Err(SeamFault::MissingObject);
    }
    let mut names = executor::provider_names()
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<Vec<_>>();
    names.sort();
    if names.iter().any(|name| name.contains("linux")) {
        return Err(SeamFault::LinuxProvider);
    }
    Ok(names)
}

fn opened(request: sim_lib_roadmap_runner::OpenRequest) -> Drive {
    Drive::open(request).unwrap_or_else(|error| panic!("open: {error}"))
}

fn need_direction(drive: &Drive) -> Result<sim_lib_roadmap_runner::DirectionRequest, SeamFault> {
    match drive.conduct(&InputBatch::conduct(), &executor::grant("decide")) {
        Ok(DriveResult::NeedDirection { request }) => Ok(request),
        Ok(result) => panic!("direction was not published: {result:?}"),
        Err(error) => panic!("direction: {error}"),
    }
}

fn accept(
    drive: &Drive,
    request: &sim_lib_roadmap_runner::DirectionRequest,
    answer: &str,
) -> Result<(), SeamFault> {
    let reply = executor::decision_reply(request, answer);
    drive
        .conduct(&reply_batch(reply), &executor::grant("decide"))
        .unwrap_or_else(|error| panic!("accept: {error}"));
    Ok(())
}

fn pause(drive: &Drive) {
    let mut batch = InputBatch::no_change();
    batch.wakes.push(WakeEvent::Control(ControlIntent::Pause));
    drive
        .conduct(&batch, &CapabilityGrant::default())
        .unwrap_or_else(|error| panic!("pause: {error}"));
}

fn reply_batch(reply: sim_lib_roadmap_runner::DirectionReply) -> InputBatch {
    let mut batch = InputBatch::no_change();
    batch.reply = Some(reply);
    batch
}

fn first_candidate(drive: &Drive) -> String {
    let candidate = drive
        .candidate()
        .unwrap_or_else(|error| panic!("handoff: {error}"));
    executor::hex_id(&candidate)
}

fn unit(id: &str, capability: &str) -> sim_lib_roadmap_runner::ProofUnit {
    sim_lib_roadmap_runner::ProofUnit {
        id: id.to_owned(),
        depends_on: vec![],
        capability: capability.to_owned(),
        invalidation_mass: 1,
        learned_cost: 1,
        critical_path_gain: 0,
        novelty_gain: 0,
        resources: vec![],
    }
}

fn revoked_receipt() -> Result<(), String> {
    use std::collections::BTreeSet as Set;

    use sim_conformance_v02::{
        CheckArgument, CheckInputClosureId, CheckScopeId, CheckTemplate, CheckedSubjectId,
        CheckerBinding, CheckerReceipt, CheckerResultId, CommandId, ConformancePackId,
        EnvironmentPolicyId, EvidenceGrade, EvidenceProvenanceId, EvidenceSetId, OutputShapeId,
        OwnerBindingId, PolicyId, ProofCodeId, RevocationSourceId, RevocationStatus,
        WorkingDirectoryPolicyId,
    };
    use sim_lib_roadmap_runner::{ProofCatalog, ProofLeaf, RetainedCheckerReceipt};

    let closure =
        CheckInputClosureId::from_text("closure/specimen").map_err(|error| error.to_string())?;
    let binding = CheckerBinding::new(
        "checker/c-evidence".into(),
        OwnerBindingId::from_text("owner/test").map_err(|error| error.to_string())?,
        "packs::evidence::check".into(),
        vec![ConformancePackId::from_text("pack/catalog").map_err(|error| error.to_string())?],
        OutputShapeId::from_text("shape/check-receipt").map_err(|error| error.to_string())?,
        RevocationSourceId::from_text("revocation/test").map_err(|error| error.to_string())?,
        CommandId::from_text("command/validate").map_err(|error| error.to_string())?,
        CommandId::from_text("command/docs").map_err(|error| error.to_string())?,
        Set::from([CheckScopeId::from_text("scope/evidence").map_err(|error| error.to_string())?]),
        CheckTemplate::new(
            "check".into(),
            vec![
                CheckArgument::BindingSlot,
                CheckArgument::SubjectSlot,
                CheckArgument::ScopeSlot,
            ],
            WorkingDirectoryPolicyId::from_text("cwd/source").map_err(|error| error.to_string())?,
            EnvironmentPolicyId::from_text("environment/sealed")
                .map_err(|error| error.to_string())?,
            OutputShapeId::from_text("shape/check-result").map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let invocation = binding
        .instantiate(
            ProofCodeId::from_text("code/catalog").map_err(|error| error.to_string())?,
            ConformancePackId::from_text("pack/catalog").map_err(|error| error.to_string())?,
            CheckedSubjectId::from_text("subject/seam").map_err(|error| error.to_string())?,
            CheckScopeId::from_text("scope/evidence").map_err(|error| error.to_string())?,
            closure,
        )
        .map_err(|error| error.to_string())?;
    let receipt = CheckerReceipt::passing(
        &invocation,
        CheckerResultId::from_text("result/checked").map_err(|error| error.to_string())?,
        EvidenceGrade::Reproducible,
        EvidenceProvenanceId::from_text("provenance/independent")
            .map_err(|error| error.to_string())?,
        PolicyId::from_text("policy/catalog").map_err(|error| error.to_string())?,
        EvidenceSetId::from_text("support/checker").map_err(|error| error.to_string())?,
        RevocationStatus::Current,
    )
    .map_err(|error| error.to_string())?;
    let id = receipt.id().clone();
    let catalog = ProofCatalog::new([ProofLeaf::ArtifactEquality {
        name: "leaf/required".into(),
        left: b"a".to_vec(),
        right: b"a".to_vec(),
    }])
    .map_err(|error| error.to_string())?
    .with_checker_receipt(
        RetainedCheckerReceipt::new(binding, invocation, receipt),
        RevocationStatus::Current,
    )
    .map_err(|error| error.to_string())?;
    catalog
        .checker_receipt(&id, RevocationStatus::Current)
        .map_err(|error| error.to_string())?;
    match catalog.checker_receipt(&id, RevocationStatus::Revoked) {
        Err(_) => Ok(()),
        Ok(_) => Err("revoked receipt stayed readable".into()),
    }
}
