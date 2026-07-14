//! Capability names used by the core runtime surface.

use sim_kernel::CapabilityName;

/// The capability gating browse reads (`browse.read`).
pub fn browse_read_capability() -> CapabilityName {
    CapabilityName::new("browse.read")
}

/// The capability gating browse-driven test runs (`browse.run-tests`).
pub fn browse_run_tests_capability() -> CapabilityName {
    CapabilityName::new("browse.run-tests")
}

/// The capability gating internal browse surfaces (`browse.internal`).
pub fn browse_internal_capability() -> CapabilityName {
    CapabilityName::new("browse.internal")
}

/// The capability gating the configured list implementation (`config.list.impl`).
pub fn config_list_impl_capability() -> CapabilityName {
    CapabilityName::new("config.list.impl")
}

/// The capability gating the configured table implementation (`config.table.impl`).
pub fn config_table_impl_capability() -> CapabilityName {
    CapabilityName::new("config.table.impl")
}

#[cfg(test)]
mod tests {
    use super::{
        browse_internal_capability, browse_read_capability, browse_run_tests_capability,
        config_list_impl_capability, config_table_impl_capability,
    };

    #[test]
    fn capability_tokens_are_stable() {
        assert_eq!(browse_read_capability().as_str(), "browse.read");
        assert_eq!(browse_run_tests_capability().as_str(), "browse.run-tests");
        assert_eq!(browse_internal_capability().as_str(), "browse.internal");
        assert_eq!(config_list_impl_capability().as_str(), "config.list.impl");
        assert_eq!(config_table_impl_capability().as_str(), "config.table.impl");
    }
}
