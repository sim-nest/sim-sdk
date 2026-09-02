use super::*;

pub(super) fn conformance_metadata(
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

pub(super) fn midi_item(ticks: i64) -> sim::lib_stream_core::StreamItem {
    sim::lib_stream_core::StreamItem::new(sim::lib_stream_core::StreamPacket::Midi(
        sim::lib_stream_core::MidiPacket::new(vec![
            sim::lib_stream_core::MidiPacketEvent::new(ticks, 480, vec![0x90, 60, 100]).unwrap(),
        ])
        .unwrap(),
    ))
}

pub(super) fn pcm_item(value: f32) -> sim::lib_stream_core::StreamItem {
    sim::lib_stream_core::StreamItem::new(sim::lib_stream_core::StreamPacket::Pcm(
        sim::lib_stream_core::PcmPacket::f32(1, 1, vec![value]).unwrap(),
    ))
}
