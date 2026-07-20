//! Executable conformance checks for the architecture claims in `SIM.md`.
//!
//! The suite intentionally lives outside the root `sim` crate and uses only the
//! public facade. It protects the checkable pieces of the "Non-Negotiable
//! Goals", "Security Model", and "Design Bets": codec totality over `Expr`,
//! class-as-function behavior, replaceable number-domain parsing and
//! promotion, read-eval/read-construct security, named eval policies, loader
//! backends, wasm ABI v1 export scope, and stream transport conformance.
//!
//! The stream-cassette assertions validate the in-memory `to_expr`/`from_expr`
//! serialization round-trip and the structural invariants a cassette must
//! satisfy to be publishable as a golden fixture. They do not compare against a
//! committed `.simcassette` corpus on disk; `validate_golden_fixture` checks a
//! cassette's invariants against a target publish path without reading a file.

use std::sync::Arc;

use sim::{
    codec::{Input, decode_with_codec},
    kernel::{
        Args, CapabilitySet, Cx, DefaultFactory, ExportKind, ExportState, Expr, LibSource,
        LibTarget, NeedPolicy, NumberLiteral, ReadPolicy, Symbol, TrustLevel,
        macro_expand_eval_capability, read_construct_capability, read_eval_capability,
    },
};

#[path = "conformance_support/mod.rs"]
mod conformance_support;
#[path = "spec/forge_author.rs"]
mod forge_author;
#[path = "spec/forge_eval.rs"]
mod forge_eval;
#[path = "spec/instrument_streams.rs"]
mod instrument_streams;
#[path = "spec/rust_intelligence.rs"]
mod rust_intelligence;
#[path = "spec/stream_matrix.rs"]
mod stream_matrix;
#[path = "spec/support.rs"]
mod support;
#[path = "spec/surface_protocol.rs"]
mod surface_protocol;

use support::*;

#[test]
fn sim_md_declares_conformance_backing() {
    let contract = normalized_conformance_contract();
    assert!(contract.contains("`sim-conformance`"));
    assert!(contract.contains("public facade only"));
    assert!(contract.contains("stream transport conformance"));
    assert!(contract.contains(stream_matrix::MATRIX_PATH));
}

#[test]
fn every_general_codec_roundtrips_every_expr_variant() {
    let mut cx = cx();
    let exprs = expr_corpus();
    assert_expr_coverage(&exprs);

    for codec in codec_symbols() {
        for expr in &exprs {
            let encoded = encode_once(&mut cx, &codec, expr);
            let decoded = decode_once(&mut cx, &codec, encoded);
            assert_eq!(
                decoded, *expr,
                "codec {codec} failed to round-trip {expr:?}"
            );
        }
    }
}

#[test]
fn every_registered_class_exposes_callable_class_protocol() {
    let cx = cx();
    let class_symbols = cx.registry().classes().keys().cloned().collect::<Vec<_>>();
    assert!(!class_symbols.is_empty());

    let mut checked_class_protocol = std::collections::BTreeSet::new();
    let mut marker_exports = std::collections::BTreeSet::new();
    for symbol in class_symbols {
        let class = cx.resolve_class(&symbol).unwrap();
        if class.object().as_class().is_some() {
            checked_class_protocol.insert(symbol.clone());
            assert!(
                class.object().as_callable().is_some(),
                "{symbol} must be callable as its constructor"
            );
        } else {
            marker_exports.insert(symbol.clone());
        }
    }

    for symbol in [
        q("core", "Class"),
        q("core", "Function"),
        q("core", "Number"),
    ] {
        assert!(
            checked_class_protocol.contains(&symbol),
            "{symbol} must expose the full class protocol"
        );
    }
    assert!(marker_exports.contains(&q("numbers", "tensor-literal")));
}

#[test]
fn public_class_constructor_constructs_instance() {
    let mut cx = cx();
    let lib = marker_class_lib(&mut cx);
    cx.load_lib(&lib).unwrap();

    let value = cx
        .call_class(&marker_symbol(), Args::new(Vec::new()))
        .unwrap();
    let class = value.object().class(&mut cx).unwrap();
    assert_eq!(
        class.object().as_expr(&mut cx).unwrap(),
        Expr::Symbol(marker_symbol())
    );
}

#[test]
fn number_domains_named_by_sim_parse_and_promote_through_lattice() {
    let mut cx = cx();
    for (text, domain) in [
        ("1.5", q("numbers", "f64")),
        ("42", q("numbers", "i64")),
        ("1/2", q("numbers", "rational")),
        ("1000000000000000000000000", q("numbers", "bigint")),
        ("1+2i", q("numbers", "complex")),
    ] {
        let literal = cx.parse_number_literal(text).unwrap().unwrap();
        assert_eq!(literal.domain, domain, "{text} parsed into wrong domain");
    }

    let value = cx
        .call_function(
            &q("math", "add"),
            Args::new(vec![
                cx.factory()
                    .number_literal(q("numbers", "f64"), "1.5".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(q("numbers", "complex"), "0.5+2i".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::Number(NumberLiteral {
            domain: q("numbers", "complex"),
            canonical: "2+2i".to_owned(),
        })
    );

    assert_lattice_reaches(&cx, q("numbers", "i64"), q("numbers", "rational"));
    assert_lattice_reaches(&cx, q("numbers", "bigint"), q("numbers", "rational"));
    assert_lattice_reaches(&cx, q("numbers", "complex"), q("numbers", "cas"));
}

#[test]
fn read_eval_is_capability_and_trust_gated_separately_from_read_construct() {
    let (mut cx, seat) = seated_cx();
    let denied = decode_with_codec(
        &mut cx,
        &q("codec", "lisp"),
        Input::Text("#eval(1)".to_owned()),
        ReadPolicy::default(),
    );
    assert!(matches!(
        denied,
        Err(sim::kernel::Error::CapabilityDenied { capability })
            if capability == read_eval_capability()
    ));

    let untrusted = decode_with_codec(
        &mut cx,
        &q("codec", "lisp"),
        Input::Text("#eval(1)".to_owned()),
        ReadPolicy {
            trust: TrustLevel::Untrusted,
            capabilities: CapabilitySet::new().grant(read_eval_capability()),
        },
    );
    assert!(matches!(
        untrusted,
        Err(sim::kernel::Error::TrustDenied { capability, trust })
            if capability == read_eval_capability() && trust == TrustLevel::Untrusted
    ));

    grant_capability(&seat, &mut cx, macro_expand_eval_capability());
    let allowed = decode_with_codec(
        &mut cx,
        &q("codec", "lisp"),
        Input::Text("#eval(1)".to_owned()),
        ReadPolicy {
            trust: TrustLevel::HostInternal,
            capabilities: CapabilitySet::new().grant(read_eval_capability()),
        },
    )
    .unwrap();
    match allowed {
        Expr::Number(number) => assert_eq!(number.canonical, "1"),
        other => panic!("expected read-eval number, got {other:?}"),
    }

    let lib = marker_class_lib(&mut cx);
    cx.load_lib(&lib).unwrap();
    let read_construct_policy = ReadPolicy {
        trust: TrustLevel::HostInternal,
        capabilities: CapabilitySet::new().grant(read_construct_capability()),
    };

    let read_construct_denied = decode_with_codec(
        &mut cx,
        &q("codec", "lisp"),
        Input::Text("#(ConformanceMarker)".to_owned()),
        ReadPolicy::default(),
    );
    assert!(matches!(
        read_construct_denied,
        Err(sim::kernel::Error::CapabilityDenied { capability })
            if capability == read_construct_capability()
    ));

    grant_capability(&seat, &mut cx, read_construct_capability());
    decode_with_codec(
        &mut cx,
        &q("codec", "lisp"),
        Input::Text("#(ConformanceMarker)".to_owned()),
        read_construct_policy.clone(),
    )
    .unwrap();

    let read_eval_inside_construct = decode_with_codec(
        &mut cx,
        &q("codec", "lisp"),
        Input::Text("#(ConformanceMarker #eval(1))".to_owned()),
        read_construct_policy,
    );
    assert!(matches!(
        read_eval_inside_construct,
        Err(sim::kernel::Error::CapabilityDenied { capability })
            if capability == read_eval_capability()
    ));
}

#[test]
fn eval_policies_named_by_runtime_exist() {
    let mut cx = Cx::new(Arc::new(NeedPolicy), Arc::new(DefaultFactory));
    sim::runtime::install_core_runtime(&mut cx);
    let policies = cx
        .call_function(&q("core", "eval-policies"), Args::new(Vec::new()))
        .unwrap();
    let Expr::List(entries) = policies.object().as_expr(&mut cx).unwrap() else {
        panic!("expected eval policy entries");
    };

    for policy in ["eager", "lazy", "lazy-by-need", "strict-by-shape", "hybrid"] {
        assert!(entries.iter().any(|entry| {
            table_value(entry, &Symbol::new("id")) == Some(&Expr::Symbol(q("core", policy)))
        }));
    }
}

#[test]
fn loader_backends_named_by_runtime_are_available() {
    let mut cx = cx();
    let registry = sim::loaders::standard_loader_registry();
    let host_lib = marker_class_lib(&mut cx);
    let manifest = registry
        .inspect_manifest(&mut cx, LibSource::Host(Box::new(host_lib)))
        .unwrap();
    assert_eq!(manifest.target, LibTarget::HostRegistered);

    assert_loader_selected(
        registry.load_lib(&mut cx, sim::loaders::path_source("missing.l8b")),
        "binary-precompiled-lib",
    );
    assert_loader_selected(
        registry.load_lib(&mut cx, sim::loaders::path_source("missing.lisp")),
        "lisp-source",
    );
    assert_loader_selected(
        registry.load_lib(
            &mut cx,
            sim::loaders::path_source(format!("missing.{}", std::env::consts::DLL_EXTENSION)),
        ),
        "native-dylib",
    );

    let wasm_registry = sim::loaders::standard_loader_registry_with_wasm(Arc::new(
        sim::wasm_abi::InMemoryWasmRuntime::new(),
    ));
    assert_loader_selected(
        wasm_registry.load_lib(&mut cx, sim::loaders::path_source("missing.wasm")),
        "wasm-abi-module",
    );
}

#[test]
fn wasm_abi_v1_executes_functions_and_marks_richer_exports_unsupported() {
    let mut cx = cx();
    let lib = StubWasmExportsLib {
        exports: vec![
            sim::wasm_abi::WasmExport::Function {
                symbol: q("wasm-test", "call"),
            },
            sim::wasm_abi::WasmExport::Class {
                symbol: q("wasm-test", "Class"),
                constructor: None,
            },
            sim::wasm_abi::WasmExport::Codec {
                symbol: q("codec", "wasm-test"),
            },
            sim::wasm_abi::WasmExport::Shape {
                symbol: q("wasm-test", "Shape"),
            },
            sim::wasm_abi::WasmExport::NumberDomain {
                symbol: q("numbers", "wasm-test"),
            },
        ],
    };
    cx.load_lib(&lib).unwrap();

    let function = cx.resolve_function(&q("wasm-test", "call")).unwrap();
    assert!(function.object().as_callable().is_some());

    let loaded = cx.registry().lib(&q("wasm-test", "abi")).unwrap();
    assert_export_state(
        loaded,
        ExportKind::FUNCTION,
        q("wasm-test", "call"),
        |state| matches!(state, ExportState::Resolved { .. }),
    );
    for (kind, symbol, reason) in [
        (
            ExportKind::CLASS,
            q("wasm-test", "Class"),
            "class runtime exports",
        ),
        (
            ExportKind::CODEC,
            q("codec", "wasm-test"),
            "codec runtime exports",
        ),
        (
            ExportKind::SHAPE,
            q("wasm-test", "Shape"),
            "shape runtime exports",
        ),
        (
            ExportKind::NUMBER_DOMAIN,
            q("numbers", "wasm-test"),
            "number-domain runtime exports",
        ),
    ] {
        assert_export_state(
            loaded,
            kind,
            symbol,
            |state| matches!(state, ExportState::Unsupported { reason: found } if found.contains(reason)),
        );
    }
    assert!(CONFORMANCE_CONTRACT.contains("wasm ABI scope"));
}

#[test]
fn conformance_contract_describes_current_scope() {
    let contract = normalized_conformance_contract();
    for phrase in [
        "codec totality",
        "class semantics",
        "number-domain replaceability",
        "capability gating",
        "eval policy",
        "loader behavior",
        "reversible library lifecycle",
        "boot receipt replay",
        "wasm ABI scope",
        "stream transport conformance",
    ] {
        assert!(
            contract.contains(phrase),
            "missing conformance scope phrase: {phrase}"
        );
    }
}

#[test]
fn stream_cassettes_replay_and_round_trip_through_codecs_and_publish_invariants() {
    use sim::lib_server::FrameEnvelope;
    use sim::lib_stream_combinators::{Stream, record_cassette_bang, replay_cassette};
    use sim::lib_stream_core::{
        ClockDomain, StreamMedia, StreamPacket, StreamValue, TransportProfile,
        stream_remote_network_capability,
    };
    use sim::lib_stream_fabric::{
        cassette_to_stream_frames, stream_frames_to_cassette, stream_frames_to_stream,
        stream_to_frames,
    };
    use sim::lib_stream_host::{FakeBackend, HostBackendRegistry, HostCallbackCassette};
    use sim::lib_web_bridge::{FixtureTransport, Transport};

    let mut serde_cx = cx();

    let midi_items = vec![midi_item(0), midi_item(240)];
    let memory = Stream::pull(
        conformance_metadata(
            "stream/conformance-memory",
            StreamMedia::Midi,
            ClockDomain::MidiTick,
        ),
        midi_items.clone(),
    );
    let cassette = record_cassette_bang(&memory, TransportProfile::lan_midi_control()).unwrap();
    assert_eq!(
        replay_cassette(&cassette).unwrap().take_packets(4).unwrap(),
        midi_items
    );
    assert_publishable_fixture(
        &mut serde_cx,
        &cassette,
        "fixtures/streams/golden/conformance-midi.simcassette",
    );

    let mut registry = HostBackendRegistry::new();
    registry.register(FakeBackend::new()).unwrap();
    let opened = registry
        .open(FakeBackend::data_request(4).unwrap())
        .unwrap();
    let mut host = HostCallbackCassette::new();
    host.record_packet(StreamPacket::data(
        q("stream/data", "expr"),
        Expr::String("host callback".to_owned()),
    ));
    let shared_host = host
        .to_stream_cassette(
            opened.config().metadata(),
            TransportProfile::remote_stream_fabric(),
        )
        .unwrap();
    HostCallbackCassette::from_stream_cassette(&shared_host)
        .unwrap()
        .replay(opened.queue())
        .unwrap();
    assert_eq!(opened.queue().drain(4).unwrap().len(), 1);

    let (mut fabric_cx, fabric_seat) = seated_cx();
    grant_capability(
        &fabric_seat,
        &mut fabric_cx,
        stream_remote_network_capability(),
    );
    let server_stream = StreamValue::pull(
        conformance_metadata(
            "stream/conformance-server",
            StreamMedia::Midi,
            ClockDomain::MidiTick,
        ),
        vec![midi_item(0), midi_item(240)],
    );
    let frames = stream_to_frames(&mut fabric_cx, &server_stream, q("codec", "lisp")).unwrap();
    let server_cassette = stream_frames_to_cassette(&mut fabric_cx, &frames).unwrap();
    let replay_frames = cassette_to_stream_frames(
        &mut fabric_cx,
        &server_cassette,
        q("codec", "lisp"),
        FrameEnvelope::default(),
    )
    .unwrap();
    let server_replay = stream_frames_to_stream(&mut fabric_cx, &replay_frames).unwrap();
    assert_eq!(server_replay.take_packets(4).unwrap().len(), 2);

    let pcm_items = vec![pcm_item(1.0), pcm_item(-1.0)];
    let pcm_stream = StreamValue::pull(
        conformance_metadata(
            "stream/conformance-web",
            StreamMedia::Pcm,
            ClockDomain::BrowserFrame,
        ),
        pcm_items.clone(),
    );
    let preview = sim::lib_stream_file::stream_to_cassette(
        &pcm_stream,
        TransportProfile::lan_buffered_audio_preview(),
    )
    .unwrap();
    let preview_stream = sim::lib_stream_file::cassette_expr_to_stream(&preview.to_expr()).unwrap();
    assert_eq!(preview_stream.take_packets(4).unwrap(), pcm_items);
    let mut web = FixtureTransport::new()
        .with_finite_stream(preview.metadata().clone(), preview.items().unwrap());
    let inspector = web.stream_subscribe(preview.metadata().id()).unwrap();
    assert_eq!(inspector.buffered, 2);
    assert_eq!(
        web.stream_read(preview.metadata().id(), 4).unwrap().len(),
        2
    );
    assert_publishable_fixture(
        &mut serde_cx,
        &preview,
        "fixtures/streams/golden/web-preview.simcassette",
    );
}

/// Asserts a cassette is a publishable golden fixture and survives serialization.
///
/// First checks the structural invariants a cassette must satisfy to be
/// published as a golden fixture at `target_path` (finite trace, sequenced
/// envelopes, replay- or preview-only transport, and no unredacted payload or
/// host-device name). No `.simcassette` file is read; `validate_golden_fixture`
/// validates the in-memory cassette against the target path. Then it exercises
/// the real regression guard: the cassette's `to_expr`/`from_expr` serialization
/// round-trips losslessly through the general-purpose lisp codec.
fn assert_publishable_fixture(
    cx: &mut Cx,
    cassette: &sim::lib_stream_core::StreamCassette,
    target_path: &str,
) {
    cassette
        .validate_golden_fixture(target_path)
        .unwrap_or_else(|err| panic!("{target_path}: cassette fails publish invariants: {err:?}"));
    assert_cassette_round_trips(cx, cassette, target_path);
}

/// Round-trips a cassette through `to_expr` -> lisp codec -> `from_expr`.
///
/// Asserts the serialized form survives the codec byte-for-structure and that
/// deserialization reconstructs an equal cassette form. A regression in the
/// cassette format or the lisp codec fails this guard.
fn assert_cassette_round_trips(
    cx: &mut Cx,
    cassette: &sim::lib_stream_core::StreamCassette,
    label: &str,
) {
    let expr = cassette.to_expr();
    let encoded = encode_once(cx, &q("codec", "lisp"), &expr);
    let decoded = decode_once(cx, &q("codec", "lisp"), encoded);
    assert_eq!(
        decoded, expr,
        "{label}: cassette serialization did not survive the lisp codec"
    );
    let restored = sim::lib_stream_core::StreamCassette::from_expr(&decoded)
        .unwrap_or_else(|err| panic!("{label}: cassette failed to deserialize: {err:?}"));
    assert_eq!(
        restored.to_expr(),
        expr,
        "{label}: cassette did not deserialize to an equal form"
    );
}

#[test]
fn stream_security_capabilities_limits_and_redaction_are_conformant() {
    use sim::lib_stream_core::{
        ClockDomain, StreamCassette, StreamItem, StreamMedia, StreamPacket, StreamRedactionFinding,
        StreamRemoteLimits, StreamSecurityPolicy, StreamStats, TransportProfile,
        stream_redaction_finding_symbols, stream_security_capability_names,
    };

    assert_eq!(
        stream_security_capability_names()
            .into_iter()
            .map(|capability| capability.as_str().to_owned())
            .collect::<Vec<_>>(),
        vec![
            "stream.open",
            "stream.read",
            "stream.push",
            "stream.cancel",
            "stream.stats",
            "stream.remote.preview",
            "stream.remote.render",
            "stream.lan.midi",
            "stream.host.device",
            "stream.remote.network",
        ]
    );
    assert_eq!(
        stream_redaction_finding_symbols()
            .into_iter()
            .map(|symbol| symbol.as_qualified_str())
            .collect::<Vec<_>>(),
        vec![
            "stream/redaction/private-path",
            "stream/redaction/host-name",
            "stream/redaction/absolute-path",
            "stream/redaction/credential",
            "stream/redaction/patch-bank-payload",
            "stream/redaction/large-binary-data",
        ]
    );

    let limits = StreamRemoteLimits::default();
    assert_eq!(limits.max_frame_payload_bytes, 1024 * 1024);
    assert_eq!(limits.max_stream_frames, 1024);
    assert_eq!(limits.max_inflight_frames, 64);
    assert_eq!(limits.max_duration_ms, 60_000);
    assert_eq!(limits.max_rate_hz, 120);
    assert_eq!(limits.max_binary_payload_bytes, 256 * 1024);
    assert_eq!(limits.effective_frame_limit(), 1024);
    assert!(
        limits
            .validate_profile(&TransportProfile::remote_stream_fabric())
            .is_ok()
    );
    assert!(
        limits
            .validate_profile(&TransportProfile::realtime_local_audio())
            .is_err()
    );

    let policy = StreamSecurityPolicy::default();
    assert_eq!(
        policy.finding_for_expr(&Expr::String("token=abc123".to_owned())),
        Some(StreamRedactionFinding::Credential)
    );
    assert_eq!(
        policy.finding_for_expr(&Expr::String("https://sim.example/stream".to_owned())),
        Some(StreamRedactionFinding::HostName)
    );

    let cassette = StreamCassette::from_items(
        conformance_metadata(
            "stream/conformance-security",
            StreamMedia::Data,
            ClockDomain::ServerFrame,
        ),
        vec![StreamItem::new(StreamPacket::data(
            q("stream/data", "expr"),
            Expr::Map(vec![
                (
                    Expr::Symbol(Symbol::new("path")),
                    Expr::String("private-path=session.mid".to_owned()),
                ),
                (
                    Expr::Symbol(Symbol::new("bank")),
                    Expr::String("dx7 patch-bank payload".to_owned()),
                ),
            ]),
        ))],
        TransportProfile::remote_stream_fabric(),
        StreamStats::default(),
    )
    .unwrap();

    // The unredacted cassette must fail the publish invariants (it carries a
    // private path and a patch-bank payload). Redacting it produces a
    // publishable golden fixture that also survives the serialization round-trip.
    assert!(
        cassette
            .validate_golden_fixture("fixtures/streams/golden/conformance-security.simcassette")
            .is_err()
    );
    let mut serde_cx = cx();
    assert_publishable_fixture(
        &mut serde_cx,
        &cassette.redacted().unwrap(),
        "fixtures/streams/golden/conformance-security.simcassette",
    );
}

fn conformance_metadata(
    id: &str,
    media: sim::lib_stream_core::StreamMedia,
    clock: sim::lib_stream_core::ClockDomain,
) -> sim::lib_stream_core::StreamMetadata {
    sim::lib_stream_core::StreamMetadata::new(
        Symbol::new(id),
        media,
        sim::lib_stream_core::StreamDirection::Source,
        clock.symbol(),
        sim::lib_stream_core::BufferPolicy::bounded(8).unwrap(),
    )
}

fn midi_item(ticks: i64) -> sim::lib_stream_core::StreamItem {
    sim::lib_stream_core::StreamItem::new(sim::lib_stream_core::StreamPacket::Midi(
        sim::lib_stream_core::MidiPacket::new(vec![
            sim::lib_stream_core::MidiPacketEvent::new(ticks, 480, vec![0x90, 60, 100]).unwrap(),
        ])
        .unwrap(),
    ))
}

fn pcm_item(value: f32) -> sim::lib_stream_core::StreamItem {
    sim::lib_stream_core::StreamItem::new(sim::lib_stream_core::StreamPacket::Pcm(
        sim::lib_stream_core::PcmPacket::f32(1, 1, vec![value]).unwrap(),
    ))
}

fn normalized_conformance_contract() -> String {
    CONFORMANCE_CONTRACT
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
