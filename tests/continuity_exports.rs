#![cfg(feature = "continuity")]

use sim::{
    continuity::{ContinuityPlan, NetworkPolicy, RoleDemand},
    kernel::Symbol,
};

const PLAN_DATA: &str = include_str!("../recipes/continuity/plans/plans.toml");
const REQUIRED: [&str; 7] = [
    "phone-review",
    "lifecycle",
    "mounts",
    "capture",
    "render",
    "stop",
    "journal-append",
];

fn symbols(values: &[&str]) -> Vec<Symbol> {
    values.iter().copied().map(Symbol::new).collect()
}

fn minimum() -> ContinuityPlan {
    ContinuityPlan {
        plan_id: Symbol::qualified("continuity", "carrier-only"),
        roles: vec![RoleDemand {
            role: Symbol::new("android-root"),
            root: true,
            required_services: symbols(&REQUIRED),
            fallbacks: vec![],
        }],
        available_services: symbols(&REQUIRED),
        network: NetworkPolicy::Offline,
        ..ContinuityPlan::default()
    }
}

fn rejected_before_bind(plan: &ContinuityPlan) -> bool {
    let mut bound = false;
    let rejected = plan.validate().is_err();
    if !rejected {
        bound = true;
    }
    assert!(!bound, "invalid plan reached the service-bind boundary");
    rejected
}

#[test]
fn versioned_plan_data_keeps_every_endpoint_optional() {
    for id in [
        "carrier-only",
        "walk",
        "desk",
        "halo-optional",
        "future-endpoint",
    ] {
        assert!(PLAN_DATA.contains(&format!("id = \"continuity/{id}\"")));
    }
    assert!(PLAN_DATA.contains("revision = 1"));
    assert!(PLAN_DATA.contains("lunar-lapel-projector"));
    assert!(
        minimum().validate().is_ok(),
        "deleting all optional roles must remain complete"
    );
}

#[test]
fn watch_only_has_the_stable_no_root_refusal() {
    assert!(PLAN_DATA.contains("id = \"continuity/watch-only\""));
    assert!(PLAN_DATA.contains("expected = \"no-continuity-root\""));
    let mut plan = minimum();
    plan.roles[0].root = false;
    assert!(rejected_before_bind(&plan));
}

#[test]
fn hostile_plans_fail_before_service_binding() {
    let mut malformed_roots = minimum();
    malformed_roots.roles.push(RoleDemand {
        role: Symbol::new("second-root"),
        root: true,
        required_services: vec![],
        fallbacks: vec![],
    });
    assert!(rejected_before_bind(&malformed_roots));

    let mut remote_fallback = minimum();
    remote_fallback.allowed_network_routes = vec![Symbol::new("remote-model")];
    assert!(rejected_before_bind(&remote_fallback));

    let mut missing_stop = minimum();
    missing_stop
        .available_services
        .retain(|service| service.to_string() != "stop");
    assert!(rejected_before_bind(&missing_stop));

    let mut unbounded_retention = minimum();
    unbounded_retention.retention_turns = 0;
    assert!(rejected_before_bind(&unbounded_retention));

    let mut aggregate_grants = minimum();
    aggregate_grants.disclosure = vec![
        Symbol::new("aggregate-grant"),
        Symbol::new("aggregate-grant"),
    ];
    assert!(rejected_before_bind(&aggregate_grants));

    let mut cyclic_fallback = minimum();
    cyclic_fallback.roles[0].fallbacks = vec![Symbol::new("android-root")];
    assert!(rejected_before_bind(&cyclic_fallback));
}

#[test]
fn sdk_exports_the_owner_types_without_a_wrapper_model() {
    let _: sim::continuity::ContinuityPlan = minimum();
    let _: sim::continuity::MemoryJournal = Default::default();
}
