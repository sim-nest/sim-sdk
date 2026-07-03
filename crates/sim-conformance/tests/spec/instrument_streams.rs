use sim::{
    kernel::{Expr, Symbol},
    lib_audio_graph_core::{PrepareConfig, ProcessBlock, Processor},
    lib_audio_graph_live::{
        LiveGraphConfig, LiveGraphRunner, LiveStreamLane, LiveTransportClock,
        validate_realtime_local_audio_profile,
    },
    lib_daw_session::instrument_session_fixture,
    lib_music_synth::daw::{
        ALL_LOCAL_PLACEMENT_RECIPE_PATH, BROWSER_WASM_LOCAL_RECIPE_PATH,
        DX7_LOCAL_STREAM_RECIPE_PATH, GENERIC_INSTRUMENT_STREAM_RECIPE_PATH,
        MODULAR_LOCAL_STREAM_RECIPE_PATH, VOICE_LOCAL_LAN_PREVIEW_RECIPE_PATH,
        VOICE_LOCAL_SERVER_FX_RECIPE_PATH, instrument_placement_recipe_specs,
        instrument_recipe_paths, instrument_stream_recipe_specs,
    },
    lib_stream_core::{
        LatencyClass, MidiPacket, MidiPacketEvent, PcmPacket, StreamCapability, StreamItem,
        StreamPacket, TransportProfile,
    },
    lib_view_daw::stream_packet_preview_view,
};

#[derive(Clone, Debug)]
struct CountingProcessor;

impl Processor for CountingProcessor {
    fn prepare(&mut self, _cfg: PrepareConfig) {}

    fn reset(&mut self) {}

    fn process(&mut self, block: &mut ProcessBlock<'_>) {
        let offset = block.in_events.len() as f32;
        let frames = block.frames as usize;
        for (input, output) in block.in_audio.iter().zip(block.out_audio.iter_mut()) {
            for (source, target) in input.iter().zip(output.iter_mut()).take(frames) {
                *target = *source + offset;
            }
        }
    }
}

#[test]
fn instrument_recipe_descriptors_record_paths_streams_sites_and_artifacts() {
    assert_eq!(
        instrument_recipe_paths(),
        vec![
            GENERIC_INSTRUMENT_STREAM_RECIPE_PATH,
            DX7_LOCAL_STREAM_RECIPE_PATH,
            MODULAR_LOCAL_STREAM_RECIPE_PATH,
            ALL_LOCAL_PLACEMENT_RECIPE_PATH,
            VOICE_LOCAL_SERVER_FX_RECIPE_PATH,
            VOICE_LOCAL_LAN_PREVIEW_RECIPE_PATH,
            BROWSER_WASM_LOCAL_RECIPE_PATH,
        ]
    );

    let stream_specs = instrument_stream_recipe_specs();
    assert_eq!(stream_specs.len(), 3);
    assert!(stream_specs[0].stream_ids().contains(&"stream/live/patch"));
    assert!(
        stream_specs[0]
            .stream_ids()
            .contains(&"stream/live/audio-output")
    );
    assert!(
        stream_specs[0]
            .stream_ids()
            .contains(&"stream/live/preview")
    );
    assert!(
        stream_specs
            .iter()
            .filter(|spec| spec.refuses_remote_hard_realtime())
            .map(|spec| spec.id())
            .eq(["dx7-local-stream", "modular-local-stream"])
    );
    for spec in stream_specs {
        assert!(!spec.latency_budget().is_empty());
        assert!(
            spec.artifact_names()
                .iter()
                .any(|artifact| artifact.ends_with(".sha256")),
            "{} must name a hashable artifact",
            spec.id()
        );
    }

    let placement_specs = instrument_placement_recipe_specs();
    assert_eq!(placement_specs.len(), 4);
    assert!(
        placement_specs
            .iter()
            .any(|spec| spec.site_map().contains("stream/site/host-callback"))
    );
    assert!(
        placement_specs
            .iter()
            .any(|spec| spec.site_map().contains("stream/site/process"))
    );
    assert!(placement_specs.iter().any(|spec| {
        spec.site_map()
            .contains("stream/profile/lan-buffered-audio-preview")
    }));
    assert!(
        placement_specs
            .iter()
            .any(|spec| spec.site_map().contains("stream/site/browser-wasm"))
    );
    for spec in placement_specs {
        assert!(spec.path().ends_with("/recipe.toml"));
        assert!(spec.stream_ids().contains(&"stream/live/audio-output"));
        assert!(!spec.latency_budget().is_empty());
        assert!(
            spec.artifact_names()
                .iter()
                .any(|artifact| artifact.ends_with(".sha256")),
            "{} must name a hashable artifact",
            spec.id()
        );
    }
}

#[test]
fn instrument_midi_control_stream_runs_offline_live_and_web_preview_paths() {
    let session = instrument_session_fixture();
    let render = session.render_offline(4).expect("offline render");

    assert_eq!(render.tracks_rendered(), 1);
    assert_eq!(render.clips_rendered(), 1);
    assert!(
        session
            .routes()
            .iter()
            .any(|route| route.kind().as_str() == "midi")
    );
    assert!(
        session
            .routes()
            .iter()
            .any(|route| route.kind().as_str() == "parameter-automation")
    );

    let mut runner = LiveGraphRunner::new(
        CountingProcessor,
        LiveGraphConfig::stereo(48_000, 4).unwrap(),
    )
    .unwrap();
    let mut output = [0.0; 8];
    runner.enqueue_midi_short(0, &[0x90, 60, 100]).unwrap();
    runner.enqueue_param_set(0, 7, 0.5).unwrap();
    let report = runner
        .process_interleaved_f32(
            Some(&[0.0; 8]),
            &mut output,
            4,
            LiveTransportClock::sample_frame(48_000)
                .unwrap()
                .transport_at(0, true),
        )
        .unwrap();

    assert_eq!(report.control_events(), 2);
    assert_eq!(output, [2.0; 8]);
    let live_stream_ids = LiveStreamLane::all()
        .iter()
        .map(|lane| lane.stream_id().as_qualified_str())
        .collect::<Vec<_>>();
    for stream_id in [
        "stream/live/audio-output",
        "stream/live/midi",
        "stream/live/parameter",
        "stream/live/diagnostic",
    ] {
        assert!(
            live_stream_ids.iter().any(|live| live == stream_id),
            "missing live stream id {stream_id}"
        );
    }

    let remote = TransportProfile::new(
        Symbol::qualified("stream/profile", "remote-hard-realtime"),
        LatencyClass::RemoteCollaboration,
        vec![StreamCapability::Remote, StreamCapability::Bounded],
    )
    .unwrap();
    let err = validate_realtime_local_audio_profile(&remote).unwrap_err();
    assert!(err.to_string().contains("remote"));

    let preview = stream_packet_preview_view(&[
        StreamItem::new(StreamPacket::Midi(
            MidiPacket::new(vec![
                MidiPacketEvent::new(0, 480, vec![0x90, 60, 100]).unwrap(),
            ])
            .unwrap(),
        )),
        StreamItem::new(StreamPacket::data(
            Symbol::qualified("stream/live", "parameter"),
            Expr::Nil,
        )),
        StreamItem::new(StreamPacket::Pcm(
            PcmPacket::f32(2, 2, vec![0.0, 0.0, 0.0, 0.0]).unwrap(),
        )),
    ]);

    assert!(contains_role(&preview, "stream-packet-preview"));
    assert!(contains_symbol(&preview, "stream/packet", "midi"));
    assert!(contains_symbol(&preview, "stream/live", "parameter"));
    assert!(contains_symbol(&preview, "stream/packet", "pcm"));
}

fn contains_role(expr: &Expr, role: &str) -> bool {
    field(expr, "role") == Some(Expr::Symbol(Symbol::new(role)))
        || expr_children(expr)
            .iter()
            .any(|child| contains_role(child, role))
}

fn contains_symbol(expr: &Expr, namespace: &str, name: &str) -> bool {
    match expr {
        Expr::Symbol(symbol)
            if symbol.namespace.as_deref() == Some(namespace) && symbol.name.as_ref() == name =>
        {
            true
        }
        _ => expr_children(expr)
            .iter()
            .any(|child| contains_symbol(child, namespace, name)),
    }
}

fn field(map: &Expr, name: &str) -> Option<Expr> {
    let Expr::Map(entries) = map else { return None };
    entries.iter().find_map(|(key, value)| {
        matches!(key, Expr::Symbol(symbol) if symbol.name.as_ref() == name).then(|| value.clone())
    })
}

fn expr_children(expr: &Expr) -> Vec<&Expr> {
    match expr {
        Expr::List(items) | Expr::Vector(items) | Expr::Set(items) | Expr::Block(items) => {
            items.iter().collect()
        }
        Expr::Map(entries) => entries
            .iter()
            .flat_map(|(key, value)| [key, value])
            .collect(),
        Expr::Call { operator, args } => std::iter::once(operator.as_ref()).chain(args).collect(),
        Expr::Infix { left, right, .. } => vec![left, right],
        Expr::Prefix { arg, .. } | Expr::Postfix { arg, .. } => vec![arg],
        Expr::Quote { expr, .. } | Expr::Annotated { expr, .. } => vec![expr],
        Expr::Extension { payload, .. } => vec![payload],
        Expr::Nil
        | Expr::Bool(_)
        | Expr::Number(_)
        | Expr::Symbol(_)
        | Expr::Local(_)
        | Expr::String(_)
        | Expr::Bytes(_) => Vec::new(),
    }
}
