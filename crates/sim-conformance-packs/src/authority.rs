//! Host-bound authority for `checker/c-op` `operation/local` qualification.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use sim_conformance_core::{
    CheckInvocation, CheckScopeId, CheckedSubjectId, CheckerBinding, CheckerReceipt,
    CheckerRevocationDecision, CheckerRevocationHeadId, CheckerRevocationKey,
    CheckerRevocationKeyId, CheckerRevocationObservation, CheckerRevocationSelection,
    CheckerRevocationSet, ConformanceError, EvidenceGrade, EvidenceProvenanceId, EvidenceSetId,
    RevocationStatus,
};
use sim_kernel::{CapabilityName, Cx, GrantSeat};

use crate::revocation::operation_spec;
use crate::{
    CheckObservation, PackFailure, PackRequest, PackSubject, PackVerdict,
    operation_local_policy_id, owner_revocation_source, packs,
};

const OWNER_BIND_CAPABILITY: &str = "conformance/operation-local-owner";
static NEXT_AUTHORITY: AtomicU64 = AtomicU64::new(1);

/// Failure while binding, issuing, or observing owner-qualified evidence.
#[derive(Debug)]
pub enum OperationLocalAuthorityError {
    /// The host grant seat was foreign or the owner capability was absent.
    Kernel(sim_kernel::Error),
    /// A canonical conformance identity or join was invalid.
    Conformance(ConformanceError),
    /// The installed checker refused the supplied evidence.
    Refused(PackFailure),
    /// The requested checker/scope is not the owner implemented here.
    Unsupported,
    /// The authority state could not be read without ambiguity.
    Unavailable,
    /// The qualified artifact belongs to a different owner instance.
    ForeignAuthority,
}

impl fmt::Display for OperationLocalAuthorityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Kernel(error) => error.fmt(formatter),
            Self::Conformance(error) => error.fmt(formatter),
            Self::Refused(failure) => write!(formatter, "{}: {}", failure.code, failure.detail),
            Self::Unsupported => formatter.write_str("unsupported checker or scope"),
            Self::Unavailable => formatter.write_str("operation/local owner state unavailable"),
            Self::ForeignAuthority => {
                formatter.write_str("qualified receipt belongs to another owner authority")
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

impl From<sim_kernel::Error> for OperationLocalAuthorityError {
    fn from(error: sim_kernel::Error) -> Self {
        Self::Kernel(error)
    }
}

#[derive(Debug)]
struct OwnerState {
    instance: u64,
    revision: u64,
    decisions: BTreeMap<CheckerRevocationKeyId, (CheckerRevocationKey, RevocationStatus)>,
}

/// Read and qualification side of the SDK owner's live currentness boundary.
///
/// This handle has no neutral-data constructor. A host must bind it through
/// [`bind_operation_local_owner`] while holding a kernel [`GrantSeat`]. Loaded
/// callables receive a [`Cx`] but never its seat, so they cannot mint an owner
/// or turn a neutral `Current` record into authority.
#[derive(Clone, Debug)]
pub struct OperationLocalAuthority {
    state: Arc<Mutex<OwnerState>>,
}

/// Mutation side of one host-bound SDK owner instance.
///
/// The control is deliberately separate from [`OperationLocalAuthority`]. Give
/// the read handle to checker consumers; retain this control at the selected
/// boot provider that owns current/revoked decisions.
#[derive(Debug)]
pub struct OperationLocalOwnerControl {
    state: Arc<Mutex<OwnerState>>,
}

/// Opaque result of an actual SDK-owned `operation/local` check.
///
/// Its fields and constructor are private. Neutral receipt/revocation records
/// remain useful canonical data, but cannot be promoted to this authority-bearing
/// artifact by a caller.
#[derive(Clone, Debug)]
pub struct QualifiedOperationLocalReceipt {
    authority: Arc<Mutex<OwnerState>>,
    binding: CheckerBinding,
    invocation: CheckInvocation,
    selection: CheckerRevocationSelection,
    receipt: CheckerReceipt,
    observation: CheckerRevocationObservation,
    observations: Vec<CheckObservation>,
}

/// Binds one operation/local owner to a host-controlled boot context.
///
/// The grant seat is checked against `cx`, then the private owner capability is
/// seated in that context. A fresh context/seat pair creates a distinct owner;
/// artifacts from one owner instance are never accepted by another.
pub fn bind_operation_local_owner(
    cx: &mut Cx,
    seat: &GrantSeat,
) -> Result<(OperationLocalAuthority, OperationLocalOwnerControl), OperationLocalAuthorityError> {
    let capability = CapabilityName::new(OWNER_BIND_CAPABILITY);
    seat.grant(cx, capability.clone())?;
    cx.require(&capability)?;
    let instance = NEXT_AUTHORITY
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .map_err(|_| OperationLocalAuthorityError::Unavailable)?;
    let state = Arc::new(Mutex::new(OwnerState {
        instance,
        revision: 0,
        decisions: BTreeMap::new(),
    }));
    Ok((
        OperationLocalAuthority {
            state: Arc::clone(&state),
        },
        OperationLocalOwnerControl { state },
    ))
}

impl OperationLocalOwnerControl {
    /// Selects `Current` for one exact subject under the installed implementation.
    pub fn mark_current(
        &mut self,
        subject: CheckedSubjectId,
    ) -> Result<(), OperationLocalAuthorityError> {
        self.record(subject, RevocationStatus::Current)
    }

    /// Selects `Revoked` for one exact subject under the installed implementation.
    pub fn revoke(
        &mut self,
        subject: CheckedSubjectId,
    ) -> Result<(), OperationLocalAuthorityError> {
        self.record(subject, RevocationStatus::Revoked)
    }

    fn record(
        &mut self,
        subject: CheckedSubjectId,
        status: RevocationStatus,
    ) -> Result<(), OperationLocalAuthorityError> {
        let key = operation_key(subject)?;
        let mut state = lock(&self.state)?;
        state.revision = state
            .revision
            .checked_add(1)
            .ok_or(OperationLocalAuthorityError::Unavailable)?;
        state.decisions.insert(key.id().clone(), (key, status));
        Ok(())
    }
}

impl OperationLocalAuthority {
    /// Runs the actual installed checker and issues an opaque qualified receipt.
    #[allow(clippy::too_many_arguments)]
    pub fn check_and_issue(
        &self,
        binding: CheckerBinding,
        invocation: CheckInvocation,
        evidence: &dyn PackSubject,
        grade: EvidenceGrade,
        provenance: EvidenceProvenanceId,
        support: EvidenceSetId,
    ) -> Result<QualifiedOperationLocalReceipt, OperationLocalAuthorityError> {
        verify_operation_invocation(&binding, &invocation)?;
        let state = lock(&self.state)?;
        let set = snapshot(&state)?;
        let key = CheckerRevocationKey::for_invocation(owner_revocation_source()?, &invocation)?;
        let selection = set.lookup(&key);
        let checker = "checker/c-op";
        let binding_text = render(binding.id().content_id());
        let subject_text = render(invocation.subject().content_id());
        let scope = "operation/local";
        let request = PackRequest {
            checker,
            binding: &binding_text,
            subject: &subject_text,
            scope,
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
        Ok(QualifiedOperationLocalReceipt {
            authority: Arc::clone(&self.state),
            binding,
            invocation,
            selection,
            receipt,
            observation,
            observations,
        })
    }

    /// Resolves a qualified receipt against the owner's current live state.
    ///
    /// No snapshot or head is accepted from the caller. The lock covers head
    /// selection and exact-key lookup, so a self-consistent older pair cannot
    /// be replayed after the owner advances.
    pub fn observe_receipt(
        &self,
        qualified: &QualifiedOperationLocalReceipt,
    ) -> Result<CheckerRevocationObservation, OperationLocalAuthorityError> {
        if !Arc::ptr_eq(&self.state, &qualified.authority) {
            return Err(OperationLocalAuthorityError::ForeignAuthority);
        }
        let state = lock(&self.state)?;
        let set = snapshot(&state)?;
        let selection = set.lookup(qualified.selection.key());
        Ok(selection.bind_receipt(qualified.receipt.id().clone()))
    }
}

impl QualifiedOperationLocalReceipt {
    /// Returns the exact installed checker binding.
    pub const fn binding(&self) -> &CheckerBinding {
        &self.binding
    }

    /// Returns the exact checker invocation.
    pub const fn invocation(&self) -> &CheckInvocation {
        &self.invocation
    }

    /// Returns the owner selection observed at issue time.
    pub const fn issue_selection(&self) -> &CheckerRevocationSelection {
        &self.selection
    }

    /// Returns the exact checker receipt.
    pub const fn receipt(&self) -> &CheckerReceipt {
        &self.receipt
    }

    /// Returns the receipt-bound owner observation made at issue time.
    pub const fn issue_observation(&self) -> &CheckerRevocationObservation {
        &self.observation
    }

    /// Returns the deterministic observations covered by the result.
    pub fn observations(&self) -> &[CheckObservation] {
        &self.observations
    }

    /// Verifies issue-time coherence and fresh owner currentness.
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
    invocation: &CheckInvocation,
) -> Result<(), OperationLocalAuthorityError> {
    let spec = operation_spec()?;
    let expected = spec.checker_binding()?;
    let scope = CheckScopeId::from_text("operation/local")?;
    if binding != &expected
        || invocation.binding() != binding.id()
        || invocation.checker_code() != &spec.checker_code_id()?
        || invocation.pack() != &spec.pack_id()?
        || invocation.scope() != &scope
    {
        return Err(OperationLocalAuthorityError::Unsupported);
    }
    Ok(())
}

fn snapshot(state: &OwnerState) -> Result<CheckerRevocationSet, OperationLocalAuthorityError> {
    let head = CheckerRevocationHeadId::from_text(&format!(
        "sim-sdk/operation-local-owner/{}/revision/{}",
        state.instance, state.revision
    ))?;
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

fn lock(
    state: &Arc<Mutex<OwnerState>>,
) -> Result<MutexGuard<'_, OwnerState>, OperationLocalAuthorityError> {
    state
        .lock()
        .map_err(|_| OperationLocalAuthorityError::Unavailable)
}

fn render(id: &sim_kernel::ContentId) -> String {
    let digest = id
        .bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{}:{digest}", id.algorithm.as_qualified_str())
}
