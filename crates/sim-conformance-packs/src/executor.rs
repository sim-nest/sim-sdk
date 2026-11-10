// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! In-process caller for the frozen roadmap runner.
//!
//! Readiness, effects, and cancellation are injected fakes. Journal state is
//! the runner's memory backend. This module does not re-check the runner's
//! laws before the call.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use sha2::{Digest, Sha256};
use sim_kernel::{ContentId, Symbol};
use sim_lib_journal::MemoryBackend;
use sim_lib_roadmap_runner::{
    AuthorityGrant, CancellationPort, CapabilityGrant, DirectionHandoff, DirectionKind,
    DirectionQuestion, DirectionReply, DirectionRequest, DriveResult, EffectPort, EffectTicket,
    ExecutionIdentity, ExternalLineageRef, InputBatch, Limits, LineageView, OpenRequest, Prepared,
    ReadinessPort, RoadmapRunnerService, RunSnapshot, ServiceError, SnapshotRead,
};
use sim_roadmap_core::PhaseId;
use sim_roadmap_exec_core::{
    AttemptId, ExecutionEvent, ExecutionId, ExecutionPolicy, ExecutionPolicyId, MutationId,
    MutationPlan, Observation, Transition,
};

use crate::authority::{OperationLocalEvidence, OperationLocalSupportDefinition};
use crate::{MemorySubject, PackFailure};

const PROVIDERS: &[&str] = &["model/direction", "model/landing", "model/snapshot"];

type Service = RoadmapRunnerService<MemoryBackend, Ready, CountEffects, Never>;

/// One open execution and the effect-port counter beside it.
pub(crate) struct Drive {
    service: Arc<Mutex<Service>>,
    calls: Arc<AtomicUsize>,
    backend: Arc<MemoryBackend>,
}

impl Drive {
    /// Opens one execution on a fresh journal.
    pub(crate) fn open(request: OpenRequest) -> Result<Self, ServiceError> {
        Self::open_on(
            Arc::new(MemoryBackend::new()),
            request,
            Arc::new(AtomicUsize::new(0)),
        )
    }

    /// Opens one execution on a caller-owned journal.
    pub(crate) fn open_on(
        backend: Arc<MemoryBackend>,
        request: OpenRequest,
        calls: Arc<AtomicUsize>,
    ) -> Result<Self, ServiceError> {
        let admitted = request.phase.clone();
        let service = RoadmapRunnerService::open(
            Arc::clone(&backend),
            Ready(admitted),
            CountEffects {
                calls: Arc::clone(&calls),
            },
            Never,
            request,
            Limits::default(),
        )?;
        Ok(Self {
            service: Arc::new(Mutex::new(service)),
            calls,
            backend,
        })
    }

    pub(crate) fn backend(&self) -> Arc<MemoryBackend> {
        Arc::clone(&self.backend)
    }

    pub(crate) fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    pub(crate) fn conduct(
        &self,
        inputs: &InputBatch,
        grant: &CapabilityGrant,
    ) -> Result<DriveResult, ServiceError> {
        sim_lib_roadmap_runner::conduct_one(&self.service, inputs, grant)
    }

    pub(crate) fn prepare(
        &self,
        inputs: &InputBatch,
        grant: &CapabilityGrant,
    ) -> Result<Prepared, ServiceError> {
        self.lock()?.prepare_effect(inputs, grant)
    }

    pub(crate) fn replay(&self) -> Result<Transition, ServiceError> {
        self.lock()?.replay()
    }

    pub(crate) fn records(&self) -> Result<usize, ServiceError> {
        Ok(self.lock()?.inspect()?.journal.records.len())
    }

    pub(crate) fn revision(&self) -> Result<ContentId, ServiceError> {
        Ok(self.lock()?.revision().clone())
    }

    pub(crate) fn snapshots(&self) -> Result<Vec<RunSnapshot>, ServiceError> {
        Ok(self.lock()?.snapshots().to_vec())
    }

    pub(crate) fn read_snapshot(
        &self,
        snapshot: &ContentId,
        offset: u32,
        limit: u32,
        expected: Option<&ContentId>,
    ) -> Result<SnapshotRead, ServiceError> {
        Ok(self
            .lock()?
            .read_snapshot(snapshot, offset, limit, expected))
    }

    pub(crate) fn lineage(
        &self,
        present: &std::collections::BTreeSet<ContentId>,
    ) -> Result<Vec<LineageView>, ServiceError> {
        Ok(self.lock()?.lineage(present))
    }

    pub(crate) fn candidate(&self) -> Result<ContentId, ServiceError> {
        self.lock()?
            .handoff()
            .map(|item| item.candidate.clone())
            .ok_or_else(|| ServiceError::Adapter("handoff is absent".into()))
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Service>, ServiceError> {
        self.service
            .lock()
            .map_err(|_| ServiceError::Adapter("runner service lock poisoned".into()))
    }
}

pub(crate) fn require_subject(subject: &str) -> Result<(), PackFailure> {
    if subject.is_empty() || subject.contains('\0') || subject.contains('\n') {
        return Err(PackFailure {
            code: "missing-evidence",
            detail: "subject".into(),
        });
    }
    Ok(())
}

/// Refuses evidence that is not the executor's qualified support receipt.
///
/// The live checker owner issues that receipt after the support proof passes.
/// A fact-only copy has no receipt. A receipt issued for another subject does
/// not satisfy this subject.
pub(crate) fn require_support(request: &crate::PackRequest<'_>) -> Result<(), PackFailure> {
    let definition = support_definition(request.subject)?;
    let Some(receipt) = request.evidence.support_qualification() else {
        return Err(unqualified());
    };
    crate::executor_support::require_current(
        request.subject,
        &definition,
        request.evidence,
        receipt,
    )?;
    let evidence = OperationLocalEvidence::capture(request.evidence).map_err(|_| unqualified())?;
    definition
        .verify_identity_evidence(&evidence)
        .map_err(|_| unqualified())?;
    Ok(())
}

/// Facts and the qualified receipt that [`require_support`] accepts for `subject`.
pub fn executor_support_evidence(subject: &str) -> Result<MemorySubject, PackFailure> {
    let definition = support_definition(subject)?;
    let mut evidence = MemorySubject::default();
    for (name, identity) in definition.members() {
        evidence = evidence.with(name.clone(), render_id(identity));
    }
    Ok(evidence.with_qualification(crate::executor_support::issue(subject, &definition)?))
}

/// Foreign-project scope against the frozen executor and the source provider.
pub(crate) fn foreign_project(subject: &str) -> Result<Vec<crate::CheckObservation>, PackFailure> {
    let observations = crate::source_invoke::admission(subject)?;
    let owner = observations
        .iter()
        .find(|item| item.key == "source.subject")
        .map(|item| item.value.as_str());
    if owner != Some(subject) {
        return Err(unqualified());
    }
    Ok(observations)
}

pub(crate) fn support_definition(
    subject: &str,
) -> Result<OperationLocalSupportDefinition, PackFailure> {
    let drive = Drive::open(with_direction(subject)).map_err(|error| PackFailure {
        code: "unqualified-support",
        detail: error.to_string(),
    })?;
    let revision = drive.revision().map_err(|error| PackFailure {
        code: "unqualified-support",
        detail: error.to_string(),
    })?;
    let subject_id = ContentId::from_bytes(
        Symbol::qualified("core", "sha256-datum-v1"),
        Sha256::digest(subject.as_bytes()).into(),
    );
    OperationLocalSupportDefinition::new([
        ("executor.closure".to_owned(), runner_id()),
        ("executor.revision".to_owned(), revision),
        ("executor.subject".to_owned(), subject_id),
    ])
    .map_err(|_| unqualified())
}

pub(crate) fn unqualified() -> PackFailure {
    PackFailure {
        code: "unqualified-support",
        detail: "evidence lacks the executor support record for this subject".into(),
    }
}

pub(crate) fn render_id(id: &ContentId) -> String {
    let digest = id
        .bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{}:{digest}", id.algorithm.as_qualified_str())
}

pub(crate) fn cid(n: u8) -> ContentId {
    ContentId::from_bytes(Symbol::qualified("core", "sha256-datum-v1"), [n; 32])
}

pub(crate) fn runner_id() -> ContentId {
    let hex = crate::compiled_executor_closure();
    assert_eq!(hex.len(), 64, "executor closure digest");
    let mut bytes = [0u8; 32];
    for (index, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let text = std::str::from_utf8(chunk).expect("executor closure hex");
        bytes[index] = u8::from_str_radix(text, 16).expect("executor closure hex");
    }
    ContentId::from_bytes(Symbol::qualified("core", "sha256-datum-v1"), bytes)
}

pub(crate) fn hex_id(id: &ContentId) -> String {
    id.bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn provider_names() -> &'static [&'static str] {
    PROVIDERS
}

pub(crate) fn grant(name: &str) -> CapabilityGrant {
    CapabilityGrant {
        capabilities: vec![Symbol::new(name)],
    }
}

pub(crate) fn base_request() -> OpenRequest {
    let policy = ExecutionPolicy {
        id: ExecutionPolicyId::new("policy").expect("policy id"),
        source_deck: cid(2),
        required_promises: vec![],
        required_proofs: vec![],
    };
    OpenRequest {
        authority: AuthorityGrant {
            identity: ExecutionIdentity {
                execution: ExecutionId::new("execution").expect("execution id"),
                policy: policy.id.clone(),
                roadmap: cid(1),
                source_deck: policy.source_deck.clone(),
                conduct: cid(3),
                model: cid(4),
                launcher: cid(5),
                runner: runner_id(),
            },
            ceiling: sim_lib_roadmap_runner::EffectiveCeiling::intersect([
                sim_lib_roadmap_runner::OwnedLimit {
                    owner: Symbol::new("caller"),
                    unit: Symbol::new("turns"),
                    amount: 4,
                },
            ]),
            grant: cid(7),
        },
        phase: PhaseId::new("leaf").expect("phase id"),
        attempt: AttemptId::new("attempt").expect("attempt id"),
        policy,
        mutation: MutationPlan::new(
            MutationId::new("mutation").expect("mutation id"),
            vec![],
            vec![],
        )
        .expect("mutation plan"),
        resources: vec![],
        direction: None,
        handoff: None,
    }
}

pub(crate) fn with_direction(subject: &str) -> OpenRequest {
    let mut request = base_request();
    request.direction = Some(DirectionQuestion {
        id: cid(30),
        question: subject.to_owned(),
        answer_kind: DirectionKind::Decision,
        required_authority: Some(Symbol::new("decide")),
    });
    request
}

pub(crate) fn with_handoff(begin: Option<ContentId>) -> OpenRequest {
    let mut request = base_request();
    request.handoff = Some(DirectionHandoff {
        exact_source: cid(10),
        candidate: cid(11),
        selection: cid(15),
        begin,
        external_lineage: vec![lineage_ref(cid(13)), lineage_ref(cid(14))],
    });
    request
}

pub(crate) fn with_candidate(candidate: ContentId) -> OpenRequest {
    let mut request = with_handoff(Some(cid(12)));
    if let Some(handoff) = request.handoff.as_mut() {
        handoff.candidate = candidate;
    }
    request
}

pub(crate) fn decision_reply(request: &DirectionRequest, answer: &str) -> DirectionReply {
    DirectionReply {
        request: request.id.clone(),
        execution: request.execution.clone(),
        expected_revision: request.expected_revision.clone(),
        answer_kind: DirectionKind::Decision,
        answer: answer.to_owned(),
        decision: cid(40),
    }
}

pub(crate) fn advice_reply(request: &DirectionRequest) -> DirectionReply {
    DirectionReply {
        request: request.id.clone(),
        execution: request.execution.clone(),
        expected_revision: request.expected_revision.clone(),
        answer_kind: DirectionKind::Advice,
        answer: "note".into(),
        decision: cid(40),
    }
}

fn lineage_ref(value: ContentId) -> ExternalLineageRef {
    ExternalLineageRef {
        relation: Symbol::new("relation"),
        owner: Symbol::new("owner"),
        shape: Symbol::new("shape"),
        value,
    }
}

struct Ready(PhaseId);

impl ReadinessPort for Ready {
    fn admitted_leaf(&self, _: &ContentId) -> Result<Option<PhaseId>, ServiceError> {
        Ok(Some(self.0.clone()))
    }
}

struct CountEffects {
    calls: Arc<AtomicUsize>,
}

impl EffectPort for CountEffects {
    fn invoke(
        &self,
        identity: &ExecutionIdentity,
        phase: &PhaseId,
        _: &Transition,
        _: Option<&Symbol>,
    ) -> Result<ExecutionEvent, ServiceError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(ExecutionEvent {
            execution: identity.execution.clone(),
            phase: phase.clone(),
            attempt: AttemptId::new("attempt").expect("attempt id"),
            observation: Observation::new(Symbol::new("start"), cid(20)),
        })
    }

    fn receipts(&self, _: &ExecutionIdentity) -> Result<Vec<ExecutionEvent>, ServiceError> {
        Ok(vec![])
    }
}

struct Never;

impl CancellationPort for Never {
    fn cancellation_requested(&self, _: &ExecutionId) -> bool {
        false
    }
}

/// Effect admission returned by [`Drive::prepare`]. The port has not run.
pub(crate) fn admitted(prepared: Prepared) -> Option<EffectTicket> {
    match prepared {
        Prepared::Ready(ticket) => Some(*ticket),
        Prepared::Done(_) => None,
    }
}

#[cfg(test)]
mod subject_tests {
    use super::require_subject;

    #[test]
    fn subject_refuses_empty_nul_and_newline() {
        assert_eq!(require_subject("").unwrap_err().code, "missing-evidence");
        assert_eq!(
            require_subject("a\0b").unwrap_err().code,
            "missing-evidence"
        );
        assert_eq!(
            require_subject("a\nb").unwrap_err().code,
            "missing-evidence"
        );
        assert!(require_subject("subject/seam").is_ok());
    }
}
