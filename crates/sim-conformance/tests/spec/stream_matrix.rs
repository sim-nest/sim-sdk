use std::time::Duration;

use sim::{
    kernel::{Expr, Ref, Symbol},
    lib_server::FrameEnvelope,
    lib_stream_core::{
        BufferOverflowPolicy, BufferPolicy, ClockDomain, PushResult, StreamDiagnostic,
        StreamDirection, StreamEnvelope, StreamItem, StreamMedia, StreamMetadata, StreamPacket,
        StreamValue, TransportProfile, stream_remote_network_capability,
    },
    lib_stream_fabric::{
        event_buffer_to_stream, refused_profile_diagnostic_kind, stream_frames_to_stream,
        stream_to_frames_with_profile,
    },
    lib_stream_host::{
        FakeBackend, HostBackend, HostDirection, HostStreamConfigRequest, fake_backend_symbol,
    },
    lib_web_bridge::{BrowserStreamStatus, FixtureTransport, SessionStatus, Transport},
};

use super::{conformance_metadata, midi_item, pcm_item};
use crate::support::{cx, q};

pub(crate) const MATRIX_PATH: &str = "crates/sim-conformance/tests/spec/stream_matrix.rs";

const ALL_FIXTURES: [StreamFixture; 10] = [
    StreamFixture::Pcm,
    StreamFixture::Midi,
    StreamFixture::Diagnostic,
    StreamFixture::Data,
    StreamFixture::Cancel,
    StreamFixture::Done,
    StreamFixture::Overflow,
    StreamFixture::Timeout,
    StreamFixture::Reconnect,
    StreamFixture::RefusedProfile,
];

const PCM: u16 = StreamFixture::Pcm.bit();
const MIDI: u16 = StreamFixture::Midi.bit();
const DIAG: u16 = StreamFixture::Diagnostic.bit();
const DATA: u16 = StreamFixture::Data.bit();
const CANCEL: u16 = StreamFixture::Cancel.bit();
const DONE: u16 = StreamFixture::Done.bit();
const OVERFLOW: u16 = StreamFixture::Overflow.bit();
const TIMEOUT: u16 = StreamFixture::Timeout.bit();
const RECONNECT: u16 = StreamFixture::Reconnect.bit();
const REFUSED: u16 = StreamFixture::RefusedProfile.bit();

const L0_SUPPORTED: FixtureSet =
    FixtureSet(PCM | MIDI | DIAG | DATA | CANCEL | DONE | OVERFLOW | TIMEOUT);
const L0_SKIPPED: FixtureSet = FixtureSet(RECONNECT | REFUSED);
const L1_SUPPORTED: FixtureSet = FixtureSet(PCM | MIDI | DIAG | DATA | DONE);
const L1_SKIPPED: FixtureSet = FixtureSet(CANCEL | OVERFLOW | TIMEOUT | RECONNECT | REFUSED);
const L2_SUPPORTED: FixtureSet = FixtureSet(PCM | DIAG | CANCEL | OVERFLOW | TIMEOUT);
const L2_SKIPPED: FixtureSet = FixtureSet(MIDI | DATA | DONE | RECONNECT | REFUSED);
const L3_SUPPORTED: FixtureSet = FixtureSet(PCM | MIDI | DATA | CANCEL | OVERFLOW);
const L3_SKIPPED: FixtureSet = FixtureSet(DIAG | DONE | TIMEOUT | RECONNECT | REFUSED);
const L4_SUPPORTED: FixtureSet = FixtureSet(PCM | MIDI | DIAG | DATA | DONE | REFUSED);
const L4_SKIPPED: FixtureSet = FixtureSet(CANCEL | OVERFLOW | TIMEOUT | RECONNECT);
const L5_SUPPORTED: FixtureSet = FixtureSet(PCM | MIDI | DIAG | DATA | DONE);
const L5_SKIPPED: FixtureSet = FixtureSet(CANCEL | OVERFLOW | TIMEOUT | RECONNECT | REFUSED);
const L6_SUPPORTED: FixtureSet = FixtureSet(PCM | DIAG | DATA | CANCEL | OVERFLOW | RECONNECT);
const L6_SKIPPED: FixtureSet = FixtureSet(MIDI | DONE | TIMEOUT | REFUSED);
const L7_SUPPORTED: FixtureSet = FixtureSet(DIAG | DATA | DONE | REFUSED);
const L7_SKIPPED: FixtureSet = FixtureSet(PCM | MIDI | CANCEL | OVERFLOW | TIMEOUT | RECONNECT);
const ALL_FIXTURE_SET: FixtureSet =
    FixtureSet(PCM | MIDI | DIAG | DATA | CANCEL | DONE | OVERFLOW | TIMEOUT | RECONNECT | REFUSED);

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum StreamFixture {
    Pcm,
    Midi,
    Diagnostic,
    Data,
    Cancel,
    Done,
    Overflow,
    Timeout,
    Reconnect,
    RefusedProfile,
}

impl StreamFixture {
    const fn bit(self) -> u16 {
        1 << self as u8
    }

    fn label(self) -> &'static str {
        match self {
            Self::Pcm => "PCM",
            Self::Midi => "MIDI",
            Self::Diagnostic => "diagnostics",
            Self::Data => "data",
            Self::Cancel => "cancel",
            Self::Done => "done",
            Self::Overflow => "overflow",
            Self::Timeout => "timeout",
            Self::Reconnect => "reconnect",
            Self::RefusedProfile => "refused profile",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FixtureSet(u16);

impl FixtureSet {
    fn contains(self, fixture: StreamFixture) -> bool {
        self.0 & fixture.bit() != 0
    }

    fn is_disjoint(self, other: Self) -> bool {
        self.0 & other.0 == 0
    }

    fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    fn labels(self) -> String {
        ALL_FIXTURES
            .into_iter()
            .filter(|fixture| self.contains(*fixture))
            .map(StreamFixture::label)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[derive(Clone, Copy)]
enum MatrixRunner {
    Memory,
    Coroutine,
    Queue,
    Host,
    Fabric,
    Browser,
}

struct MatrixRow {
    layer: &'static str,
    profile_label: &'static str,
    runner: MatrixRunner,
    profile: fn() -> TransportProfile,
    supported: FixtureSet,
    skipped: FixtureSet,
}

#[test]
fn stream_conformance_matrix_records_layers_fixtures_and_support_table() {
    assert_eq!(
        MATRIX_PATH,
        "crates/sim-conformance/tests/spec/stream_matrix.rs"
    );
    assert_eq!(
        matrix().map(|row| row.layer),
        [
            "L0-memory",
            "L1-coroutine",
            "L2-thread",
            "L3-host",
            "L4-process",
            "L5-lan",
            "L6-browser",
            "L7-wan",
        ]
    );
    assert_eq!(
        ALL_FIXTURES.map(StreamFixture::label),
        [
            "PCM",
            "MIDI",
            "diagnostics",
            "data",
            "cancel",
            "done",
            "overflow",
            "timeout",
            "reconnect",
            "refused profile",
        ]
    );

    let table = support_table();
    assert_eq!(table.len(), 8);
    assert!(
        table
            .iter()
            .any(|row| row.contains("L0-memory | memory-local | PCM, MIDI"))
    );
    assert!(
        table
            .iter()
            .any(|row| row.contains("L7-wan | remote-stream-fabric | diagnostics, data"))
    );
    assert_matrix_partitions_all_fixtures();
}

#[test]
fn stream_matrix_runs_supported_fixtures_and_emits_skip_diagnostics() {
    for row in matrix() {
        for fixture in ALL_FIXTURES {
            if row.supported.contains(fixture) {
                run_fixture(&row, fixture);
            } else {
                assert!(row.skipped.contains(fixture));
                let diagnostic = skip_diagnostic(&row, fixture);
                assert_eq!(
                    diagnostic.kind(),
                    &Symbol::qualified("stream/conformance", "Skip")
                );
                assert!(diagnostic.message().contains(row.layer));
                assert!(diagnostic.message().contains(row.profile_label));
                assert!(diagnostic.message().contains(fixture.label()));
            }
        }
    }
}

fn matrix() -> [MatrixRow; 8] {
    [
        MatrixRow {
            layer: "L0-memory",
            profile_label: "memory-local",
            runner: MatrixRunner::Memory,
            profile: TransportProfile::memory_local,
            supported: L0_SUPPORTED,
            skipped: L0_SKIPPED,
        },
        MatrixRow {
            layer: "L1-coroutine",
            profile_label: "memory-event-projection",
            runner: MatrixRunner::Coroutine,
            profile: TransportProfile::memory_local,
            supported: L1_SUPPORTED,
            skipped: L1_SKIPPED,
        },
        MatrixRow {
            layer: "L2-thread",
            profile_label: "bounded-push-queue",
            runner: MatrixRunner::Queue,
            profile: TransportProfile::memory_local,
            supported: L2_SUPPORTED,
            skipped: L2_SKIPPED,
        },
        MatrixRow {
            layer: "L3-host",
            profile_label: "fake-host-callback",
            runner: MatrixRunner::Host,
            profile: TransportProfile::realtime_local_audio,
            supported: L3_SUPPORTED,
            skipped: L3_SKIPPED,
        },
        MatrixRow {
            layer: "L4-process",
            profile_label: "remote-stream-fabric",
            runner: MatrixRunner::Fabric,
            profile: TransportProfile::remote_stream_fabric,
            supported: L4_SUPPORTED,
            skipped: L4_SKIPPED,
        },
        MatrixRow {
            layer: "L5-lan",
            profile_label: "lan-midi-control",
            runner: MatrixRunner::Fabric,
            profile: TransportProfile::lan_midi_control,
            supported: L5_SUPPORTED,
            skipped: L5_SKIPPED,
        },
        MatrixRow {
            layer: "L6-browser",
            profile_label: "fixture-browser-bridge",
            runner: MatrixRunner::Browser,
            profile: TransportProfile::buffered_pcm_preview,
            supported: L6_SUPPORTED,
            skipped: L6_SKIPPED,
        },
        MatrixRow {
            layer: "L7-wan",
            profile_label: "remote-stream-fabric",
            runner: MatrixRunner::Fabric,
            profile: TransportProfile::remote_stream_fabric,
            supported: L7_SUPPORTED,
            skipped: L7_SKIPPED,
        },
    ]
}

fn support_table() -> Vec<String> {
    matrix()
        .into_iter()
        .map(|row| {
            format!(
                "{} | {} | {} | {}",
                row.layer,
                row.profile_label,
                row.supported.labels(),
                row.skipped.labels(),
            )
        })
        .collect()
}

fn assert_matrix_partitions_all_fixtures() {
    let mut covered_supported = FixtureSet(0);
    for row in matrix() {
        assert!(
            row.supported.is_disjoint(row.skipped),
            "{} has overlapping support and skip entries",
            row.layer
        );
        assert_eq!(
            row.supported.union(row.skipped),
            ALL_FIXTURE_SET,
            "{} does not account for every fixture",
            row.layer
        );
        covered_supported = covered_supported.union(row.supported);
    }
    assert_eq!(covered_supported, ALL_FIXTURE_SET);
}

fn run_fixture(row: &MatrixRow, fixture: StreamFixture) {
    match fixture {
        StreamFixture::Pcm
        | StreamFixture::Midi
        | StreamFixture::Diagnostic
        | StreamFixture::Data => run_packet_fixture(row, fixture),
        StreamFixture::Cancel => run_cancel_fixture(row),
        StreamFixture::Done => run_done_fixture(row),
        StreamFixture::Overflow => run_overflow_fixture(row),
        StreamFixture::Timeout => run_timeout_fixture(),
        StreamFixture::Reconnect => run_reconnect_fixture(),
        StreamFixture::RefusedProfile => run_refused_profile_fixture(),
    }
}

fn run_packet_fixture(row: &MatrixRow, fixture: StreamFixture) {
    let item = fixture_item(fixture);
    match row.runner {
        MatrixRunner::Memory => memory_roundtrip(row, item),
        MatrixRunner::Coroutine => coroutine_roundtrip(row, item),
        MatrixRunner::Queue => queue_roundtrip(row, item),
        MatrixRunner::Host => host_roundtrip(item),
        MatrixRunner::Fabric => fabric_roundtrip(row, item),
        MatrixRunner::Browser => browser_roundtrip(row, item),
    }
}

fn run_cancel_fixture(row: &MatrixRow) {
    match row.runner {
        MatrixRunner::Host => {
            let opened = FakeBackend::new()
                .open(FakeBackend::data_request(8).unwrap())
                .unwrap();
            opened.cancel().unwrap();
            assert!(opened.queue().stats().unwrap().cancelled);
        }
        MatrixRunner::Browser => {
            let metadata = matrix_metadata(row, StreamMedia::Data);
            let stream_id = metadata.id().clone();
            let mut transport = FixtureTransport::new().with_push_stream(metadata);
            transport.stream_cancel(&stream_id).unwrap();
            let inspector = transport.stream_inspector(&stream_id).unwrap();
            assert_eq!(inspector.status, BrowserStreamStatus::Cancelled);
            assert!(inspector.stats.cancelled);
        }
        _ => {
            let stream = StreamValue::push(matrix_metadata(row, StreamMedia::Data));
            stream.cancel().unwrap();
            assert!(stream.stats().unwrap().cancelled);
            assert!(stream.is_done().unwrap());
        }
    }
}

fn run_done_fixture(row: &MatrixRow) {
    let item = fixture_item(StreamFixture::Data);
    match row.runner {
        MatrixRunner::Coroutine => {
            let metadata = matrix_metadata(row, StreamMedia::Data);
            let mut run_cx = cx();
            let stream = StreamValue::pull(metadata.clone(), vec![item.clone()]);
            let events = stream
                .run_events(
                    &mut run_cx,
                    Ref::Symbol(Symbol::qualified("stream/conformance", "done-run")),
                    0,
                )
                .unwrap();
            let replay = event_buffer_to_stream(&mut run_cx, metadata, events).unwrap();
            assert_eq!(replay.take_packets(2).unwrap(), vec![item]);
            assert!(replay.is_done().unwrap());
        }
        MatrixRunner::Fabric => {
            let remote = fabric_stream(row, item.clone());
            assert_eq!(remote.take_packets(2).unwrap(), vec![item]);
            assert!(remote.is_done().unwrap());
        }
        MatrixRunner::Browser => {
            let metadata = matrix_metadata(row, StreamMedia::Data);
            let stream_id = metadata.id().clone();
            let mut transport =
                FixtureTransport::new().with_finite_stream(metadata, vec![item.clone()]);
            assert_eq!(transport.stream_read(&stream_id, 4).unwrap(), vec![item]);
            let inspector = transport.stream_inspector(&stream_id).unwrap();
            assert_eq!(inspector.status, BrowserStreamStatus::Ended);
        }
        _ => {
            let stream = StreamValue::pull(matrix_metadata(row, StreamMedia::Data), vec![item]);
            assert_eq!(stream.take_packets(2).unwrap().len(), 1);
            assert!(stream.is_done().unwrap());
        }
    }
}

fn run_overflow_fixture(row: &MatrixRow) {
    let metadata = overflow_metadata(row);
    let first = data_item("first");
    let second = data_item("second");
    match row.runner {
        MatrixRunner::Host => {
            let opened = FakeBackend::new().open(host_overflow_request()).unwrap();
            assert_eq!(
                opened.queue().callback_item(first).unwrap(),
                PushResult::Accepted
            );
            assert!(matches!(
                opened.queue().callback_item(second).unwrap(),
                PushResult::Rejected(_)
            ));
            assert_eq!(opened.queue().stats().unwrap().overflow_errors, 1);
        }
        MatrixRunner::Browser => {
            let stream_id = metadata.id().clone();
            let mut transport = FixtureTransport::new().with_push_stream(metadata.clone());
            let first = StreamEnvelope::from_item(&metadata, 0, &first).unwrap();
            let second = StreamEnvelope::from_item(&metadata, 1, &second).unwrap();
            assert_eq!(
                transport.stream_push(&stream_id, first).unwrap(),
                PushResult::Accepted
            );
            assert!(matches!(
                transport.stream_push(&stream_id, second).unwrap(),
                PushResult::Rejected(_)
            ));
            let inspector = transport.stream_inspector(&stream_id).unwrap();
            assert_eq!(inspector.status, BrowserStreamStatus::BufferOverflow);
        }
        _ => {
            let stream = StreamValue::push(metadata);
            assert_eq!(stream.push_packet(first).unwrap(), PushResult::Accepted);
            assert!(matches!(
                stream.push_packet(second).unwrap(),
                PushResult::Rejected(_)
            ));
            assert_eq!(stream.stats().unwrap().overflow_errors, 1);
        }
    }
}

fn run_timeout_fixture() {
    let stream = StreamValue::push(StreamMetadata::new(
        Symbol::new("stream/conformance-timeout"),
        StreamMedia::Data,
        StreamDirection::Source,
        ClockDomain::ServerFrame.symbol(),
        BufferPolicy::bounded(1).unwrap(),
    ));
    assert!(
        stream
            .next_packet_timeout(Duration::from_millis(0))
            .unwrap()
            .is_none()
    );
    assert_eq!(stream.stats().unwrap().timed_out, 1);
}

fn run_reconnect_fixture() {
    let mut transport = FixtureTransport::new();
    assert_eq!(transport.status(), SessionStatus::Connected);
    transport.disconnect();
    assert_eq!(transport.status(), SessionStatus::Disconnected);
    transport.begin_reconnect();
    assert_eq!(transport.status(), SessionStatus::Reconnecting);
    transport.reconnect();
    assert_eq!(transport.status(), SessionStatus::Connected);
}

fn run_refused_profile_fixture() {
    let mut run_cx = cx();
    run_cx.grant(stream_remote_network_capability());
    let metadata = conformance_metadata(
        "stream/conformance-refused",
        StreamMedia::Pcm,
        ClockDomain::Sample,
    );
    let stream = StreamValue::pull(metadata, vec![pcm_item(0.25)]);
    let frames = stream_to_frames_with_profile(
        &mut run_cx,
        &stream,
        q("codec", "lisp"),
        FrameEnvelope::default(),
        TransportProfile::realtime_local_audio(),
    )
    .unwrap();
    let remote = stream_frames_to_stream(&mut run_cx, &frames).unwrap();
    let item = remote.next_packet().unwrap().unwrap();
    let StreamPacket::Diagnostic(packet) = item.packet() else {
        panic!("expected refused profile diagnostic");
    };
    assert_eq!(packet.kind(), &refused_profile_diagnostic_kind());
    assert!(packet.message().contains("realtime-local-audio"));
}

fn memory_roundtrip(row: &MatrixRow, item: StreamItem) {
    let stream = StreamValue::pull(
        matrix_metadata(row, item.packet().media()),
        vec![item.clone()],
    );
    assert_eq!(stream.take_packets(2).unwrap(), vec![item]);
    assert!(stream.is_done().unwrap());
}

fn coroutine_roundtrip(row: &MatrixRow, item: StreamItem) {
    let metadata = matrix_metadata(row, item.packet().media());
    let mut run_cx = cx();
    let stream = StreamValue::pull(metadata.clone(), vec![item.clone()]);
    let events = stream
        .run_events(
            &mut run_cx,
            Ref::Symbol(Symbol::qualified("stream/conformance", "coroutine-run")),
            0,
        )
        .unwrap();
    let replay = event_buffer_to_stream(&mut run_cx, metadata, events).unwrap();
    assert_eq!(replay.take_packets(2).unwrap(), vec![item]);
}

fn queue_roundtrip(row: &MatrixRow, item: StreamItem) {
    let stream = StreamValue::push(matrix_metadata(row, item.packet().media()));
    assert_eq!(
        stream.push_packet(item.clone()).unwrap(),
        PushResult::Accepted
    );
    stream.close_push().unwrap();
    assert_eq!(stream.take_packets(2).unwrap(), vec![item]);
    assert!(stream.is_done().unwrap());
}

fn host_roundtrip(item: StreamItem) {
    let opened = FakeBackend::new()
        .open(host_request(item.packet().media()))
        .unwrap();
    assert_eq!(
        opened.queue().callback_item(item.clone()).unwrap(),
        PushResult::Accepted
    );
    assert_eq!(opened.queue().drain(2).unwrap(), vec![item]);
}

fn fabric_roundtrip(row: &MatrixRow, item: StreamItem) {
    let remote = fabric_stream(row, item.clone());
    assert_eq!(remote.take_packets(2).unwrap(), vec![item]);
}

fn browser_roundtrip(row: &MatrixRow, item: StreamItem) {
    let metadata = matrix_metadata(row, item.packet().media());
    let stream_id = metadata.id().clone();
    let mut transport = FixtureTransport::new().with_finite_stream(metadata, vec![item.clone()]);
    assert_eq!(transport.stream_subscribe(&stream_id).unwrap().buffered, 1);
    assert_eq!(transport.stream_read(&stream_id, 2).unwrap(), vec![item]);
}

fn fabric_stream(row: &MatrixRow, item: StreamItem) -> StreamValue {
    let mut run_cx = cx();
    run_cx.grant(stream_remote_network_capability());
    let stream = StreamValue::pull(matrix_metadata(row, item.packet().media()), vec![item]);
    let frames = stream_to_frames_with_profile(
        &mut run_cx,
        &stream,
        q("codec", "lisp"),
        FrameEnvelope::default(),
        (row.profile)(),
    )
    .unwrap();
    stream_frames_to_stream(&mut run_cx, &frames).unwrap()
}

fn skip_diagnostic(row: &MatrixRow, fixture: StreamFixture) -> StreamDiagnostic {
    StreamDiagnostic::new(
        Symbol::qualified("stream/conformance", "Skip"),
        format!(
            "{} profile {} does not support {} fixture",
            row.layer,
            row.profile_label,
            fixture.label()
        ),
    )
}

fn fixture_item(fixture: StreamFixture) -> StreamItem {
    match fixture {
        StreamFixture::Pcm => pcm_item(0.5),
        StreamFixture::Midi => midi_item(0),
        StreamFixture::Diagnostic => {
            StreamItem::new(StreamPacket::Diagnostic(StreamDiagnostic::new(
                Symbol::qualified("stream/conformance", "DiagnosticFixture"),
                "diagnostic fixture",
            )))
        }
        StreamFixture::Data => data_item("payload"),
        StreamFixture::Cancel
        | StreamFixture::Done
        | StreamFixture::Overflow
        | StreamFixture::Timeout
        | StreamFixture::Reconnect
        | StreamFixture::RefusedProfile => unreachable!("not a packet fixture"),
    }
}

fn data_item(payload: &str) -> StreamItem {
    StreamItem::new(StreamPacket::data(
        Symbol::qualified("stream/data", "expr"),
        Expr::String(payload.to_owned()),
    ))
}

fn matrix_metadata(row: &MatrixRow, media: StreamMedia) -> StreamMetadata {
    conformance_metadata(
        &format!("stream/conformance-{}", row.layer),
        media,
        clock_for(row, media),
    )
}

fn overflow_metadata(row: &MatrixRow) -> StreamMetadata {
    StreamMetadata::new(
        Symbol::new(format!("stream/conformance-overflow-{}", row.layer)),
        StreamMedia::Data,
        StreamDirection::Source,
        ClockDomain::ServerFrame.symbol(),
        BufferPolicy::bounded_with_overflow(1, BufferOverflowPolicy::Error).unwrap(),
    )
}

fn clock_for(row: &MatrixRow, media: StreamMedia) -> ClockDomain {
    match (row.runner, media) {
        (MatrixRunner::Browser, StreamMedia::Pcm) => ClockDomain::BrowserFrame,
        (_, StreamMedia::Pcm) => ClockDomain::Sample,
        (_, StreamMedia::Midi) => ClockDomain::MidiTick,
        _ => ClockDomain::ServerFrame,
    }
}

fn host_request(media: StreamMedia) -> HostStreamConfigRequest {
    match media {
        StreamMedia::Pcm => HostStreamConfigRequest::new(
            fake_backend_symbol(),
            Symbol::new("fake/pcm"),
            StreamMedia::Pcm,
            HostDirection::Output,
            BufferPolicy::bounded(8).unwrap(),
        )
        .with_clock(ClockDomain::Sample.symbol()),
        StreamMedia::Midi => HostStreamConfigRequest::new(
            fake_backend_symbol(),
            Symbol::new("fake/midi"),
            StreamMedia::Midi,
            HostDirection::Input,
            BufferPolicy::bounded(8).unwrap(),
        )
        .with_clock(ClockDomain::MidiTick.symbol()),
        StreamMedia::Data => FakeBackend::data_request(8).unwrap(),
        StreamMedia::Diagnostic => unreachable!("fake host does not expose diagnostic media"),
    }
}

fn host_overflow_request() -> HostStreamConfigRequest {
    HostStreamConfigRequest::new(
        fake_backend_symbol(),
        Symbol::new("fake/data"),
        StreamMedia::Data,
        HostDirection::Input,
        BufferPolicy::bounded_with_overflow(1, BufferOverflowPolicy::Error).unwrap(),
    )
}
