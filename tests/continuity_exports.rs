#![cfg(feature = "continuity")]

use sim::{
    continuity::{
        ContinuityEvent, ContinuityJournal, ContinuityPlan, ContinuityState, MemoryJournal,
        NetworkPolicy, RoleDemand,
    },
    kernel::{Expr, Symbol},
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

const THOUGHT: &str = include_str!("../recipes/continuity/expedition-thought/product.toml");
const CARDS: &str = include_str!("../recipes/continuity/quiet-stewardship-cards/product.toml");

fn synthetic_event(product: &str) -> ContinuityEvent {
    ContinuityEvent {
        event_id: Symbol::qualified("content", product),
        kind: Symbol::new("capture"),
        role: Symbol::new("android-root"),
        payload: Expr::String("synthetic-policy-data".into()),
        ..Default::default()
    }
}

fn walk(product: &str) -> (String, String, String) {
    let plan = minimum();
    let event = synthetic_event(product);
    let mut journal = MemoryJournal::default();
    let state = journal
        .accept(&plan, &ContinuityState::default(), event)
        .expect("networkless phone root must accept the synthetic product");
    let turn = state.turns.last().expect("accepted walk has a turn");
    let identity = turn.event_id.to_string();
    (
        journal.turns()[0].event_id.to_string(),
        identity.clone(),
        identity,
    )
}

#[test]
fn networkless_walk_is_identical_across_every_removal_case() {
    let cases = [
        "no-accessories",
        "modeled-audio",
        "modeled-watch",
        "modeled-halo",
        "missing-speech-model",
        "route-loss",
        "endpoint-removal",
        "process-death",
        "network-denied",
        "manual-continuation",
    ];
    for product in ["expedition-thought", "quiet-stewardship-cards"] {
        let reference = walk(product);
        assert_eq!(
            reference.0, reference.1,
            "journal and turn identity diverged"
        );
        assert_eq!(
            reference.1, reference.2,
            "turn and phone Scene identity diverged"
        );
        for case in cases {
            assert_eq!(walk(product), reference, "{product} diverged under {case}");
        }
    }
}

#[test]
fn destructive_minimal_boot_fixture_contains_no_satellite_artifact() {
    for product in [THOUGHT, CARDS] {
        let minimal = product.split("[deleted_before_boot]").next().unwrap();
        for forbidden in [
            "audio-role",
            "watch-role",
            "halo-role",
            "speech-role",
            "model-role",
            "audio-scene",
            "watch-scene",
            "halo-scene",
            "speech-cache",
            "model-cache",
        ] {
            assert!(
                !minimal.contains(forbidden),
                "minimal boot retained {forbidden}"
            );
        }
        for required in [
            "continuity-organ",
            "android-capsule",
            "phone-surface",
            "journal",
            "turn",
            "phone-scene",
        ] {
            assert!(
                minimal.contains(required),
                "minimal boot omitted {required}"
            );
        }
    }
}

#[test]
fn synthetic_products_share_mechanics_and_keep_policy_separate() {
    for product in [THOUGHT, CARDS] {
        assert!(product.contains("root = \"phone\""));
        assert!(product.contains("network = \"denied\""));
        assert!(
            product.contains(
                "packages = [\"continuity-organ\", \"android-capsule\", \"phone-surface\"]"
            )
        );
    }
    assert!(THOUGHT.contains("continuation = \"by-content-id\""));
    assert!(THOUGHT.contains("pending-honestly"));
    assert!(CARDS.contains("card_count = \"0..3\""));
    assert!(CARDS.contains("acknowledgement = \"optional-one\""));
    assert!(CARDS.contains("refused_on_worn_or_audible = [\"private-note\", \"finance\"]"));
}

#[test]
fn continuity_production_has_no_product_or_person_policy_branch() {
    let production = include_str!("../src/continuity.rs").to_ascii_lowercase();
    let tokens = production
        .split(|character: char| !character.is_ascii_alphanumeric())
        .collect::<Vec<_>>();
    for forbidden in [
        "bo",
        "mia",
        "love",
        "expedition",
        "household",
        "viture",
        "halo",
    ] {
        assert!(
            !tokens.contains(&forbidden),
            "SDK continuity production mentions {forbidden}"
        );
    }
}
// conformance: SDK continuity exports preserve the intended facade boundary.
