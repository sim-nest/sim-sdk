//! Owner-issued currentness policy for the installed operation checker.

use sim_conformance_core::{
    CheckedSubjectId, CheckerRevocationDecision, CheckerRevocationHeadId, CheckerRevocationKey,
    CheckerRevocationSet, ConformanceError, PolicyId, RevocationSourceId, RevocationStatus,
};

use crate::{PackSpec, find_pack};

const OWNER_REVOCATION_SOURCE: &str = "sim-sdk/sim-conformance-packs-revocations-v1";

/// Explicit owner decision for one exact `operation/local` subject.
///
/// Omission is not current: a subject absent from the issued snapshot resolves
/// to [`RevocationStatus::Unknown`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationLocalRevocationDecision {
    subject: CheckedSubjectId,
    status: RevocationStatus,
}

impl OperationLocalRevocationDecision {
    /// Records that this exact subject remains current under the owner policy.
    pub fn current(subject: CheckedSubjectId) -> Self {
        Self {
            subject,
            status: RevocationStatus::Current,
        }
    }

    /// Records that this exact subject has been revoked by the owner policy.
    pub fn revoked(subject: CheckedSubjectId) -> Self {
        Self {
            subject,
            status: RevocationStatus::Revoked,
        }
    }
}

/// Returns the SDK-owned revocation source used by every installed pack binding.
pub fn owner_revocation_source() -> Result<RevocationSourceId, ConformanceError> {
    RevocationSourceId::from_text(OWNER_REVOCATION_SOURCE)
}

/// Returns the exact installed `operation/local` policy identity.
pub fn operation_local_policy_id() -> Result<PolicyId, ConformanceError> {
    let code = operation_spec()?.checker_code_id()?;
    PolicyId::from_text(&format!(
        "sim-sdk/operation-local-currentness-v1;checker-code={}",
        render(code.content_id())
    ))
}

/// Issues one immutable owner snapshot for exact `operation/local` subjects.
///
/// `head` is selected by the qualified owner control boundary and must identify
/// the source state from which `decisions` was derived. This function supplies
/// no default-current rule: omitted subjects resolve to `Unknown`.
pub fn issue_operation_local_revocation_set(
    head: CheckerRevocationHeadId,
    decisions: impl IntoIterator<Item = OperationLocalRevocationDecision>,
) -> Result<CheckerRevocationSet, ConformanceError> {
    let spec = operation_spec()?;
    let binding = spec.checker_binding()?;
    let source = owner_revocation_source()?;
    let checker_code = spec.checker_code_id()?;
    let pack = spec.pack_id()?;
    let decisions = decisions
        .into_iter()
        .map(|decision| {
            CheckerRevocationDecision::new(
                CheckerRevocationKey::new(
                    source.clone(),
                    binding.id().clone(),
                    decision.subject,
                    checker_code.clone(),
                    pack.clone(),
                )?,
                decision.status,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    CheckerRevocationSet::from_owner_snapshot(
        binding.owner().clone(),
        source,
        operation_local_policy_id()?,
        head,
        decisions,
    )
}

fn operation_spec() -> Result<&'static PackSpec, ConformanceError> {
    let spec = find_pack("checker/c-op").ok_or(ConformanceError::UnresolvedBinding)?;
    if !spec.implemented_scopes.contains(&"operation/local") {
        return Err(ConformanceError::UnresolvedBinding);
    }
    Ok(spec)
}

fn render(id: &sim_kernel::ContentId) -> String {
    let digest = id
        .bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{}:{digest}", id.algorithm.as_qualified_str())
}
