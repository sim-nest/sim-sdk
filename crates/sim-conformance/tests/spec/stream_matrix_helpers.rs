use sim::{
    kernel::{Expr, Symbol},
    lib_stream_core::{
        BufferOverflowPolicy, BufferPolicy, ClockDomain, StreamDirection, StreamItem, StreamMedia,
        StreamMetadata, StreamPacket,
    },
    lib_stream_host::{FakeBackend, HostDirection, HostStreamConfigRequest, fake_backend_symbol},
};

use super::{MatrixRow, MatrixRunner, conformance_metadata};

pub(super) fn data_item(payload: &str) -> StreamItem {
    StreamItem::new(StreamPacket::data(
        Symbol::qualified("stream/data", "expr"),
        Expr::String(payload.to_owned()),
    ))
}

pub(super) fn matrix_metadata(row: &MatrixRow, media: StreamMedia) -> StreamMetadata {
    conformance_metadata(
        &format!("stream/conformance-{}", row.layer),
        media,
        clock_for(row, media),
    )
}

pub(super) fn overflow_metadata(row: &MatrixRow) -> StreamMetadata {
    StreamMetadata::new(
        Symbol::new(format!("stream/conformance-overflow-{}", row.layer)),
        StreamMedia::Data,
        StreamDirection::Source,
        ClockDomain::ServerFrame.symbol(),
        BufferPolicy::bounded_with_overflow(1, BufferOverflowPolicy::Error).unwrap(),
    )
}

pub(super) fn host_request(media: StreamMedia) -> HostStreamConfigRequest {
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

pub(super) fn host_overflow_request() -> HostStreamConfigRequest {
    HostStreamConfigRequest::new(
        fake_backend_symbol(),
        Symbol::new("fake/data"),
        StreamMedia::Data,
        HostDirection::Input,
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
