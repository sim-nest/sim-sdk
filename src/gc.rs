//! Standard-distribution managed-object collection policy.
//!
//! `standard` selects tracing collection. Retention is available only through
//! the explicit `standard-gc-retain` feature for minimal and test closures; it
//! never reclaims cycles and fails closed at the arena's hard object cap.

use sim_lib_standard_core::LanguageProfile;

/// The managed-object policy selected by this SDK build.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollectorPolicy {
    /// Bounded stop-the-world tracing, including unreachable-cycle reclamation.
    #[cfg(feature = "standard-gc-tracing")]
    Tracing,
    /// Hard-capped retention for explicit minimal/test builds; cycles leak.
    #[cfg(feature = "standard-gc-retain")]
    RetainCycles,
}

/// Stable inspection projection for the selected distribution policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CollectorInspection {
    /// Machine-readable policy name.
    pub policy: &'static str,
    /// Whether unreachable cycles are reclaimed.
    pub reclaims_cycles: bool,
    /// Intended distribution scope.
    pub scope: &'static str,
}

/// Returns the policy selected by the feature closure.
pub const fn selected_policy() -> CollectorPolicy {
    #[cfg(feature = "standard-gc-tracing")]
    return CollectorPolicy::Tracing;
    #[cfg(all(not(feature = "standard-gc-tracing"), feature = "standard-gc-retain"))]
    return CollectorPolicy::RetainCycles;
    #[cfg(not(any(feature = "standard-gc-tracing", feature = "standard-gc-retain")))]
    compile_error!("standard-mutation requires an explicit collector policy");
}

/// Projects the selected policy through the SDK's ordinary inspection API.
pub const fn inspect_selected_policy() -> CollectorInspection {
    match selected_policy() {
        #[cfg(feature = "standard-gc-tracing")]
        CollectorPolicy::Tracing => CollectorInspection {
            policy: "gc/tracing",
            reclaims_cycles: true,
            scope: "standard-production",
        },
        #[cfg(feature = "standard-gc-retain")]
        CollectorPolicy::RetainCycles => CollectorInspection {
            policy: "gc/retain-hard-capped",
            reclaims_cycles: false,
            scope: "explicit-minimal-or-test",
        },
    }
}

/// Declares whether a guest profile allocates managed objects that may cycle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManagedAllocation {
    /// The profile does not allocate managed cyclic graphs.
    Acyclic,
    /// The profile may allocate reference cycles and therefore needs reclamation.
    Cyclic,
}

/// Admits a guest profile to a production distribution.
///
/// Cyclic profiles fail closed unless the build selected a reclaiming collector;
/// retention is never silently treated as production garbage collection.
pub fn require_production_collector(
    profile: &LanguageProfile,
    allocation: ManagedAllocation,
) -> sim_kernel::Result<CollectorInspection> {
    for (field, symbol) in [
        ("reader", &profile.reader),
        ("lowering", &profile.lowering),
        ("eval policy", &profile.eval_policy),
    ] {
        if symbol.namespace.as_deref() == Some("standard/unspecified") {
            return Err(sim_kernel::Error::Eval(format!(
                "guest profile {} has no declared {field}",
                profile.symbol
            )));
        }
    }
    if profile.organs.is_empty()
        || profile.capabilities.is_empty()
        || profile.unsupported_forms.is_empty()
    {
        return Err(sim_kernel::Error::Eval(format!(
            "guest profile {} has incomplete production evidence",
            profile.symbol
        )));
    }
    let inspection = inspect_selected_policy();
    if allocation == ManagedAllocation::Cyclic && !inspection.reclaims_cycles {
        return Err(sim_kernel::Error::Eval(format!(
            "production guest profile {} allocates managed cycles but selected policy {} does not reclaim them",
            profile.symbol, inspection.policy
        )));
    }
    Ok(inspection)
}

#[cfg(test)]
mod tests {
    use sim_kernel::{CapabilityName, Symbol};
    #[cfg(feature = "standard-gc-tracing")]
    use sim_lib_mutation::{
        EdgeId, EdgeVisitor, HardCappedRetainPolicy, ManagedArena, ManagedId, ManagedObject,
    };

    use super::*;

    #[cfg(feature = "standard-gc-tracing")]
    #[derive(Clone, Default)]
    struct Node(Vec<ManagedId>);
    #[cfg(feature = "standard-gc-tracing")]
    impl ManagedObject for Node {
        fn trace_edges(&self, visitor: &mut dyn EdgeVisitor) {
            for (edge, target) in self.0.iter().copied().enumerate() {
                visitor.strong(EdgeId(edge as u32), target);
            }
        }
        fn clear_weak_edge(&mut self, _: EdgeId, _: ManagedId) -> bool {
            false
        }
    }

    fn cyclic_guest() -> LanguageProfile {
        LanguageProfile::new(Symbol::qualified("lang", "cyclic-specimen/v1"))
            .with_reader(Symbol::qualified("codec", "lisp"))
            .with_lowering(Symbol::qualified("lower", "cyclic"))
            .with_eval_policy(Symbol::qualified("eval", "eager"))
            .with_organ(sim_lib_standard_core::OrganUse::new(Symbol::qualified(
                "organ", "mutation",
            )))
            .requiring(CapabilityName::new("managed.allocate"))
            .with_unsupported_form(Symbol::qualified("gap", "native-finalizer"))
    }

    #[test]
    #[cfg(feature = "standard-gc-tracing")]
    fn standard_tracing_reclaims_cycle_while_explicit_retention_hits_cap() {
        let inspection =
            require_production_collector(&cyclic_guest(), ManagedAllocation::Cyclic).unwrap();
        assert_eq!(inspection.policy, "gc/tracing");

        let mut traced = ManagedArena::new(HardCappedRetainPolicy::new(2).unwrap());
        let a = traced.allocate(Node::default()).unwrap();
        let b = traced.allocate(Node(vec![a.id()])).unwrap();
        traced.get_mut(a).unwrap().0.push(b.id());
        let receipt = sim_lib_gc_tracing::collect(
            &mut traced,
            sim_lib_gc_tracing::CollectionLimits {
                objects: 2,
                edges: 2,
                stack: 2,
                work: 16,
                clears: 0,
                finalizers: 0,
            },
        )
        .unwrap();
        assert_eq!(receipt.swept, vec![a.id(), b.id()]);
        assert!(traced.is_empty());

        let mut retained = ManagedArena::new(HardCappedRetainPolicy::new(2).unwrap());
        retained.allocate(Node::default()).unwrap();
        retained.allocate(Node::default()).unwrap();
        assert!(matches!(
            retained.allocate(Node::default()),
            Err(sim_lib_mutation::ArenaError::CapacityExceeded { cap: 2 })
        ));
    }

    #[test]
    #[cfg(all(feature = "standard-gc-retain", not(feature = "standard-gc-tracing")))]
    fn explicit_retention_cannot_admit_a_cyclic_production_guest() {
        let error = require_production_collector(&cyclic_guest(), ManagedAllocation::Cyclic)
            .expect_err("retain-only policy must fail closed");
        assert!(error.to_string().contains("does not reclaim"));
        assert_eq!(inspect_selected_policy().scope, "explicit-minimal-or-test");
    }
}
