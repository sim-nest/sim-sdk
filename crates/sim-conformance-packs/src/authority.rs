//! Boot-owned authority for `checker/c-op` `operation/local` qualification.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, Weak};

use sim_conformance_core::{
    CheckInputClosureId, CheckScopeId, CheckedSubjectId, CheckerBinding, CheckerReceipt,
    CheckerRevocationDecision, CheckerRevocationHeadId, CheckerRevocationKey,
    CheckerRevocationKeyId, CheckerRevocationObservation, CheckerRevocationSelection,
    CheckerRevocationSet, ConformanceError, EvidenceGrade, EvidenceProvenanceId, EvidenceSetId,
    RevocationStatus,
};
use sim_kernel::{ContentId, Datum, Symbol};

use crate::revocation::operation_spec;
use crate::{
    CheckObservation, PackFailure, PackRequest, PackSubject, PackVerdict,
    operation_local_policy_id, owner_revocation_source, packs,
};

/// Maximum byte length of one canonical `operation/local` evidence value.
pub const MAX_OPERATION_LOCAL_EVIDENCE_BYTES: usize = 16_384;
/// Maximum number of facts in one canonical `operation/local` evidence value.
pub const MAX_OPERATION_LOCAL_EVIDENCE_FACTS: usize = 256;

/// Refusal while freezing one exact `operation/local` evidence value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperationLocalEvidenceError {
    /// The canonical byte representation exceeds its fixed resource ceiling.
    ByteBound,
    /// The fact count exceeds its fixed resource ceiling.
    FactBound,
    /// The input is not in the one canonical lexical form.
    Malformed(String),
}

impl fmt::Display for OperationLocalEvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ByteBound => formatter.write_str("evidence exceeds 16384 bytes"),
            Self::FactBound => formatter.write_str("evidence exceeds 256 facts"),
            Self::Malformed(detail) => detail.fmt(formatter),
        }
    }
}

impl std::error::Error for OperationLocalEvidenceError {}

/// One owned, bounded, unambiguous snapshot of `operation/local` facts.
///
/// Construction reads a [`PackSubject`] exactly once. Identity derivation and
/// checker execution subsequently use only this sealed map, so a stateful
/// subject cannot change between those operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationLocalEvidence {
    facts: BTreeMap<String, String>,
    canonical: String,
}

impl OperationLocalEvidence {
    /// Parses the strict canonical P3 text representation.
    pub fn parse(bytes: &[u8]) -> Result<Self, OperationLocalEvidenceError> {
        if bytes.len() > MAX_OPERATION_LOCAL_EVIDENCE_BYTES {
            return Err(OperationLocalEvidenceError::ByteBound);
        }
        let text = std::str::from_utf8(bytes)
            .map_err(|_| OperationLocalEvidenceError::Malformed("evidence is not UTF-8".into()))?;
        if text.is_empty() {
            return Err(OperationLocalEvidenceError::Malformed(
                "empty evidence".into(),
            ));
        }
        let body = text.strip_suffix('\n').ok_or_else(|| {
            OperationLocalEvidenceError::Malformed(
                "canonical evidence must end with a newline".into(),
            )
        })?;
        if body.is_empty() {
            return Err(OperationLocalEvidenceError::Malformed(
                "empty evidence".into(),
            ));
        }
        let mut facts = BTreeMap::new();
        let mut previous: Option<&str> = None;
        for (index, line) in body.split('\n').enumerate() {
            if index >= MAX_OPERATION_LOCAL_EVIDENCE_FACTS {
                return Err(OperationLocalEvidenceError::FactBound);
            }
            let (key, value) = line.split_once('=').ok_or_else(|| {
                OperationLocalEvidenceError::Malformed(format!("fact {index} has no `=`"))
            })?;
            validate_fact(index, key, value)?;
            if previous.is_some_and(|seen| seen >= key) {
                return Err(OperationLocalEvidenceError::Malformed(format!(
                    "fact keys are duplicate or unsorted at `{key}`"
                )));
            }
            facts.insert(key.to_owned(), value.to_owned());
            previous = Some(key);
        }
        Ok(Self {
            facts,
            canonical: text.to_owned(),
        })
    }

    /// Reads an arbitrary subject once and freezes it under the exact P3 rules.
    pub fn capture(subject: &dyn PackSubject) -> Result<Self, OperationLocalEvidenceError> {
        let keys = subject.fact_keys();
        if keys.is_empty() {
            return Err(OperationLocalEvidenceError::Malformed(
                "empty evidence".into(),
            ));
        }
        if keys.len() > MAX_OPERATION_LOCAL_EVIDENCE_FACTS {
            return Err(OperationLocalEvidenceError::FactBound);
        }
        let mut facts = BTreeMap::new();
        let mut canonical = String::new();
        let mut previous: Option<&str> = None;
        for (index, key) in keys.into_iter().enumerate() {
            if previous.is_some_and(|seen| seen >= key) {
                return Err(OperationLocalEvidenceError::Malformed(format!(
                    "fact keys are duplicate or unsorted at `{key}`"
                )));
            }
            let value = subject.fact(key).ok_or_else(|| {
                OperationLocalEvidenceError::Malformed(format!(
                    "fact {index} disappeared while being captured"
                ))
            })?;
            validate_fact(index, key, value)?;
            facts.insert(key.to_owned(), value.to_owned());
            canonical.push_str(key);
            canonical.push('=');
            canonical.push_str(value);
            canonical.push('\n');
            if canonical.len() > MAX_OPERATION_LOCAL_EVIDENCE_BYTES {
                return Err(OperationLocalEvidenceError::ByteBound);
            }
            previous = Some(key);
        }
        Ok(Self { facts, canonical })
    }

    /// Returns the unique canonical bytes bound into P3 identities.
    pub fn canonical(&self) -> &str {
        &self.canonical
    }

    /// Derives the exact checked-subject identity of this frozen value.
    pub fn subject_id(&self) -> Result<CheckedSubjectId, ConformanceError> {
        CheckedSubjectId::from_fields(vec![(
            Symbol::qualified("conformance", "evidence"),
            Datum::String(self.canonical.clone()),
        )])
    }

    /// Derives the exact input-closure identity of this frozen value.
    pub fn input_closure_id(&self) -> Result<CheckInputClosureId, ConformanceError> {
        CheckInputClosureId::from_fields(vec![(
            Symbol::qualified("conformance", "evidence"),
            Datum::String(self.canonical.clone()),
        )])
    }
}

impl PackSubject for OperationLocalEvidence {
    fn fact(&self, key: &str) -> Option<&str> {
        self.facts.get(key).map(String::as_str)
    }

    fn fact_keys(&self) -> Vec<&str> {
        self.facts.keys().map(String::as_str).collect()
    }
}

fn validate_fact(index: usize, key: &str, value: &str) -> Result<(), OperationLocalEvidenceError> {
    if key.is_empty()
        || value.is_empty()
        || key.len() > 256
        || value.len() > 4_096
        || key.contains('=')
        || key
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
        || value.chars().any(char::is_control)
    {
        return Err(OperationLocalEvidenceError::Malformed(format!(
            "fact {index} is empty, unbounded, ambiguous, or contains control text"
        )));
    }
    Ok(())
}

/// Failure while issuing or observing owner-qualified evidence.
#[derive(Debug)]
pub enum OperationLocalAuthorityError {
    /// A canonical conformance identity or join was invalid.
    Conformance(ConformanceError),
    /// The installed checker refused the supplied evidence.
    Refused(PackFailure),
    /// The requested checker/scope is not the owner implemented here.
    Unsupported,
    /// The boot owner has stopped or its state cannot be read safely.
    Unavailable,
    /// The qualified artifact belongs to another boot generation.
    ForeignAuthority,
}

impl fmt::Display for OperationLocalAuthorityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Conformance(error) => error.fmt(formatter),
            Self::Refused(failure) => write!(formatter, "{}: {}", failure.code, failure.detail),
            Self::Unsupported => formatter.write_str("unsupported checker or scope"),
            Self::Unavailable => formatter.write_str("operation/local boot owner is not live"),
            Self::ForeignAuthority => {
                formatter.write_str("qualified receipt belongs to another boot generation")
            }
        }
    }
}

impl std::error::Error for OperationLocalAuthorityError {}

impl From<ConformanceError> for OperationLocalAuthorityError {
    fn from(error: ConformanceError) -> Self {
        Self::Conformance(error)
    }
}

#[derive(Debug)]
struct OwnerState {
    generation: ContentId,
    revision: u64,
    decisions: BTreeMap<CheckerRevocationKeyId, (CheckerRevocationKey, RevocationStatus)>,
}

/// The boot-selected provider's sole strong owner of operation/local state.
///
/// `generation` is a durable content identity selected by provider boot, not a
/// process-local counter. Dropping this value invalidates all issued handles.
#[derive(Debug)]
pub struct OperationLocalOwner {
    state: Arc<Mutex<OwnerState>>,
}

/// Narrow, read/execute-only handle issued by the live boot owner.
///
/// Its weak reference cannot keep the provider alive.
#[derive(Clone, Debug)]
pub struct OperationLocalAuthority {
    state: Weak<Mutex<OwnerState>>,
    generation: ContentId,
}

/// Opaque result of the exact operation/local checker join.
///
/// Neutral receipts cannot be promoted to this type, and its weak binding
/// cannot keep a dead provider current.
#[derive(Clone, Debug)]
pub struct QualifiedOperationLocalCheck {
    authority: Weak<Mutex<OwnerState>>,
    generation: ContentId,
    binding: CheckerBinding,
    invocation: sim_conformance_core::CheckInvocation,
    selection: CheckerRevocationSelection,
    receipt: CheckerReceipt,
    observation: CheckerRevocationObservation,
    observations: Vec<CheckObservation>,
}

impl OperationLocalOwner {
    /// Starts one provider-owned generation from its durable boot identity.
    pub fn boot(generation: ContentId) -> Self {
        Self {
            state: Arc::new(Mutex::new(OwnerState {
                generation,
                revision: 0,
                decisions: BTreeMap::new(),
            })),
        }
    }

    /// Issues a weak, operation-only handle for an admitted consumer.
    pub fn operation_handle(&self) -> OperationLocalAuthority {
        let generation = self
            .state
            .lock()
            .map(|state| state.generation.clone())
            .unwrap_or_else(|poisoned| poisoned.into_inner().generation.clone());
        OperationLocalAuthority {
            state: Arc::downgrade(&self.state),
            generation,
        }
    }

    /// Selects `Current` for one exact checked subject.
    pub fn mark_current(
        &self,
        subject: CheckedSubjectId,
    ) -> Result<(), OperationLocalAuthorityError> {
        self.record(subject, RevocationStatus::Current)
    }

    /// Selects `Revoked` for one exact checked subject.
    pub fn revoke(&self, subject: CheckedSubjectId) -> Result<(), OperationLocalAuthorityError> {
        self.record(subject, RevocationStatus::Revoked)
    }

    fn record(
        &self,
        subject: CheckedSubjectId,
        status: RevocationStatus,
    ) -> Result<(), OperationLocalAuthorityError> {
        let key = operation_key(subject)?;
        let mut state = lock_arc(&self.state)?;
        state.revision = state
            .revision
            .checked_add(1)
            .ok_or(OperationLocalAuthorityError::Unavailable)?;
        state.decisions.insert(key.id().clone(), (key, status));
        Ok(())
    }
}

impl OperationLocalAuthority {
    /// Returns the durable boot-generation identity bound to this handle.
    pub const fn generation(&self) -> &ContentId {
        &self.generation
    }

    /// Confirms that the exact provider generation which issued this handle is live.
    pub fn ensure_live(&self) -> Result<(), OperationLocalAuthorityError> {
        let strong = self
            .state
            .upgrade()
            .ok_or(OperationLocalAuthorityError::Unavailable)?;
        let state = lock_arc(&strong)?;
        if state.generation == self.generation {
            Ok(())
        } else {
            Err(OperationLocalAuthorityError::ForeignAuthority)
        }
    }

    /// Runs the checker and issues an opaque, live-bound exact-input artifact.
    #[allow(clippy::too_many_arguments)]
    pub fn check_and_issue(
        &self,
        binding: CheckerBinding,
        invocation: sim_conformance_core::CheckInvocation,
        evidence: &OperationLocalEvidence,
        grade: EvidenceGrade,
        provenance: EvidenceProvenanceId,
        support: EvidenceSetId,
    ) -> Result<QualifiedOperationLocalCheck, OperationLocalAuthorityError> {
        verify_operation_invocation(&binding, &invocation, evidence)?;
        let strong = self
            .state
            .upgrade()
            .ok_or(OperationLocalAuthorityError::Unavailable)?;
        let state = lock_arc(&strong)?;
        if state.generation != self.generation {
            return Err(OperationLocalAuthorityError::ForeignAuthority);
        }
        let set = snapshot(&state)?;
        let key = CheckerRevocationKey::for_invocation(owner_revocation_source()?, &invocation)?;
        let selection = set.lookup(&key);
        let binding_text = render(binding.id().content_id());
        let subject_text = render(invocation.subject().content_id());
        let request = PackRequest {
            checker: "checker/c-op",
            binding: &binding_text,
            subject: &subject_text,
            scope: "operation/local",
            evidence,
        };
        let (result, observations) = match packs::operation::check(&request) {
            PackVerdict::Pass {
                result,
                observations,
            } => (result, observations),
            PackVerdict::Refused(failure) => {
                return Err(OperationLocalAuthorityError::Refused(failure));
            }
            PackVerdict::UnimplementedPack { .. } => {
                return Err(OperationLocalAuthorityError::Unsupported);
            }
        };
        let receipt = CheckerReceipt::passing(
            &binding,
            &invocation,
            result,
            grade,
            provenance,
            operation_local_policy_id()?,
            support,
            &selection,
        )?;
        let observation = selection.bind_receipt(receipt.id().clone());
        receipt.verify(&binding, &invocation, &observation)?;
        Ok(QualifiedOperationLocalCheck {
            authority: Arc::downgrade(&strong),
            generation: self.generation.clone(),
            binding,
            invocation,
            selection,
            receipt,
            observation,
            observations,
        })
    }

    /// Resolves an artifact against the exact live boot owner.
    pub fn observe_receipt(
        &self,
        qualified: &QualifiedOperationLocalCheck,
    ) -> Result<CheckerRevocationObservation, OperationLocalAuthorityError> {
        if self.generation != qualified.generation
            || !Weak::ptr_eq(&self.state, &qualified.authority)
        {
            return Err(OperationLocalAuthorityError::ForeignAuthority);
        }
        let strong = self
            .state
            .upgrade()
            .ok_or(OperationLocalAuthorityError::Unavailable)?;
        let state = lock_arc(&strong)?;
        let set = snapshot(&state)?;
        Ok(set
            .lookup(qualified.selection.key())
            .bind_receipt(qualified.receipt.id().clone()))
    }
}

impl QualifiedOperationLocalCheck {
    /// Returns the exact installed checker binding.
    pub const fn binding(&self) -> &CheckerBinding {
        &self.binding
    }
    /// Returns the exact checker invocation.
    pub const fn invocation(&self) -> &sim_conformance_core::CheckInvocation {
        &self.invocation
    }
    /// Returns the issue-time owner selection.
    pub const fn issue_selection(&self) -> &CheckerRevocationSelection {
        &self.selection
    }
    /// Returns the exact checker receipt.
    pub const fn receipt(&self) -> &CheckerReceipt {
        &self.receipt
    }
    /// Returns the receipt-bound issue observation.
    pub const fn issue_observation(&self) -> &CheckerRevocationObservation {
        &self.observation
    }
    /// Returns deterministic observations covered by the result.
    pub fn observations(&self) -> &[CheckObservation] {
        &self.observations
    }

    /// Verifies issue coherence and fresh live-owner currentness.
    pub fn verify_current(
        &self,
        authority: &OperationLocalAuthority,
    ) -> Result<CheckerRevocationObservation, OperationLocalAuthorityError> {
        if self.selection.bind_receipt(self.receipt.id().clone()) != self.observation {
            return Err(ConformanceError::InvocationMismatch("issue observation").into());
        }
        self.receipt
            .verify(&self.binding, &self.invocation, &self.observation)?;
        let current = authority.observe_receipt(self)?;
        self.receipt
            .verify(&self.binding, &self.invocation, &current)?;
        Ok(current)
    }
}

fn operation_key(subject: CheckedSubjectId) -> Result<CheckerRevocationKey, ConformanceError> {
    let spec = operation_spec()?;
    let binding = spec.checker_binding()?;
    CheckerRevocationKey::new(
        owner_revocation_source()?,
        binding.id().clone(),
        subject,
        spec.checker_code_id()?,
        spec.pack_id()?,
    )
}

fn verify_operation_invocation(
    binding: &CheckerBinding,
    invocation: &sim_conformance_core::CheckInvocation,
    evidence: &OperationLocalEvidence,
) -> Result<(), OperationLocalAuthorityError> {
    let spec = operation_spec()?;
    let expected = spec.checker_binding()?;
    let scope = CheckScopeId::from_text("operation/local")?;
    let subject = evidence.subject_id()?;
    let closure = evidence.input_closure_id()?;
    if binding != &expected
        || invocation.binding() != binding.id()
        || invocation.checker_code() != &spec.checker_code_id()?
        || invocation.pack() != &spec.pack_id()?
        || invocation.scope() != &scope
        || invocation.subject() != &subject
        || invocation.input_closure() != &closure
    {
        return Err(OperationLocalAuthorityError::Unsupported);
    }
    Ok(())
}

fn snapshot(state: &OwnerState) -> Result<CheckerRevocationSet, OperationLocalAuthorityError> {
    let head = CheckerRevocationHeadId::from_fields(vec![
        (
            Symbol::qualified("conformance", "owner-generation"),
            Datum::Node {
                tag: Symbol::qualified("conformance", "content-id"),
                fields: vec![
                    (
                        Symbol::new("algorithm"),
                        Datum::Symbol(state.generation.algorithm.clone()),
                    ),
                    (
                        Symbol::new("digest"),
                        Datum::Bytes(state.generation.bytes.to_vec()),
                    ),
                ],
            },
        ),
        (
            Symbol::qualified("conformance", "owner-revision"),
            Datum::String(state.revision.to_string()),
        ),
    ])?;
    let decisions = state
        .decisions
        .values()
        .map(|(key, status)| CheckerRevocationDecision::new(key.clone(), *status))
        .collect::<Result<Vec<_>, _>>()?;
    let binding = operation_spec()?.checker_binding()?;
    Ok(CheckerRevocationSet::from_owner_snapshot(
        binding.owner().clone(),
        owner_revocation_source()?,
        operation_local_policy_id()?,
        head,
        decisions,
    )?)
}

fn lock_arc(
    state: &Arc<Mutex<OwnerState>>,
) -> Result<MutexGuard<'_, OwnerState>, OperationLocalAuthorityError> {
    state
        .lock()
        .map_err(|_| OperationLocalAuthorityError::Unavailable)
}

fn render(id: &ContentId) -> String {
    let digest = id
        .bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{}:{digest}", id.algorithm.as_qualified_str())
}
