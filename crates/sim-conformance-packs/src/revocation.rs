//! Owner-issued currentness policy for the installed operation checker.

use sim_conformance_core::{ConformanceError, PolicyId, RevocationSourceId};

use crate::{PackSpec, find_pack};

const OWNER_REVOCATION_SOURCE: &str = "sim-sdk/sim-conformance-packs-revocations-v1";

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

pub(crate) fn operation_spec() -> Result<&'static PackSpec, ConformanceError> {
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
