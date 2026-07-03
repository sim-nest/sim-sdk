use sim_kernel::{Expr, Symbol};
use sim_lib_midi_core::{MetaEvent, MidiEvent, MidiPayload, TickTime, synthetic_origin};
use sim_lib_music_core::{Articulation, Channel, Melody, MelodyItem, Music, Note, Score, Time};
use sim_lib_pitch_core::Pitch;
use sim_lib_sound_shapes::encode_tuning_descriptor;
use sim_lib_sound_tuning::TuningDescriptor;

use super::support::{codec_symbols, cx, decode_once, encode_once};

/// A roundtrip corpus case: an expression plus an assertion over its decode.
type Roadmap11Case = (Expr, fn(&Expr));

fn roadmap11_corpus() -> Vec<Roadmap11Case> {
    vec![
        (
            Expr::Extension {
                tag: Symbol::qualified("pitch", "Pitch"),
                payload: Box::new(Expr::String("C4".to_owned())),
            },
            |expr| {
                let Expr::Extension { tag, payload } = expr else {
                    panic!("expected extension");
                };
                assert_eq!(tag, &Symbol::qualified("pitch", "Pitch"));
                let Expr::String(text) = payload.as_ref() else {
                    panic!("expected string payload");
                };
                sim_lib_pitch_shapes::decode_pitch(text).expect("pitch parse");
            },
        ),
        (
            Expr::Extension {
                tag: Symbol::qualified("midi", "MidiEvent"),
                payload: Box::new(Expr::String(sim_lib_midi_shapes::encode_midi_event(
                    &MidiEvent {
                        time: TickTime::new(480, 480).expect("tick"),
                        origin: synthetic_origin(),
                        payload: MidiPayload::Meta(MetaEvent::EndOfTrack),
                    },
                ))),
            },
            |expr| {
                let Expr::Extension { tag, payload } = expr else {
                    panic!("expected extension");
                };
                assert_eq!(tag, &Symbol::qualified("midi", "MidiEvent"));
                let Expr::String(text) = payload.as_ref() else {
                    panic!("expected string payload");
                };
                sim_lib_midi_shapes::decode_midi_event(text).expect("midi parse");
            },
        ),
        (
            Expr::Extension {
                tag: Symbol::qualified("music", "Score"),
                payload: Box::new(Expr::String(sim_lib_music_shapes::encode_music_file(
                    &Score::new(
                        120,
                        (4, 4),
                        Some("C major".to_owned()),
                        Music::Melody(
                            Melody::new(vec![MelodyItem::Note(
                                Note::new(
                                    Time::new(1, 4),
                                    Pitch::from_midi(60),
                                    100,
                                    Channel::new(0).expect("channel"),
                                    Articulation::Normal,
                                )
                                .expect("note"),
                            )])
                            .expect("melody"),
                        ),
                    )
                    .expect("score"),
                ))),
            },
            |expr| {
                let Expr::Extension { tag, payload } = expr else {
                    panic!("expected extension");
                };
                assert_eq!(tag, &Symbol::qualified("music", "Score"));
                let Expr::String(text) = payload.as_ref() else {
                    panic!("expected string payload");
                };
                sim_lib_music_shapes::decode_music_file(text).expect("music parse");
            },
        ),
        (
            Expr::Extension {
                tag: Symbol::qualified("sound", "TuningDescriptor"),
                payload: Box::new(Expr::String(encode_tuning_descriptor(
                    &TuningDescriptor::EqualTemperament {
                        divisions: 12,
                        reference_midi: 69,
                        reference_hz: 440.0,
                    },
                ))),
            },
            |expr| {
                let Expr::Extension { tag, payload } = expr else {
                    panic!("expected extension");
                };
                assert_eq!(tag, &Symbol::qualified("sound", "TuningDescriptor"));
                let Expr::String(text) = payload.as_ref() else {
                    panic!("expected string payload");
                };
                sim_lib_sound_shapes::decode_tuning_descriptor(text).expect("sound parse");
            },
        ),
    ]
}

#[test]
fn roadmap11_layer_values_roundtrip_through_installed_codecs() {
    let mut cx = cx();
    for codec in codec_symbols() {
        for (expr, validate) in roadmap11_corpus() {
            let encoded = encode_once(&mut cx, &codec, &expr);
            let decoded = decode_once(&mut cx, &codec, encoded);
            assert!(
                decoded.canonical_eq(&expr),
                "codec {} failed roadmap11 roundtrip {:?} -> {:?}",
                codec,
                expr,
                decoded
            );
            validate(&decoded);
        }
    }
}
