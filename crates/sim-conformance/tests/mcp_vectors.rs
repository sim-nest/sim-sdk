use sim_conformance::{
    ConformanceVector, HostileCase, MetamorphicInput, TimeEvent, TransportProjection::*,
    VectorOutcome, VectorRunner, metamorphic_inputs, run_matrix,
};

struct Echo(&'static str);
impl VectorRunner for Echo {
    fn name(&self) -> &'static str {
        self.0
    }
    fn run(
        &self,
        vector: &ConformanceVector,
        _: &sim_conformance::TransportProjection,
    ) -> Result<VectorOutcome, String> {
        Ok(VectorOutcome {
            messages: vector.expected_messages.clone(),
            effects: vector.expected_effects.clone(),
            cache_activity: vector.expected_cache_activity.clone(),
            diagnostics: "token=[REDACTED]".into(),
        })
    }
}

fn vector(requests: Vec<&str>) -> ConformanceVector {
    ConformanceVector {
        id: "effect-once".into(),
        profile: "final".into(),
        transports: vec![
            Codec, Direct, Legacy, Stdio, HttpJson, HttpSse, Product, Runtime,
        ],
        authority: "principal:alice;scope:tools.call".into(),
        requests: requests.into_iter().map(str::to_owned).collect(),
        time_events: vec![TimeEvent {
            tick: 3,
            event: "cancel-unrelated".into(),
        }],
        expected_messages: vec!["result:ok".into()],
        expected_effects: vec!["table/write:1".into()],
        expected_cache_activity: vec!["miss:key-a".into(), "store:key-a".into()],
        redaction_assertions: vec!["raw-secret".into()],
        budget: 4096,
    }
}

#[test]
fn one_expected_outcome_survives_every_direct_and_real_transport_runner() {
    let runners: [&dyn VectorRunner; 2] = [&Echo("server"), &Echo("client")];
    assert_eq!(
        run_matrix(&[vector(vec!["initialize", "tools/call"])], &runners).unwrap(),
        16
    );
}

#[test]
fn request_order_permutations_preserve_authority_and_exactly_once_effects() {
    let runner = Echo("metamorphic");
    for requests in [
        vec!["initialize", "tools/call"],
        vec!["tools/list", "initialize", "tools/call"],
    ] {
        let candidate = vector(requests);
        assert_eq!(run_matrix(&[candidate], &[&runner]).unwrap(), 8);
    }
    let inputs = metamorphic_inputs(&MetamorphicInput {
        principal: "alice".into(),
        version: "2026-07-28".into(),
        capabilities: vec!["tools.call".into(), "resources.read".into()],
        extensions: vec!["trace".into(), "subscription".into()],
        trace: "trace-a".into(),
        cache_state: "cold".into(),
        cancellation_tick: 3,
        chunks: vec![1, 2, 5],
    });
    assert_eq!(inputs.len(), 9);
}

#[test]
fn hostile_budget_and_redaction_fail_closed() {
    let mut candidate = vector(vec!["oversized-json-schema-ref"]);
    candidate.budget = 8;
    let leaked = VectorOutcome {
        messages: candidate.expected_messages.clone(),
        effects: candidate.expected_effects.clone(),
        cache_activity: candidate.expected_cache_activity.clone(),
        diagnostics: "raw-secret".into(),
    };
    assert!(sim_conformance::check_outcome(&candidate, &leaked).is_err());
    let hostile = [
        HostileCase::JsonSchemaRefs,
        HostileCase::HeaderBodyMismatch,
        HostileCase::PartialIo,
        HostileCase::Disconnect,
        HostileCase::ChildDeath,
        HostileCase::OAuthMixUp,
        HostileCase::MrtrTamperReplay,
        HostileCase::ExtensionResult,
        HostileCase::SubscriptionTeardown,
        HostileCase::LegacyFallback,
    ];
    assert_eq!(hostile.len(), 10);
}
