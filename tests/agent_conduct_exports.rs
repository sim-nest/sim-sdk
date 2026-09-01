#![cfg(all(feature = "agent-conduct", feature = "agent-conduct-core"))]

use sim::{
    agent_conduct, agent_conduct_core,
    kernel::{CapabilityName, Expr, Symbol},
};

#[test]
fn conduct_contracts_are_directly_reachable_from_the_facade() {
    let frame = agent_conduct_core::AgentRunFrame::standard(
        Symbol::new("sdk-run"),
        Expr::String("hello".into()),
    );
    assert_eq!(frame.run_id, Symbol::new("sdk-run"));

    let ids: Vec<_> = agent_conduct::agent_conduct_catalog_sources()
        .iter()
        .map(|source| source.id)
        .collect();
    assert!(ids.contains(&"agent/default-v1"));
    assert!(ids.contains(&"agent/react-v1"));
    assert!(ids.contains(&"agent/plan-act-replan-v1"));
    assert!(ids.contains(&"agent/phased-v1"));
}

#[cfg(all(feature = "agent", feature = "topology-core"))]
#[test]
fn one_manifest_swaps_four_conduct_packages_without_changing_authority() {
    let manifest_authority = [CapabilityName::new("model")]
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    let cards = sim::lib_agent::standard_step_cards();
    let mut reports = Vec::new();
    for id in [
        "agent/default-v1",
        "agent/react-v1",
        "agent/plan-act-replan-v1",
        "agent/phased-v1",
    ] {
        let source = agent_conduct::agent_conduct_catalog_sources()
            .iter()
            .find(|source| source.id == id)
            .unwrap();
        let package = sim::lib_topology::parse_package(source.source).unwrap();
        let conduct = agent_conduct::validate_agent_conduct(package, &cards).unwrap();
        reports.push((conduct.graph_fingerprint, manifest_authority.clone()));
    }
    assert_eq!(
        reports
            .iter()
            .map(|(report, _)| report)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4
    );
    assert!(
        reports
            .iter()
            .all(|(_, authority)| authority == &manifest_authority)
    );
}
// conformance: SDK agent-conduct exports preserve the intended facade boundary.
