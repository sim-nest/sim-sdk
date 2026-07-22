use std::sync::Mutex;

use sim::codec_bridge::{
    BridgeBook, BridgeCallPayload, BridgeHeader, BridgePacket, BridgePart, BridgeProvenance,
    encode_bridge_text, packet_to_expr, stamp_packet_cid,
};
use sim::forge::{
    ForgeResolver, IntentLibrary, IntentStatus, LiftOptions, ProbeOracle, PromotePolicy,
    RouteAttemptStatus, RoutePolicy, RouteTarget, Verifier, VerifyCatalog, VerifyProbe, run_eval,
    run_intent_routed_report, standard_eval_arms, standard_eval_corpus,
};
use sim::kernel::{
    ContentId, Cx, Error, EvalFabric, EvalReply, EvalRequest, Expr, Result, Symbol,
    testing::eager_cx as cx,
};
use sim::lib_agent_runner_core::ModelResponse;
use sim_value::access::field_any as field;

#[test]
fn forge_eval_report_records_cache_replay_and_downshift_claims() {
    let mut cx = cx();
    let corpus = standard_eval_corpus();
    let report = run_eval(&mut cx, &corpus, &standard_eval_arms()).unwrap();
    let raw = report.metrics("raw-prose-baseline").unwrap();
    let cached = report.metrics("compiled-cached").unwrap();
    let replay = report.metrics("identical-request-replay").unwrap();
    let downshift = report.metrics("compiled-cached-downshifted").unwrap();

    assert_eq!(corpus.len(), 4);
    assert_eq!(cached.compiler_calls, 0);
    assert_eq!(replay.execution_calls, 0);
    assert_eq!(replay.replay_hits, corpus.len() as u64);
    assert!(downshift.tokens < raw.tokens);
    assert!(downshift.accuracy >= raw.accuracy);
}

#[test]
#[ignore = "requires the generated constellation meta-workspace BRIDGE packet-shape dependency set"]
fn forge_sdk_facade_lifts_verifies_reuses_golden_and_routes_downshift() {
    let mut cx = route_cx();
    let verifier_id = Symbol::new("A1");
    let candidate = stamp_packet_cid(&candidate_packet(vec![check_part("A1")])).unwrap();
    let lift = ScriptedLiftFabric::new(vec![packet_to_expr(&candidate)]);
    let mut catalog = verifier_catalog("ok");
    let probe_id = catalog
        .register_probe(
            lift_options().name,
            VerifyProbe {
                args: Expr::List(Vec::new()),
                oracle: ProbeOracle::Expected(Expr::String("ok".to_owned())),
                verifier_ids: vec![verifier_id],
            },
        )
        .unwrap();
    let mut resolver =
        ForgeResolver::new_with_verifiers(IntentLibrary::new(), lift_options(), catalog.clone());

    let verified = resolver
        .resolve(
            &mut cx,
            &lift,
            "summarize the transcript",
            PromotePolicy::AutoVerifiedOnProbePass,
        )
        .unwrap();

    assert_eq!(verified.status, IntentStatus::Verified);
    assert_eq!(verified.probes, vec![probe_id]);
    assert_eq!(verified.approval, None);
    assert_eq!(lift.request_count(), 1);

    let mut golden = verified.clone();
    golden.status = IntentStatus::Golden;
    golden.approval = Some(content_id(99));
    resolver.library_mut().store(golden.clone()).unwrap();
    let lift_must_not_run = FailingLiftFabric::default();

    let reused = resolver
        .resolve(
            &mut cx,
            &lift_must_not_run,
            " summarize \n the\ttranscript ",
            PromotePolicy::KeepCandidate,
        )
        .unwrap();

    assert_eq!(reused, golden);
    assert_eq!(lift_must_not_run.request_count(), 0);

    let cheap = ScriptedAnswerFabric::new(vec![Expr::String("ok".to_owned())]);
    let strong = ScriptedAnswerFabric::new(vec![Expr::String("unused".to_owned())]);
    let policy = RoutePolicy::new(
        vec![
            RouteTarget::new("cheap-downshift", &cheap).with_card("card:cheap"),
            RouteTarget::new("strong", &strong),
        ],
        1,
    )
    .with_repair_retries(0)
    .with_verify_catalog(catalog);

    let report = run_intent_routed_report(&mut cx, &reused, &Expr::Nil, &policy).unwrap();

    assert_eq!(report.answer, Expr::String("ok".to_owned()));
    assert_eq!(report.provenance.target, "cheap-downshift");
    assert_eq!(report.provenance.card, Some("card:cheap".to_owned()));
    assert_eq!(report.attempts[0].status, RouteAttemptStatus::Accepted);
    assert_eq!(cheap.request_count(), 1);
    assert_eq!(strong.request_count(), 0);
}

struct ScriptedLiftFabric {
    responses: Mutex<Vec<Expr>>,
    requests: Mutex<Vec<Expr>>,
}

impl ScriptedLiftFabric {
    fn new(responses: Vec<Expr>) -> Self {
        Self {
            responses: Mutex::new(responses),
            requests: Mutex::new(Vec::new()),
        }
    }

    fn request_count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}

impl EvalFabric for ScriptedLiftFabric {
    fn realize(&self, cx: &mut Cx, request: EvalRequest) -> Result<EvalReply> {
        self.requests.lock().unwrap().push(request.expr.clone());
        let parent_cid = bridge_cid_from_request(&request)?;
        let payload = {
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                return Err(Error::Eval(
                    "scripted SDK forge lift cassette is exhausted".to_owned(),
                ));
            }
            responses.remove(0)
        };
        let reply = stamp_packet_cid(&reply_packet(&parent_cid, payload))?;
        let response = ModelResponse::new(
            Symbol::qualified("runner", "sdk-forge-lift-cassette"),
            "sdk-forge-lift-cassette",
            vec![text_content(encode_bridge_text(
                &reply,
                &BridgeBook::standard(),
            )?)],
            Symbol::new("stop"),
        );
        Ok(EvalReply {
            value: cx.factory().expr(Expr::from(response))?,
            diagnostics: Vec::new(),
            trace: None,
        })
    }
}

#[derive(Default)]
struct FailingLiftFabric {
    requests: Mutex<Vec<Expr>>,
}

impl FailingLiftFabric {
    fn request_count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}

impl EvalFabric for FailingLiftFabric {
    fn realize(&self, _cx: &mut Cx, request: EvalRequest) -> Result<EvalReply> {
        self.requests.lock().unwrap().push(request.expr);
        Err(Error::Eval(
            "golden SDK forge hit must not call the lift cassette".to_owned(),
        ))
    }
}

struct ScriptedAnswerFabric {
    responses: Mutex<Vec<Expr>>,
    requests: Mutex<Vec<Expr>>,
}

impl ScriptedAnswerFabric {
    fn new(responses: Vec<Expr>) -> Self {
        Self {
            responses: Mutex::new(responses),
            requests: Mutex::new(Vec::new()),
        }
    }

    fn request_count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}

impl EvalFabric for ScriptedAnswerFabric {
    fn realize(&self, cx: &mut Cx, request: EvalRequest) -> Result<EvalReply> {
        self.requests.lock().unwrap().push(request.expr);
        let response = {
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                return Err(Error::Eval(
                    "scripted SDK forge answer cassette is exhausted".to_owned(),
                ));
            }
            responses.remove(0)
        };
        let response = ModelResponse::new(
            Symbol::qualified("runner", "sdk-forge-answer-cassette"),
            "sdk-forge-answer-cassette",
            vec![text_content(
                sim::codec_json::expr_to_json(&response).to_string(),
            )],
            Symbol::new("stop"),
        );
        Ok(EvalReply {
            value: cx.factory().expr(Expr::from(response))?,
            diagnostics: Vec::new(),
            trace: None,
        })
    }
}

fn route_cx() -> Cx {
    let mut cx = cx();
    let json = sim::codec_json::JsonCodecLib::new(cx.registry_mut().fresh_codec_id());
    cx.load_lib(&json).unwrap();
    cx
}

fn bridge_cid_from_request(request: &EvalRequest) -> Result<String> {
    match field(&request.expr, "bridge-cid") {
        Some(Expr::String(cid)) => Ok(cid.clone()),
        _ => Err(Error::Eval(
            "SDK forge lift request is missing bridge-cid".to_owned(),
        )),
    }
}

fn text_content(text: String) -> Expr {
    Expr::Map(vec![
        entry("type", Expr::Symbol(Symbol::new("text"))),
        entry("text", Expr::String(text)),
    ])
}

fn reply_packet(parent_cid: &str, payload: Expr) -> BridgePacket {
    BridgePacket {
        header: BridgeHeader {
            cid: None,
            move_kind: Symbol::new("reply"),
            from: "model:forge-lift".to_owned(),
            to: vec!["sim".to_owned()],
            role: Symbol::new("implementer"),
            parents: vec![parent_cid.to_owned()],
            task: Symbol::new("A1"),
            output: Symbol::new("A1"),
            ceiling: Vec::new(),
            context: Vec::new(),
            provenance: BridgeProvenance::default(),
        },
        body: vec![BridgePart {
            id: Symbol::new("A1"),
            kind: Symbol::qualified("bridge", "Return"),
            payload,
        }],
        warrant: None,
    }
}

fn candidate_packet(verifier_parts: Vec<BridgePart>) -> BridgePacket {
    let mut body = vec![
        BridgePart {
            id: Symbol::new("C1"),
            kind: Symbol::qualified("bridge", "Call"),
            payload: BridgeCallPayload::new(Symbol::qualified("forge", "answer")).to_expr(),
        },
        BridgePart {
            id: Symbol::new("O1"),
            kind: Symbol::qualified("bridge", "Return"),
            payload: Expr::Map(vec![
                entry("codec", Expr::Symbol(Symbol::qualified("codec", "json"))),
                entry("shape", Expr::Symbol(Symbol::qualified("core", "String"))),
            ]),
        },
    ];
    body.extend(verifier_parts);
    BridgePacket {
        header: BridgeHeader {
            cid: None,
            move_kind: Symbol::new("request"),
            from: "sim".to_owned(),
            to: vec!["model:worker".to_owned()],
            role: Symbol::new("implementer"),
            parents: Vec::new(),
            task: Symbol::new("C1"),
            output: Symbol::new("O1"),
            ceiling: Vec::new(),
            context: Vec::new(),
            provenance: BridgeProvenance::default(),
        },
        body,
        warrant: None,
    }
}

fn check_part(id: &str) -> BridgePart {
    BridgePart {
        id: Symbol::new(id),
        kind: Symbol::qualified("bridge", "Check"),
        payload: Expr::Map(vec![entry(
            "predicate",
            Expr::Symbol(Symbol::qualified("forge", "equals")),
        )]),
    }
}

fn lift_options() -> LiftOptions {
    LiftOptions {
        name: Symbol::qualified("forge", "summarize"),
        max_repairs: 0,
    }
}

fn verifier_catalog(expected: &str) -> VerifyCatalog {
    let mut catalog = VerifyCatalog::new();
    catalog.register_verifier(
        Symbol::new("A1"),
        Verifier::Assertion {
            predicate: Expr::Map(vec![
                entry(
                    "predicate",
                    Expr::Symbol(Symbol::qualified("forge", "equals")),
                ),
                entry("expected", Expr::String(expected.to_owned())),
            ]),
        },
    );
    catalog
}

fn content_id(byte: u8) -> ContentId {
    ContentId::from_bytes(Symbol::qualified("core", "sha256"), [byte; 32])
}

fn entry(key: &str, value: Expr) -> (Expr, Expr) {
    (Expr::Symbol(Symbol::new(key)), value)
}

#[test]
fn forge_eval_field_lookup_uses_provider_key_policy() {
    let bare = Expr::Map(vec![entry("bridge-cid", Expr::String("bare".into()))]);
    let string_key = Expr::Map(vec![(
        Expr::String("bridge-cid".into()),
        Expr::String("string".into()),
    )]);
    let qualified = Expr::Map(vec![(
        Expr::Symbol(Symbol::qualified("bridge", "bridge-cid")),
        Expr::String("qualified".into()),
    )]);

    assert_eq!(
        field(&bare, "bridge-cid"),
        Some(&Expr::String("bare".into()))
    );
    assert_eq!(
        field(&string_key, "bridge-cid"),
        Some(&Expr::String("string".into()))
    );
    assert_eq!(field(&qualified, "bridge-cid"), None);
}
