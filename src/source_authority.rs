//! Canonical host-built source admission contracts.
//!
//! This module presents the shared runtime owner directly. Build one
//! [`SourceAuthority`] in trusted host code, then pass it to
//! [`ReadEvalRequest::new`] or a [`DynamicSourcePolicy`] evaluation method.
//! Guest-language crates own syntax and semantics, not authority envelopes.

pub use sim_lib_core::{
    DynamicSourcePolicy, ReadEvalAdmission, ReadEvalBroker, ReadEvalDecision, ReadEvalOutcome,
    ReadEvalRequest, ReadEvalSource, RequestOrigin, SourceAuthority,
};

#[cfg(test)]
mod tests {
    use super::{DynamicSourcePolicy, ReadEvalRequest, SourceAuthority};

    #[test]
    fn exposes_canonical_request_builders_without_a_facade_envelope() {
        let _ = std::any::type_name::<SourceAuthority>();
        let _ = std::any::type_name::<ReadEvalRequest>();
        let _ = std::any::type_name::<DynamicSourcePolicy>();
    }
}
